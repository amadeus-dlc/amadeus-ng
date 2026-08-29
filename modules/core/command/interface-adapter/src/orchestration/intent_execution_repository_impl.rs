//! `IntentExecutionRepository` の実 Gateway (1 trait 1 Impl — gateway-taxonomy §5)。
//!
//! 集約 `IntentExecution` の再構成と永続化を、**本家 event-store-adapter-rs の
//! イベントストアを内包して**行う (ADR-010 Conformist — 腐敗防止層なし)。格納形式が
//! SQLite であることも、揮発の memory であることも、この実装の内部詳細である
//! (`Sqlite...` のような技術接頭辞を型名に出さないのはそのためである)。
//!
//! 型引数 `S` は本家の [`EventStore`] を満たすバックエンドで、[`IntentExecutionRepositoryImpl::open`]
//! が SQLite、[`IntentExecutionRepositoryImpl::in_memory`] が memory を選ぶ。
//! 手順はどちらも**同一**であり、そうしておくことで契約テストが両方に同じ約束を課せる (BR2.7)。
//!
//! # 封筒を組むのはここである (ADR-010 / B7)
//!
//! 本家 v3 で輸送のメタデータは [`EventEnvelope`] が運ぶ。ドメインイベントは純粋なドメイン
//! 内容だけを持つので、**封筒を組むのはアダプタ層のこの実装**である。材料はすべて適用後の
//! 集約が持っている: 集約識別子・そのイベントの通番 (`seq_nr()`)・発生時刻
//! (`last_updated_at()`)。型判別子は [`EVENT_MANIFEST`] を書く。
//!
//! # 楽観 version は不透明なトークンである (BR5.3)
//!
//! `version` を採番するのはストアであり、我々は解釈も比較もしない — `seq_nr` から導く
//! 前提検査は置かない (オーナー裁定 2026-08-27 (B))。**版はポートを往復する**:
//! `find_by_id` が読んだ版を返し、呼出側が `store` へ提示する。ここで書込直前に読み直しては
//! ならない — 常に最新版を提示することになり、楽観ロックが成立しなくなる。
//!
//! 本家 v3 の書込規約 (実測) はこの 2 つで、この実装はそれに合わせるだけである:
//!
//! - **新規作成** — `seq_nr == 1` かつ `expected_version == 0`。ストアが journal と snapshot を
//!   原子的に作り、version を 1 で採番する。対応が崩れた呼出しは `ContractViolation` になる。
//! - **更新** — `seq_nr > 1` かつ `expected_version` は読取済みの版。`WHERE version = expected`
//!   の CAS を通れば `version + 1` を記録する。
//!
//! どちらも `persist_event_and_snapshot` を使う。イベントのみの `persist_event` は
//! snapshot 行の `seq_nr` 列を進めないため、Quint モデル `journal_protocol` の不変条件
//! `snapshot_tracks_journal` (snapSeq == journalLen) を破る。
//!
//! [`EventEnvelope`]: event_store_adapter_rs::event_envelope::EventEnvelope

use std::io::ErrorKind;

use core_command_domain::orchestration::{
    ApplyError, EVENT_MANIFEST, Intent, IntentExecution, IntentExecutionEvent, IntentExecutionId,
};
use core_command_domain::workspace::StorePath;
use core_command_use_case::orchestration::{
    CorruptCause, IntentExecutionRepository, RehydratedIntentExecution, RepositoryError,
};
use event_store_adapter_rs::event_envelope::EventEnvelope;
use event_store_adapter_rs::types::{EventStore, EventStoreReadError, EventStoreWriteError};
use event_store_adapter_rs::{EventStoreForMemory, EventStoreForSqlite};

use super::store_failure::io_kind_of_source;

/// 集約の最初の `seq_nr` (`Started` は必ず 1 — BR1.1)。本家 v3 はこの値で新規作成と更新を
/// 分岐する (`is_created()` は廃止された)。
const FIRST_SEQ_NR: usize = 1;

/// SQLite ファイルを格納先にするイベントストア (本家)。
type SqliteStore = EventStoreForSqlite<IntentExecutionId, IntentExecution, IntentExecutionEvent>;

/// 揮発の格納先にするイベントストア (本家)。
type MemoryStore = EventStoreForMemory<IntentExecutionId, IntentExecution, IntentExecutionEvent>;

