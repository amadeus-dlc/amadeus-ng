//! `IntentRepository` の実 Gateway (1 trait 1 Impl — gateway-taxonomy §5)。
//!
//! 集約 `Intent` の再構成と永続化を、**本家 event-store-adapter-rs のイベントストアを
//! 内包して**行う (ADR-010 Conformist — 腐敗防止層なし)。手順は
//! [`IntentExecutionRepositoryImpl`] と同型で、違いは集約が静的であることに由来する 2 点
//! だけである (issue #50):
//!
//! - **イベントは `Created` 1 種** — つまり書込は必ず初回 (`seq_nr == 1`) であり、本家の
//!   作成規約どおり journal と snapshot を原子的に書く (`persist_event_and_snapshot`)。
//!   スナップショットの書き直し間隔 ([`SnapshotStrategy`]) を差すのは変異イベントが増えて
//!   更新経路 (`persist_event`) が生まれるときで、その分岐点は `store` 内の網羅 match が
//!   ビルドエラーで教える。
//! - **発生時刻は引数で受ける** — intent は時刻を状態に持たない (時刻は実行の語彙) ので、
//!   ジャーナル封筒のメタデータに刻む時刻は呼出側の clock から来る。
//!
//! # 実行のストアと同居する (issue #50 の設計裁定)
//!
//! intent のストリームは実行のストリームと**同じストアファイル**に置く — ワークスペースの
//! ストアは 1 つであり、集約種別は鍵 ([`IntentAggregateKey`] の `type_name = "Intent"`) が
//! 分ける。前提は集約識別子の値の一意性である ([`IntentAggregateKey`] の doc を参照 —
//! UUID である限り満たされる)。
//!
//! # 楽観 version はポート面に現れない
//!
//! `Intent` は不変集約で更新コマンドが無く、版を引き回す往復そのものが存在しない。genesis の
//! `expected_version` は本家の作成規約のリテラル 0 で、重複作成は現行スロット行の一意性が
//! `Conflict` として拒む。
//!
//! [`IntentExecutionRepositoryImpl`]: super::intent_execution_repository_impl::IntentExecutionRepositoryImpl
//! [`SnapshotStrategy`]: super::snapshot_strategy::SnapshotStrategy

use std::io::ErrorKind;

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{Intent, IntentEvent, IntentId};
use core_command_domain::workspace::StorePath;
use core_command_use_case::orchestration::{IntentRepository, RepositoryError};
use event_store_adapter_rs::event_envelope::EventEnvelope;
use event_store_adapter_rs::types::{EventStore, EventStoreReadError, EventStoreWriteError};
use event_store_adapter_rs::{EventStoreForMemory, EventStoreForSqlite};

use super::store_failure::io_kind_of_source;
use super::wire::{IntentAggregateKey, WireDecodeError, WireIntent, WireIntentEvent};

/// ジャーナル行 `manifest` 列に書く型判別子 — **書く側の正本**。
///
/// 実行のジャーナル (`intent-execution-event/1`) とは別の型・別の読み方なので判別子も別で
/// ある。版を上げるのは payload の読み方が変わるときだけである。
const EVENT_MANIFEST: &str = "intent-event/1";

/// 集約の最初の `seq_nr` (`Created` は必ず 1)。本家 v3 はこの値で新規作成と更新を分岐する。
const FIRST_SEQ_NR: usize = 1;

/// 新規作成時に提示する `expected_version` (本家 v3 の作成規約のリテラル)。
const GENESIS_EXPECTED_VERSION: usize = 0;

/// SQLite ファイルを格納先にするイベントストア (本家)。
///
/// 型引数はいずれも**この層の永続化 DTO** である — ドメイン型はストアに触れない
/// (`coding-rules/domain-persistence-neutrality.md`)。
pub type IntentSqliteStore = EventStoreForSqlite<IntentAggregateKey, WireIntent, WireIntentEvent>;

/// 揮発の格納先にするイベントストア (本家)。
pub type IntentMemoryStore = EventStoreForMemory<IntentAggregateKey, WireIntent, WireIntentEvent>;