/// 本家のイベントストアを**単一所有**する `IntentExecutionRepository` の実装。
///
/// 内部可変性は持たない — 再構成 (Query) は `&self`、永続化 (Command) は `&mut self` で、
/// 本家 `EventStore` のレシーバとそのまま揃う
/// (`coding-rules/interior-mutability.md` / `coding-rules/command-query-separation.md`)。
#[derive(Debug)]
pub struct IntentExecutionRepositoryImpl<S> {
    store: S,
    /// 失敗の材料に添える場所 (揮発のストアには無いので `Option`)。
    location: Option<StorePath>,
}

/// `apply_event` の失敗を `Corrupt` の原因へ写す。
const fn apply_cause(error: &ApplyError) -> CorruptCause {
    match error {
        ApplyError::SequenceGap { .. } => CorruptCause::SequenceGap,
        // 通番枯渇 — 現在位置の続きとして適用できない点で SequenceGap と同類に写す。
        ApplyError::SequenceExhausted => CorruptCause::SequenceGap,
        // 再生に渡した intent が実行のものでない — ジャーナル先頭の `Started` から復元した
        // intent と後続イベントが食い違っている、すなわち行が壊れている。
        ApplyError::IntentMismatch
        | ApplyError::UnknownStage(_)
        | ApplyError::InvariantViolation(_) => CorruptCause::InvariantViolation,
    }
}