/// 本家のイベントストアを**単一所有**する `IntentRepository` の実装。
///
/// 内部可変性は持たない — 再構成 (Query) は `&self`、永続化 (Command) は `&mut self` で、
/// 本家 `EventStore` のレシーバとそのまま揃う
/// (`coding-rules/interior-mutability.md` / `coding-rules/command-query-separation.md`)。
#[derive(Debug)]
pub struct IntentRepositoryImpl<S> {
    store: S,
    /// 失敗の材料に添える場所 (揮発のストアには無いので `Option`)。
    location: Option<StorePath>,
}

/// `Corrupt` の原因分類 — **この実装の私有物** (裁定 6: エラー分類はポート契約に載せない。
/// 契約は「壊れていた」としか約束せず、診断表示は `Error::source` の連鎖で残る)。
#[derive(Debug)]
enum CorruptDetail {
    /// ジャーナル行はあるのにスナップショット行が無い (genesis は原子的に両方書く)。
    MissingSnapshot,
    /// ジャーナル行が別の型判別子を名乗っている (foreign manifest)。
    ForeignManifest,
    /// 差分行の通番が連続していない (行の欠け — 再生すると誤った状態になる)。
    SequenceGap,
    /// 行のペイロードをドメイン型へ復号できない。
    Undecodable(WireDecodeError),
    /// ストアの復号そのものが失敗した (本家の `DeserializationError`)。
    StoreDeserialization,
    /// 呼出側の書込契約違反 (本家の `ContractViolation` / `SerializationError`)。
    WriteContract,
}

impl std::fmt::Display for CorruptDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CorruptDetail::MissingSnapshot => f.write_str("missing snapshot"),
            CorruptDetail::ForeignManifest => f.write_str("foreign manifest"),
            CorruptDetail::SequenceGap => f.write_str("sequence gap"),
            CorruptDetail::Undecodable(_) => f.write_str("undecodable payload"),
            CorruptDetail::StoreDeserialization => f.write_str("store deserialization failed"),
            CorruptDetail::WriteContract => f.write_str("write contract violation"),
        }
    }
}

impl std::error::Error for CorruptDetail {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CorruptDetail::Undecodable(inner) => Some(inner),
            _ => None,
        }
    }
}

impl IntentRepositoryImpl<IntentSqliteStore> {
    /// SQLite ファイルのストアを開く (無ければ作る)。
    ///
    /// 親ディレクトリ (`intents/`) は upstream の既存ディレクトリなので**作らない** —
    /// 無ければ `Io { kind: NotFound }` で止まる。表と索引は本家が冪等に作る。
    ///
    /// # Errors
    ///
    /// 親ディレクトリ欠落・権限・ディスク (`Io`) を返す。
    pub fn open(
        path: &StorePath,
    ) -> Result<IntentRepositoryImpl<IntentSqliteStore>, RepositoryError<IntentId>> {
        let store =
            IntentSqliteStore::new(path.as_path()).map_err(|error| RepositoryError::Io {
                kind: match &error {
                    EventStoreWriteError::IOError(source) => io_kind_of_source(source.as_ref()),
                    _ => ErrorKind::Other,
                },
                path: Some(path.as_path().to_path_buf()),
            })?;
        Ok(IntentRepositoryImpl {
            store,
            location: Some(path.clone()),
        })
    }

    /// 内包しているストアの場所 (開き直しの材料)。
    #[must_use]
    pub const fn path(&self) -> Option<&StorePath> {
        self.location.as_ref()
    }
}

impl IntentRepositoryImpl<IntentMemoryStore> {
    /// 揮発のストアを持つ Repository を作る (テストとユースケース試験の足場)。
    ///
    /// テストダブルではなく**本家の memory バックエンド**であり、手順は SQLite と 1 行も
    /// 違わない。だからこそ契約テストが両方に同じ約束を課せる (BR2.7)。
    #[must_use]
    pub fn in_memory() -> IntentRepositoryImpl<IntentMemoryStore> {
        IntentRepositoryImpl {
            store: IntentMemoryStore::new(),
            location: None,
        }
    }
}

impl<S: Clone> IntentRepositoryImpl<S> {
    /// **同じストアを指す**別インスタンスを開き直す (別プロセスからの再オープン相当)。
    ///
    /// 本家のストアはどのバックエンドでも `Clone` が基底状態 (SQLite なら接続、memory なら
    /// 表) を共有する設計なので、写しではなく同じストアを指す別の口が得られる。
    #[must_use]
    pub fn reopened(&self) -> IntentRepositoryImpl<S> {
        IntentRepositoryImpl {
            store: self.store.clone(),
            location: self.location.clone(),
        }
    }
}

impl<S> IntentRepositoryImpl<S>
where
    S: EventStore<AID = IntentAggregateKey, A = WireIntent, P = WireIntentEvent>,
{
    /// 本家の読取失敗を Repository 面へ写す。
    fn read_error(&self, error: &EventStoreReadError, id: &IntentId) -> RepositoryError<IntentId> {
        match error {
            EventStoreReadError::DeserializationError(_) => RepositoryError::Corrupt {
                id: id.clone(),
                seq_nr: None,
                source: Box::new(CorruptDetail::StoreDeserialization),
            },
            EventStoreReadError::IOError(source) => RepositoryError::Io {
                kind: io_kind_of_source(source.as_ref()),
                path: self.store_path(),
            },
            EventStoreReadError::OtherError(_) => RepositoryError::Io {
                kind: ErrorKind::Other,
                path: self.store_path(),
            },
        }
    }

    /// 本家の書込失敗を Repository 面へ写す。
    ///
    /// 重複作成 (現行スロット行の一意性違反) は本家が `OptimisticLockError` で返すので
    /// `Conflict` に写す — 材料の `actual` はストアに実在する版を読み直して添える
    /// (この読み直しは**失敗の材料を揃えるためだけ**であり、書込の判定には関与しない)。
    /// `ContractViolation` / `SerializationError` は我々が封筒を組み違えたときにしか出ない
    /// ので、破損した書込要求として `Corrupt` に写す。
    async fn write_error(
        &self,
        error: EventStoreWriteError,
        intent: &Intent,
    ) -> RepositoryError<IntentId> {
        match error {
            EventStoreWriteError::OptimisticLockError(_) => RepositoryError::Conflict {
                expected: GENESIS_EXPECTED_VERSION,
                actual: self.stored_version(intent.id()).await,
            },
            EventStoreWriteError::SerializationError(_)
            | EventStoreWriteError::ContractViolation(_) => RepositoryError::Corrupt {
                id: intent.id().clone(),
                seq_nr: Some(FIRST_SEQ_NR),
                source: Box::new(CorruptDetail::WriteContract),
            },
            EventStoreWriteError::IOError(source) => RepositoryError::Io {
                kind: io_kind_of_source(source.as_ref()),
                path: self.store_path(),
            },
            EventStoreWriteError::OtherError(_) => RepositoryError::Io {
                kind: ErrorKind::Other,
                path: self.store_path(),
            },
        }
    }

    /// 失敗の材料に添える場所 (揮発のストアには無い)。
    fn store_path(&self) -> Option<std::path::PathBuf> {
        self.location
            .as_ref()
            .map(|path| path.as_path().to_path_buf())
    }

    /// ストアに実在する楽観 version (行が無い・読めないときは 0)。競合の材料にだけ使う。
    async fn stored_version(&self, id: &IntentId) -> usize {
        self.store
            .get_latest_snapshot_by_id(&IntentAggregateKey::of(id))
            .await
            .ok()
            .flatten()
            .map_or(0, |snapshot| snapshot.version())
    }
}