impl IntentExecutionRepositoryImpl<SqliteStore> {
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
    ) -> Result<IntentExecutionRepositoryImpl<SqliteStore>, RepositoryError> {
        let store = SqliteStore::new(path.as_path()).map_err(|error| RepositoryError::Io {
            kind: match &error {
                EventStoreWriteError::IOError(source) => io_kind_of_source(source.as_ref()),
                _ => ErrorKind::Other,
            },
            path: Some(path.as_path().to_path_buf()),
        })?;
        Ok(IntentExecutionRepositoryImpl {
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

impl IntentExecutionRepositoryImpl<MemoryStore> {
    /// 揮発のストアを持つ Repository を作る (テストとユースケース試験の足場)。
    ///
    /// テストダブルではなく**本家の memory バックエンド**であり、手順は SQLite と 1 行も
    /// 違わない。だからこそ契約テストが両方に同じ約束を課せる (BR2.7)。
    #[must_use]
    pub fn in_memory() -> IntentExecutionRepositoryImpl<MemoryStore> {
        IntentExecutionRepositoryImpl {
            store: MemoryStore::new(),
            location: None,
        }
    }
}

impl<S: Clone> IntentExecutionRepositoryImpl<S> {
    /// **同じストアを指す**別インスタンスを開き直す (別プロセスからの再オープン相当)。
    ///
    /// 本家のストアはどのバックエンドでも `Clone` が基底状態 (SQLite なら接続、memory なら
    /// 表) を共有する設計なので、写しではなく同じストアを指す別の口が得られる。
    #[must_use]
    pub fn reopened(&self) -> IntentExecutionRepositoryImpl<S> {
        IntentExecutionRepositoryImpl {
            store: self.store.clone(),
            location: self.location.clone(),
        }
    }
}

impl<S> IntentExecutionRepositoryImpl<S>
where
    S: EventStore<AID = IntentExecutionId, A = IntentExecution, P = IntentExecutionEvent>,
{
    /// 適用後の集約とドメインイベントから、本家のイベント封筒を組む。
    ///
    /// 材料はすべて集約が持っている — 識別子、`commit` 成功後の `seq_nr` (= そのイベントの
    /// 通番)、`last_updated_at` (= そのイベントの発生時刻)。**呼出側の不整合を検査する余地は
    /// 無い**: 旧実装が見ていた「イベントと集約が同じ集約を指すか」「通番が一致するか」は、
    /// イベントがその 2 つを持たなくなったことで**構成不能**になった (型による強制 — B6 で
    /// `seq_nr = 0` に対して行ったのと同じ形)。
    fn envelope(
        event: IntentExecutionEvent,
        aggregate: &IntentExecution,
    ) -> EventEnvelope<IntentExecutionId, IntentExecutionEvent> {
        EventEnvelope::new(
            aggregate.id().clone(),
            aggregate.seq_nr(),
            *aggregate.last_updated_at(),
            event,
        )
        .with_manifest(EVENT_MANIFEST)
    }

    /// 本家の読取失敗を Repository 面へ写す (BR1.5)。
    fn read_error(&self, error: &EventStoreReadError, id: &IntentExecutionId) -> RepositoryError {
        match error {
            EventStoreReadError::DeserializationError(_) => RepositoryError::Corrupt {
                aggregate_id: id.clone(),
                seq_nr: None,
                cause: CorruptCause::UndecodablePayload,
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

    /// 本家の書込失敗を Repository 面へ写す (BR1.5)。
    ///
    /// 楽観ロックの競合だけは材料 (`expected` / `actual`) を組み直す — 本家は整形済みの
    /// 文字列 1 本で返すので、文言を解析する代わりに**ストアに実在する version を読み直す**。
    /// 読めなければ「行が無い」= 0 として扱う (材料の欠落で失敗そのものを握り潰さない)。
    /// この読み直しは**失敗の材料を揃えるためだけ**であり、書込の判定には一切関与しない。
    ///
    /// `ContractViolation` は v3 で増えた腕で、ストレージ障害ではなく**呼出側の契約違反**
    /// (`seq_nr == 0`、新規作成と更新の対応崩れ) を表す。我々が封筒と `expected_version` を
    /// 組み違えたときにしか出ないので、破損した書込要求として `Corrupt` に写す。
    async fn write_error(
        &self,
        error: EventStoreWriteError,
        aggregate: &IntentExecution,
        expected_version: usize,
    ) -> RepositoryError {
        match error {
            EventStoreWriteError::OptimisticLockError(_) => RepositoryError::Conflict {
                expected: expected_version,
                actual: self.stored_version(aggregate.id()).await,
            },
            EventStoreWriteError::SerializationError(_)
            | EventStoreWriteError::ContractViolation(_) => RepositoryError::Corrupt {
                aggregate_id: aggregate.id().clone(),
                seq_nr: Some(aggregate.seq_nr()),
                cause: CorruptCause::UndecodablePayload,
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
    async fn stored_version(&self, id: &IntentExecutionId) -> usize {
        self.store
            .get_latest_snapshot_by_id(id)
            .await
            .ok()
            .flatten()
            .map_or(0, |snapshot| snapshot.version())
    }
}

impl<S> IntentExecutionRepositoryImpl<S>
where
    S: EventStore<AID = IntentExecutionId, A = IntentExecution, P = IntentExecutionEvent>,
{
    /// ジャーナル先頭の `Started` から、開始時点の intent を読み直す。
    ///
    /// 集約は `intent_id` しか持たない (`coding-rules/aggregate-references.md`) が、再生には
    /// 計画が要る。イベントは「その時点の事実の自己完結な記録」なので、intent の材料は
    /// `Started` に載っている — そこから復元すれば、この Repository は外部から intent を
    /// 渡されなくても再生できる。
    ///
    /// 先頭が `Started` でない、または 1 件も無いジャーナルは壊れている (BR1.2)。
    async fn genesis_intent(&self, id: &IntentExecutionId) -> Result<Intent, RepositoryError> {
        let journal = self
            .store
            .get_events_by_id_since_seq_nr(id, FIRST_SEQ_NR)
            .await
            .map_err(|error| self.read_error(&error, id))?;
        let corrupt = |seq_nr: Option<usize>| RepositoryError::Corrupt {
            aggregate_id: id.clone(),
            seq_nr,
            cause: CorruptCause::UndecodablePayload,
        };
        let genesis = journal.first().ok_or_else(|| corrupt(None))?;
        if genesis.manifest() != EVENT_MANIFEST {
            return Err(corrupt(Some(genesis.seq_nr())));
        }
        match genesis.payload() {
            IntentExecutionEvent::Started(started) => Ok(started.intent().clone()),
            _ => Err(corrupt(Some(genesis.seq_nr()))),
        }
    }
}

impl<S> IntentExecutionRepository for IntentExecutionRepositoryImpl<S>
where
    S: EventStore<AID = IntentExecutionId, A = IntentExecution, P = IntentExecutionEvent>,
{
    async fn find_by_id(
        &self,
        id: &IntentExecutionId,
    ) -> Result<RehydratedIntentExecution, RepositoryError> {
        let snapshot = self
            .store
            .get_latest_snapshot_by_id(id)
            .await
            .map_err(|error| self.read_error(&error, id))?;
        let Some(snapshot) = snapshot else {
            // ジャーナル行が 1 件も無ければ「まだ無い」、あるなら「壊れている」(BR1.2)。
            let journal = self
                .store
                .get_events_by_id_since_seq_nr(id, FIRST_SEQ_NR)
                .await
                .map_err(|error| self.read_error(&error, id))?;
            return Err(if journal.is_empty() {
                RepositoryError::NotFound {
                    execution_id: id.clone(),
                }
            } else {
                RepositoryError::Corrupt {
                    aggregate_id: id.clone(),
                    seq_nr: None,
                    cause: CorruptCause::MissingSnapshot,
                }
            });
        };
        // 楽観 version はスナップショット行の列が正本 — 封筒から取り出して呼出側へ渡す (BR5.3)。
        let version = snapshot.version();
        let mut aggregate = snapshot.into_aggregate();
        // 再生には計画が要る。集約は intent を ID でしか参照しない
        // (coding-rules/aggregate-references.md) ので、**ジャーナル先頭の `Started` から
        // その時点の intent を読み直す**。`Started` は genesis 専用なので必ず 1 件目にある。
        let intent = self.genesis_intent(id).await?;
        // リプレイの開始位置は**集約自身の通番**から採る。スナップショット行の `seq_nr` 列は
        // 同じ値のストア側の写しであり、正本はドメインが持つ通番だからである (裁定 3)。
        // 列と写しが食い違えば `apply_event` が `SequenceGap` で止める。
        // 本家の差分読取は「その `seq_nr` を**含む**」ので、写しの次から読む。飽和加算なのは
        // 通番が usize::MAX に達した集約の防御 — その場合は MAX を含んで再読取することになり、
        // `apply_event` が `SequenceExhausted` で止める (黙って wrap / panic しない — NFR4.3)。
        let envelopes = self
            .store
            .get_events_by_id_since_seq_nr(id, aggregate.seq_nr().saturating_add(1))
            .await
            .map_err(|error| self.read_error(&error, id))?;
        for envelope in &envelopes {
            // 再生前に manifest を照合する。本家は manifest を検証せず復号だけして返すため、
            // ここで拒まないと foreign manifest の行（別の型名・別の読み方の版を名乗る行）が
            // そのまま状態遷移に流れ込む。読取側 (`JournalReaderImpl::decode_entry`) と同じ
            // 拒否条件・同じ分類 (`UndecodablePayload`) で対称にする (PR #31 CodeRabbit 指摘)。
            if envelope.manifest() != EVENT_MANIFEST {
                return Err(RepositoryError::Corrupt {
                    aggregate_id: id.clone(),
                    seq_nr: Some(envelope.seq_nr()),
                    cause: CorruptCause::UndecodablePayload,
                });
            }
            aggregate
                .apply_event(
                    &intent,
                    envelope.seq_nr(),
                    *envelope.occurred_at(),
                    envelope.payload(),
                )
                .map_err(|error| RepositoryError::Corrupt {
                    aggregate_id: id.clone(),
                    seq_nr: Some(envelope.seq_nr()),
                    cause: apply_cause(&error),
                })?;
        }
        Ok(RehydratedIntentExecution::new(aggregate, version))
    }

    async fn store(
        &mut self,
        event: &IntentExecutionEvent,
        aggregate: &IntentExecution,
        expected_version: usize,
    ) -> Result<(), RepositoryError> {
        // 新規作成と更新の分岐は本家 v3 と同じ導出 — 封筒の `seq_nr == 1` が新規作成である。
        // 新規作成の `expected_version` は規約上 0 で、そうでない組み合わせは本家が
        // `ContractViolation` で拒否する (我々は握り潰さず `Corrupt` に写す)。
        let envelope = IntentExecutionRepositoryImpl::<S>::envelope(event.clone(), aggregate);
        let outcome = self
            .store
            .persist_event_and_snapshot(envelope, aggregate.clone(), expected_version)
            .await;
        match outcome {
            Ok(()) => Ok(()),
            Err(error) => Err(self.write_error(error, aggregate, expected_version).await),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_command_domain::orchestration::{StageDisplay, WorkspaceScan};
    use core_command_domain::workflow_definition::{BrownfieldGreenfield, StageNumber};
    fn display(number: &str) -> StageDisplay {
        StageDisplay::new(StageNumber::parse(number).unwrap(), "Stage", "orchestrator").unwrap()
    }

    fn scan() -> WorkspaceScan {
        WorkspaceScan::new(
            BrownfieldGreenfield::Greenfield,
            "Unknown",
            "Unknown",
            "Unknown",
        )
        .unwrap()
    }

    use chrono::{DateTime, Utc};
    use core_command_domain::orchestration::{IntentId, StageCompleted, StageEntry, StartRequest};
    use core_command_domain::workflow_definition::{
        DefinitionRevision, PhaseId, PlanAction, StageSlug, WorkflowDefinitionId,
    };

    const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";
    const OTHER_INTENT: &str = "018f3b2c-4d5e-7f60-8abc-def012345678";

    /// 未永続の集約が提示する版 (新規作成の `expected_version`)。
    const UNPERSISTED: usize =
        <IntentExecutionRepositoryImpl<MemoryStore> as IntentExecutionRepository>::UNPERSISTED_VERSION;

    fn at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-08-23T00:00:00Z")
            .expect("固定の ISO 8601 UTC")
            .with_timezone(&Utc)
    }

    fn intent() -> IntentExecutionId {
        IntentExecutionId::parse(INTENT).expect("UUIDv7")
    }

    fn other_intent() -> IntentExecutionId {
        IntentExecutionId::parse(OTHER_INTENT).expect("UUIDv7")
    }

    fn intent_plan() -> Intent {
        Intent::new(
            IntentId::parse(INTENT).expect("UUIDv7"),
            WorkflowDefinitionId::parse("claude").expect("定義 id"),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision"),
            StartRequest::new("classic", "unit"),
            vec![StageEntry::new(
                StageSlug::parse("state-init").expect("slug"),
                PhaseId::Initialization,
                PlanAction::Execute,
                false,
                display("0.1"),
            )],
            scan(),
        )
        .expect("合成計画は Intent の不変条件を満たす")
    }

    fn genesis() -> (IntentExecution, IntentExecutionEvent) {
        IntentExecution::start(intent(), intent_plan(), at())
    }

    fn repository() -> IntentExecutionRepositoryImpl<MemoryStore> {
        IntentExecutionRepositoryImpl::in_memory()
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
    fn a_volatile_store_has_no_place_to_name_in_its_failures() {
        assert_eq!(repository().store_path(), None);
    }

    #[test]
    fn the_envelope_takes_every_transport_material_from_the_applied_aggregate() {
        // B7: 封筒を組むのはここである。集約識別子・通番・発生時刻は commit を通った集約が
        // 持っており、イベントは持たない — 「別集約のイベントを混ぜる」「通番を食い違わせる」
        // という旧前提検査の対象は**構成不能**になった (型による強制)。
        let (aggregate, event) = genesis();
        let envelope =
            IntentExecutionRepositoryImpl::<MemoryStore>::envelope(event.clone(), &aggregate);
        assert_eq!(envelope.aggregate_id(), &intent());
        assert_eq!(envelope.seq_nr(), aggregate.seq_nr());
        assert_eq!(envelope.occurred_at(), aggregate.last_updated_at());
        assert_eq!(envelope.manifest(), EVENT_MANIFEST);
        assert_eq!(envelope.payload(), &event);
    }

    #[tokio::test]
    async fn a_second_aggregate_gets_its_own_envelope_identity() {
        // 封筒の識別子は引数ではなく集約から来るので、別集約は別の行になる。
        let (aggregate, event) = genesis();
        let other = IntentExecution::start(other_intent(), intent_plan(), at()).0;
        let first =
            IntentExecutionRepositoryImpl::<MemoryStore>::envelope(event.clone(), &aggregate);
        let second = IntentExecutionRepositoryImpl::<MemoryStore>::envelope(event, &other);
        assert_ne!(first.aggregate_id(), second.aggregate_id());
    }

    #[tokio::test]
    async fn a_journal_whose_first_row_is_not_a_genesis_is_corrupt() {
        // 再生には計画が要り、その出所はジャーナル先頭の `Started` だけである。先頭が別の
        // 変種を名乗る行は読み方が定まらないので、推測せず `Corrupt` で止める (BR1.2)。
        let (aggregate, _) = genesis();
        let impostor = IntentExecutionEvent::StageCompleted(StageCompleted::new(
            StageSlug::parse("state-init").expect("slug"),
            None,
        ));
        let mut repository = repository();
        repository
            .store(&impostor, &aggregate, UNPERSISTED)
            .await
            .expect("行としては書ける");
        assert_eq!(
            repository.find_by_id(&intent()).await.unwrap_err(),
            RepositoryError::Corrupt {
                aggregate_id: intent(),
                seq_nr: Some(1),
                cause: CorruptCause::UndecodablePayload,
            }
        );
    }

    #[test]
    fn a_read_failure_is_mapped_by_its_kind() {
        let repository = repository();
        assert_eq!(
            repository.read_error(
                &EventStoreReadError::DeserializationError(Box::new(std::io::Error::other("x"))),
                &intent()
            ),
            RepositoryError::Corrupt {
                aggregate_id: intent(),
                seq_nr: None,
                cause: CorruptCause::UndecodablePayload,
            }
        );
        assert_eq!(
            repository.read_error(&EventStoreReadError::IOError(boxed_busy()), &intent()),
            RepositoryError::Io {
                kind: ErrorKind::WouldBlock,
                path: None,
            }
        );
        assert_eq!(
            repository.read_error(
                &EventStoreReadError::OtherError("分類できない".to_string()),
                &intent()
            ),
            RepositoryError::Io {
                kind: ErrorKind::Other,
                path: None,
            }
        );
    }

    #[tokio::test]
    async fn a_write_failure_is_mapped_by_its_kind() {
        let repository = repository();
        let (aggregate, _) = genesis();

        assert_eq!(
            repository
                .write_error(
                    EventStoreWriteError::OptimisticLockError("optimistic lock failed".to_string()),
                    &aggregate,
                    0,
                )
                .await,
            RepositoryError::Conflict {
                expected: 0,
                actual: 0,
            },
            "提示した版はそのまま、実在する version は読み直して材料にする (行が無ければ 0)"
        );
        assert_eq!(
            repository
                .write_error(
                    EventStoreWriteError::SerializationError(Box::new(std::io::Error::other("x"))),
                    &aggregate,
                    0,
                )
                .await,
            RepositoryError::Corrupt {
                aggregate_id: intent(),
                seq_nr: Some(1),
                cause: CorruptCause::UndecodablePayload,
            }
        );
        assert_eq!(
            repository
                .write_error(
                    EventStoreWriteError::ContractViolation("BR2.6".to_string()),
                    &aggregate,
                    0,
                )
                .await,
            RepositoryError::Corrupt {
                aggregate_id: intent(),
                seq_nr: Some(1),
                cause: CorruptCause::UndecodablePayload,
            },
            "契約違反は我々が封筒を組み違えたときにしか出ない (v3 で増えた腕)"
        );
        assert_eq!(
            repository
                .write_error(EventStoreWriteError::IOError(boxed_busy()), &aggregate, 0)
                .await,
            RepositoryError::Io {
                kind: ErrorKind::WouldBlock,
                path: None,
            }
        );
        assert_eq!(
            repository
                .write_error(
                    EventStoreWriteError::OtherError("分類できない".to_string()),
                    &aggregate,
                    0,
                )
                .await,
            RepositoryError::Io {
                kind: ErrorKind::Other,
                path: None,
            }
        );
    }

    #[tokio::test]
    async fn the_conflict_material_is_the_version_that_is_actually_stored() {
        let mut repository = repository();
        let (aggregate, event) = genesis();
        repository
            .store(&event, &aggregate, UNPERSISTED)
            .await
            .expect("genesis");
        assert_eq!(repository.stored_version(&intent()).await, 1);
        assert_eq!(
            repository.stored_version(&other_intent()).await,
            0,
            "行が無ければ 0"
        );
    }

    #[tokio::test]
    async fn the_volatile_store_is_shared_by_the_reopened_handle() {
        let mut repository = repository();
        let (aggregate, event) = genesis();
        repository
            .store(&event, &aggregate, UNPERSISTED)
            .await
            .expect("genesis");

        let reopened = repository.reopened();
        assert_eq!(
            reopened
                .find_by_id(&intent())
                .await
                .expect("同じストアを指す")
                .version(),
            1
        );
    }
}