impl<S> IntentRepository for IntentRepositoryImpl<S>
where
    S: EventStore<AID = IntentAggregateKey, A = WireIntent, P = WireIntentEvent>,
{
    async fn find_by_id(&self, id: &IntentId) -> Result<Intent, RepositoryError<IntentId>> {
        // 本家 example (`user_account_repository.rs`) と同型 — スナップショット行 (ある時点の
        // 集約) を基底に、その通番より後のイベントだけを差分再生する (オーナー裁定 2026-08-30)。
        let snapshot = self
            .store
            .get_latest_snapshot_by_id(&IntentAggregateKey::of(id))
            .await
            .map_err(|error| self.read_error(&error, id))?;
        let Some(snapshot) = snapshot else {
            // ジャーナル行が 1 件も無ければ「まだ無い」、あるなら「壊れている」— genesis は
            // journal と snapshot を原子的に書くので、片方だけは矛盾である。
            let journal = self
                .store
                .get_events_by_id_since_seq_nr(&IntentAggregateKey::of(id), FIRST_SEQ_NR)
                .await
                .map_err(|error| self.read_error(&error, id))?;
            return Err(if journal.is_empty() {
                RepositoryError::NotFound { id: id.clone() }
            } else {
                RepositoryError::Corrupt {
                    id: id.clone(),
                    seq_nr: None,
                    source: Box::new(CorruptDetail::MissingSnapshot),
                }
            });
        };
        // 基底の通番は封筒の列から読む — intent は通番を状態に持たない (静的集約)。
        let base_seq = snapshot.seq_nr();
        // 基底の復元は検査付き再構成経路を必ず通る。
        let base = snapshot
            .aggregate()
            .to_domain()
            .map_err(|error| RepositoryError::Corrupt {
                id: id.clone(),
                seq_nr: None,
                source: Box::new(CorruptDetail::Undecodable(error)),
            })?;
        // 差分 — 基底の通番より後のイベントだけを読む。復号は封筒ごとに行い、manifest を
        // 照合する。本家は manifest を検証せず復号だけして返すため、ここで拒まないと foreign
        // manifest の行（別の型名・別の読み方の版を名乗る行）がそのまま状態遷移に流れ込む。
        // 現状イベントは `Created` 1 種 = 差分は常に空だが、変種が増えたときの適用経路を
        // ここに閉じる (`IntentExecutionRepositoryImpl` と対称)。
        let delta = self
            .store
            .get_events_by_id_since_seq_nr(&IntentAggregateKey::of(id), base_seq + 1)
            .await
            .map_err(|error| self.read_error(&error, id))?;
        // 通番の連続性は再生前にここで検査する — 行の欠けは**読取時に分類できる破損**であり、
        // 他の破損 (MissingSnapshot / ForeignManifest / Undecodable) と同じく `Corrupt` に写す。
        let mut expected_seq = base_seq;
        let mut events = Vec::with_capacity(delta.len());
        for envelope in &delta {
            expected_seq = expected_seq
                .checked_add(1)
                .ok_or_else(|| RepositoryError::Corrupt {
                    id: id.clone(),
                    seq_nr: Some(envelope.seq_nr()),
                    source: Box::new(CorruptDetail::SequenceGap),
                })?;
            if envelope.seq_nr() != expected_seq {
                return Err(RepositoryError::Corrupt {
                    id: id.clone(),
                    seq_nr: Some(envelope.seq_nr()),
                    source: Box::new(CorruptDetail::SequenceGap),
                });
            }
            if envelope.manifest() != EVENT_MANIFEST {
                return Err(RepositoryError::Corrupt {
                    id: id.clone(),
                    seq_nr: Some(envelope.seq_nr()),
                    source: Box::new(CorruptDetail::ForeignManifest),
                });
            }
            let event =
                envelope
                    .payload()
                    .to_domain()
                    .map_err(|error| RepositoryError::Corrupt {
                        id: id.clone(),
                        seq_nr: Some(envelope.seq_nr()),
                        source: Box::new(CorruptDetail::Undecodable(error)),
                    })?;
            events.push(event);
        }
        // 差分再生 — 壊れた歴史はドメインがクラッシュで止める (オーナー裁定 2026-08-30)。
        Ok(Intent::replay(events, base))
    }

    async fn store(
        &mut self,
        event: &IntentEvent,
        intent: &Intent,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError<IntentId>> {
        // イベントの通番はイベント自身から導く — `Created` は必ず 1 (genesis)。変異イベント
        // が増えたらこの網羅 match がビルドで落ち、通番の導出とスナップショットストラテジの
        // 分岐 (`IntentExecutionRepositoryImpl::store` の形) をここへ足すことになる。
        let seq_nr = match event {
            IntentEvent::Created(_) => FIRST_SEQ_NR,
        };
        // genesis は本家の作成規約どおり journal と snapshot を原子的に書く — 基底が無いと
        // リプレイできない (初回 `persist_event_and_snapshot` — オーナー裁定 2026-08-30)。
        let envelope = EventEnvelope::new(
            IntentAggregateKey::of(intent.id()),
            seq_nr,
            occurred_at,
            WireIntentEvent::of(event),
        )
        .with_manifest(EVENT_MANIFEST);
        match self
            .store
            .persist_event_and_snapshot(envelope, WireIntent::of(intent), GENESIS_EXPECTED_VERSION)
            .await
        {
            Ok(()) => Ok(()),
            Err(error) => Err(self.write_error(error, intent).await),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use core_command_domain::orchestration::{
        Created, StageDisplay, StageEntry, StartRequest, WorkspaceScan,
    };
    use core_command_domain::workflow_definition::{
        BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
        WorkflowDefinitionId,
    };

    const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";
    const OTHER_INTENT: &str = "018f3b2c-4d5e-7f60-8abc-def012345678";

    fn at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-08-23T00:00:00Z")
            .expect("固定の ISO 8601 UTC")
            .with_timezone(&Utc)
    }

    fn intent_id() -> IntentId {
        IntentId::parse(INTENT).expect("UUIDv7")
    }

    fn other_intent_id() -> IntentId {
        IntentId::parse(OTHER_INTENT).expect("UUIDv7")
    }

    fn created() -> Created {
        Created::new(
            intent_id(),
            WorkflowDefinitionId::parse("claude").expect("定義 id"),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision"),
            StartRequest::new("classic", "unit"),
            vec![StageEntry::new(
                StageSlug::parse("state-init").expect("slug"),
                PhaseId::Initialization,
                PlanAction::Execute,
                false,
                StageDisplay::new(
                    StageNumber::parse("0.1").expect("番号"),
                    "Stage",
                    "orchestrator",
                )
                .expect("単一行"),
            )],
            WorkspaceScan::new(
                BrownfieldGreenfield::Greenfield,
                "Unknown",
                "Unknown",
                "Unknown",
            )
            .expect("単一行"),
        )
    }

    /// genesis の (集約, 誕生イベント) の対 (`Intent::create` が返す形と同じ)。
    fn genesis() -> (Intent, IntentEvent) {
        (Intent::from(created()), IntentEvent::Created(created()))
    }

    fn repository() -> IntentRepositoryImpl<IntentMemoryStore> {
        IntentRepositoryImpl::in_memory()
    }

    /// 本家が `Box<dyn Error>` に包んで運ぶ SQLite の失敗。
    fn boxed_busy() -> Box<dyn std::error::Error + Send + Sync> {
        Box::new(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::DatabaseBusy,
                extended_code: 5,
            },
            None,
        ))
    }

    #[test]
    fn a_read_failure_is_mapped_by_its_kind() {
        let repository = repository();
        let corrupt = repository.read_error(
            &EventStoreReadError::DeserializationError(Box::new(std::io::Error::other("x"))),
            &intent_id(),
        );
        assert!(matches!(
            &corrupt,
            RepositoryError::Corrupt { id, seq_nr: None, .. } if *id == intent_id()
        ));
        assert_eq!(
            std::error::Error::source(&corrupt)
                .expect("原因が連鎖する")
                .to_string(),
            "store deserialization failed"
        );
        assert!(matches!(
            repository.read_error(&EventStoreReadError::IOError(boxed_busy()), &intent_id()),
            RepositoryError::Io {
                kind: ErrorKind::WouldBlock,
                path: None,
            }
        ));
        assert!(matches!(
            repository.read_error(
                &EventStoreReadError::OtherError("分類できない".to_string()),
                &intent_id()
            ),
            RepositoryError::Io {
                kind: ErrorKind::Other,
                path: None,
            }
        ));
    }

    #[tokio::test]
    async fn a_write_failure_is_mapped_by_its_kind() {
        let repository = repository();
        let (aggregate, _) = genesis();

        assert!(
            matches!(
                repository
                    .write_error(
                        EventStoreWriteError::OptimisticLockError(
                            "optimistic lock failed".to_string()
                        ),
                        &aggregate,
                    )
                    .await,
                RepositoryError::Conflict {
                    expected: 0,
                    actual: 0,
                }
            ),
            "genesis の expected はリテラル 0、実在する version は読み直して材料にする (行が無ければ 0)"
        );
        let corrupt = repository
            .write_error(
                EventStoreWriteError::SerializationError(Box::new(std::io::Error::other("x"))),
                &aggregate,
            )
            .await;
        assert!(matches!(
            &corrupt,
            RepositoryError::Corrupt { id, seq_nr: Some(1), .. } if *id == intent_id()
        ));
        assert_eq!(
            std::error::Error::source(&corrupt)
                .expect("原因が連鎖する")
                .to_string(),
            "write contract violation"
        );
        assert!(
            matches!(
                repository
                    .write_error(
                        EventStoreWriteError::ContractViolation("BR2.2".to_string()),
                        &aggregate,
                    )
                    .await,
                RepositoryError::Corrupt {
                    seq_nr: Some(1),
                    ..
                }
            ),
            "契約違反は我々が封筒を組み違えたときにしか出ない (v3 で増えた腕)"
        );
        assert!(matches!(
            repository
                .write_error(EventStoreWriteError::IOError(boxed_busy()), &aggregate)
                .await,
            RepositoryError::Io {
                kind: ErrorKind::WouldBlock,
                path: None,
            }
        ));
        assert!(matches!(
            repository
                .write_error(
                    EventStoreWriteError::OtherError("分類できない".to_string()),
                    &aggregate,
                )
                .await,
            RepositoryError::Io {
                kind: ErrorKind::Other,
                path: None,
            }
        ));
    }

    #[tokio::test]
    async fn the_conflict_material_is_the_version_that_is_actually_stored() {
        let mut repository = repository();
        let (aggregate, event) = genesis();
        repository
            .store(&event, &aggregate, at())
            .await
            .expect("genesis");
        assert_eq!(repository.stored_version(&intent_id()).await, 1);
        assert_eq!(
            repository.stored_version(&other_intent_id()).await,
            0,
            "行が無ければ 0"
        );
    }

    #[tokio::test]
    async fn the_volatile_store_is_shared_by_the_reopened_handle() {
        let mut repository = repository();
        let (aggregate, event) = genesis();
        repository
            .store(&event, &aggregate, at())
            .await
            .expect("genesis");

        let reopened = repository.reopened();
        assert_eq!(
            reopened
                .find_by_id(&intent_id())
                .await
                .expect("同じストアを指す"),
            aggregate
        );
    }

    #[test]
    fn every_corrupt_detail_renders_its_material() {
        // 分類はポート契約に載らない (裁定 6) — 診断表示 (caused by: ...) がここの文字列である。
        for (detail, wording) in [
            (CorruptDetail::MissingSnapshot, "missing snapshot"),
            (CorruptDetail::ForeignManifest, "foreign manifest"),
            (CorruptDetail::SequenceGap, "sequence gap"),
            (
                CorruptDetail::Undecodable(WireDecodeError::InvariantViolation),
                "undecodable payload",
            ),
            (
                CorruptDetail::StoreDeserialization,
                "store deserialization failed",
            ),
            (CorruptDetail::WriteContract, "write contract violation"),
        ] {
            assert_eq!(detail.to_string(), wording);
        }
    }

    #[test]
    fn the_undecodable_detail_chains_its_wire_error() {
        let detail = CorruptDetail::Undecodable(WireDecodeError::malformed("id", "x"));
        let source = std::error::Error::source(&detail).expect("復号失敗は原因を連鎖する");
        assert_eq!(source.to_string(), "malformed field id: x");
        assert!(std::error::Error::source(&CorruptDetail::MissingSnapshot).is_none());
    }

    #[test]
    fn a_volatile_store_has_no_place_to_name_in_its_failures() {
        let repository = IntentRepositoryImpl::in_memory();
        assert_eq!(repository.store_path(), None);
        assert!(repository.reopened().location.is_none());
    }
}
