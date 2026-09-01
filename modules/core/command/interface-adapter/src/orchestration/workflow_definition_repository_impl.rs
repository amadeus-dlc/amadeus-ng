//! `WorkflowDefinitionRepository` の実 Gateway (1 trait 1 Impl — gateway-taxonomy §5)。
//!
//! 集約 `WorkflowDefinition` の再構成と永続化を、**本家 event-store-adapter-rs のイベント
//! ストアを内包して**行う (ADR-010 Conformist — 腐敗防止層なし)。
//!
//! # 2026-08-31 の全面転換 (オーナー裁定)
//!
//! 「`workflow_definition_repository_impl.rs` この実装を破棄せよ。NG 中の NG です。
//! リポジトリの実装は `EventStoreForSqlite` を使わないといけない」。
//!
//! 旧実装は dist の 3 入力 (`stage-graph.json` / `scope-grid.json` / `scopes/*.md`) を
//! ディスクから読んで集約を組み立てていた。それは `coding-rules/cqrs-boundaries.md` 規則 4
//! (コマンド側の最新状態は常に集約から。**リードモデルは遅延しているので物理的に読めない**)
//! への正面違反である。パースの中身は取込境界 [`DefinitionArtifactsClientImpl`] へ移り、
//! ここは他の 2 つの Repository と 1 行も違わない ES の手順だけになった。
//!
//! # 3 集約が 1 つのストアに同居する
//!
//! 定義のストリームは実行・intent のストリームと**同じストアファイル**に置く。集約種別は
//! 鍵 ([`WorkflowDefinitionAggregateKeyDto`] の `type_name = "WorkflowDefinition"`) が分ける。
//! 前提は集約識別子の値の一意性で、定義 id はハーネス名 (`claude` 等) なので UUID 空間と
//! 衝突しない。
//!
//! # 楽観 version はポートを往復する (BR5.3)
//!
//! `find_by_id` が読んだ版を集約に刻み、`store` がそれを提示する。**書込直前に読み直しては
//! ならない** — 常に最新版を提示することになり、楽観ロックが成立しなくなる。定義は
//! 改訂 (`Redefined`) を持つので、この往復は実際に効く。
//!
//! [`DefinitionArtifactsClientImpl`]: super::definition_artifacts_client_impl::DefinitionArtifactsClientImpl

use std::io::ErrorKind;

use core_command_domain::workflow_definition::{
    WorkflowDefinition, WorkflowDefinitionEvent, WorkflowDefinitionId,
};
use core_command_domain::workspace::StorePath;
use core_command_use_case::orchestration::{RepositoryError, WorkflowDefinitionRepository};
use event_store_adapter_rs::event_envelope::EventEnvelope;
use event_store_adapter_rs::types::{EventStore, EventStoreReadError, EventStoreWriteError};
use event_store_adapter_rs::{EventStoreForMemory, EventStoreForSqlite};

use super::dto::{
    DtoDecodeError, WorkflowDefinitionAggregateKeyDto, WorkflowDefinitionDto,
    WorkflowDefinitionEventDto,
};
use super::snapshot_strategy::SnapshotStrategy;
use super::store_failure::io_kind_of_source;

/// ジャーナル行 `manifest` 列に書く型判別子 — **書く側の正本**。
///
/// 実行 (`intent-execution-event/1`) / intent (`intent-event/1`) とは別の型・別の読み方な
/// ので判別子も別である。版を上げるのは payload の読み方が変わるときだけである。
const EVENT_MANIFEST: &str = "workflow-definition-event/1";

/// 集約の最初の `seq_nr` (`Defined` は必ず 1)。本家 v3 はこの値で新規作成と更新を分岐する。
const FIRST_SEQ_NR: usize = 1;

/// SQLite ファイルを格納先にするイベントストア (本家)。
///
/// 型引数はいずれも**この層の永続化 DTO** である — ドメイン型はストアに触れない
/// (`coding-rules/domain-persistence-neutrality.md`)。
pub type WorkflowDefinitionSqliteStore = EventStoreForSqlite<
    WorkflowDefinitionAggregateKeyDto,
    WorkflowDefinitionDto,
    WorkflowDefinitionEventDto,
>;

/// 揮発の格納先にするイベントストア (本家)。
pub type WorkflowDefinitionMemoryStore = EventStoreForMemory<
    WorkflowDefinitionAggregateKeyDto,
    WorkflowDefinitionDto,
    WorkflowDefinitionEventDto,
>;

/// 本家のイベントストアを**単一所有**する `WorkflowDefinitionRepository` の実装。
///
/// 内部可変性は持たない — 再構成 (Query) は `&self`、永続化 (Command) は `&mut self` で、
/// 本家 `EventStore` のレシーバとそのまま揃う
/// (`coding-rules/interior-mutability.md` / `coding-rules/command-query-separation.md`)。
#[derive(Debug)]
pub struct WorkflowDefinitionRepositoryImpl<S> {
    store: S,
    /// 失敗の材料に添える場所 (揮発のストアには無いので `Option`)。
    location: Option<StorePath>,
    /// いつスナップショットを書き直すか (実装の内部設定 — ポート面に現れない)。
    strategy: SnapshotStrategy,
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
    Undecodable(DtoDecodeError),
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

impl WorkflowDefinitionRepositoryImpl<WorkflowDefinitionSqliteStore> {
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
    ) -> Result<
        WorkflowDefinitionRepositoryImpl<WorkflowDefinitionSqliteStore>,
        RepositoryError<WorkflowDefinitionId>,
    > {
        let store = WorkflowDefinitionSqliteStore::new(path.as_path()).map_err(|error| {
            RepositoryError::Io {
                kind: match &error {
                    EventStoreWriteError::IOError(source) => io_kind_of_source(source.as_ref()),
                    _ => ErrorKind::Other,
                },
                path: Some(path.as_path().to_path_buf()),
            }
        })?;
        Ok(WorkflowDefinitionRepositoryImpl {
            store,
            location: Some(path.clone()),
            strategy: SnapshotStrategy::default(),
        })
    }

    /// 内包しているストアの場所 (開き直しの材料)。
    #[must_use]
    pub const fn path(&self) -> Option<&StorePath> {
        self.location.as_ref()
    }
}

impl WorkflowDefinitionRepositoryImpl<WorkflowDefinitionMemoryStore> {
    /// 揮発のストアを持つ Repository を作る (テストとユースケース試験の足場)。
    ///
    /// テストダブルではなく**本家の memory バックエンド**であり、手順は SQLite と 1 行も
    /// 違わない。だからこそ契約テストが両方に同じ約束を課せる (BR2.7)。
    #[must_use]
    pub fn in_memory() -> WorkflowDefinitionRepositoryImpl<WorkflowDefinitionMemoryStore> {
        WorkflowDefinitionRepositoryImpl {
            store: WorkflowDefinitionMemoryStore::new(),
            location: None,
            strategy: SnapshotStrategy::default(),
        }
    }
}

impl<S> WorkflowDefinitionRepositoryImpl<S> {
    /// スナップショットの書き直し間隔を差し替える (既定は 10 イベントごと)。
    #[must_use]
    pub const fn with_snapshot_strategy(
        mut self,
        strategy: SnapshotStrategy,
    ) -> WorkflowDefinitionRepositoryImpl<S> {
        self.strategy = strategy;
        self
    }
}

impl<S: Clone> WorkflowDefinitionRepositoryImpl<S> {
    /// **同じストアを指す**別インスタンスを開き直す (別プロセスからの再オープン相当)。
    ///
    /// 本家のストアはどのバックエンドでも `Clone` が基底状態 (SQLite なら接続、memory なら
    /// 表) を共有する設計なので、写しではなく同じストアを指す別の口が得られる。
    #[must_use]
    pub fn reopened(&self) -> WorkflowDefinitionRepositoryImpl<S> {
        WorkflowDefinitionRepositoryImpl {
            store: self.store.clone(),
            location: self.location.clone(),
            strategy: self.strategy,
        }
    }
}

impl<S> WorkflowDefinitionRepositoryImpl<S>
where
    S: EventStore<
            AID = WorkflowDefinitionAggregateKeyDto,
            A = WorkflowDefinitionDto,
            P = WorkflowDefinitionEventDto,
        >,
{
    /// 適用後の集約とドメインイベントから、本家のイベント封筒を組む。
    ///
    /// 材料はすべて集約が持っている — 識別子、`define` / `redefine` 成功後の `seq_nr`
    /// (= そのイベントの通番)、`last_updated_at` (= そのイベントの発生時刻)。
    /// `IntentExecutionRepositoryImpl::envelope` と同型であり、**呼出側が時刻を運ぶ口は
    /// 無い** (オーナー裁定 2026-08-31 — 手本と対にする)。
    fn envelope(
        event: &WorkflowDefinitionEvent,
        definition: &WorkflowDefinition,
    ) -> EventEnvelope<WorkflowDefinitionAggregateKeyDto, WorkflowDefinitionEventDto> {
        EventEnvelope::new(
            WorkflowDefinitionAggregateKeyDto::of(definition.id()),
            definition.seq_nr(),
            *definition.last_updated_at(),
            WorkflowDefinitionEventDto::of(event),
        )
        .with_manifest(EVENT_MANIFEST)
    }

    /// 本家の読取失敗を Repository 面へ写す。
    fn read_error(
        &self,
        error: &EventStoreReadError,
        id: &WorkflowDefinitionId,
    ) -> RepositoryError<WorkflowDefinitionId> {
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
    /// 楽観ロックの競合だけは材料 (`expected` / `actual`) を組み直す — 本家は整形済みの
    /// 文字列 1 本で返すので、文言を解析する代わりに**ストアに実在する version を読み直す**。
    /// この読み直しは**失敗の材料を揃えるためだけ**であり、書込の判定には一切関与しない。
    async fn write_error(
        &self,
        error: EventStoreWriteError,
        definition: &WorkflowDefinition,
        expected_version: usize,
    ) -> RepositoryError<WorkflowDefinitionId> {
        match error {
            EventStoreWriteError::OptimisticLockError(_) => RepositoryError::Conflict {
                expected: expected_version,
                actual: self.stored_version(definition.id()).await,
            },
            EventStoreWriteError::SerializationError(_)
            | EventStoreWriteError::ContractViolation(_) => RepositoryError::Corrupt {
                id: definition.id().clone(),
                seq_nr: Some(definition.seq_nr()),
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
    async fn stored_version(&self, id: &WorkflowDefinitionId) -> usize {
        self.store
            .get_latest_snapshot_by_id(&WorkflowDefinitionAggregateKeyDto::of(id))
            .await
            .ok()
            .flatten()
            .map_or(0, |snapshot| snapshot.version())
    }
}

impl<S> WorkflowDefinitionRepository for WorkflowDefinitionRepositoryImpl<S>
where
    S: EventStore<
            AID = WorkflowDefinitionAggregateKeyDto,
            A = WorkflowDefinitionDto,
            P = WorkflowDefinitionEventDto,
        >,
{
    async fn find_by_id(
        &self,
        id: &WorkflowDefinitionId,
    ) -> Result<WorkflowDefinition, RepositoryError<WorkflowDefinitionId>> {
        // 本家 example と同型 — スナップショット行 (ある時点の集約) を基底に、その通番より
        // 後のイベントだけを差分再生する (オーナー裁定 2026-08-30)。
        let key = WorkflowDefinitionAggregateKeyDto::of(id);
        let snapshot = self
            .store
            .get_latest_snapshot_by_id(&key)
            .await
            .map_err(|error| self.read_error(&error, id))?;
        let Some(snapshot) = snapshot else {
            // ジャーナル行が 1 件も無ければ「まだ確立されていない」、あるなら「壊れている」—
            // genesis は journal と snapshot を原子的に書くので、片方だけは矛盾である。
            let journal = self
                .store
                .get_events_by_id_since_seq_nr(&key, FIRST_SEQ_NR)
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
        let version = snapshot.version();
        // 基底の通番は封筒の列から読む — 定義のスナップショット行は通番を内容に持たない
        // (`IntentDto` と同じ形)。
        let base_seq = snapshot.seq_nr();
        // 基底の復元は検査付き再構成経路を必ず通る。
        let base = snapshot
            .aggregate()
            .to_domain()
            .map_err(|error| RepositoryError::Corrupt {
                id: id.clone(),
                seq_nr: None,
                source: Box::new(CorruptDetail::Undecodable(error)),
            })?
            .with_seq_nr(base_seq);
        // 差分 — 基底の通番より後のイベントだけを読む。復号は封筒ごとに行い、manifest を
        // 照合する。本家は manifest を検証せず復号だけして返すため、ここで拒まないと foreign
        // manifest の行 (別の型名・別の読み方の版を名乗る行) がそのまま状態遷移に流れ込む。
        let delta = self
            .store
            .get_events_by_id_since_seq_nr(&key, base_seq + 1)
            .await
            .map_err(|error| self.read_error(&error, id))?;
        // 通番の連続性は再生前にここで検査する — `apply_event` は通番の飛びをクラッシュで
        // 止めるが、行の欠けは**読取時に分類できる破損**であり、他の破損と同じく `Corrupt`
        // に写す。
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
            events.push((envelope.seq_nr(), *envelope.occurred_at(), event));
        }
        // 差分再生 — 壊れた歴史はドメインがクラッシュで止める (オーナー裁定 2026-08-30)。
        // 版はストアが読んだ値を刻む。
        Ok(WorkflowDefinition::replay(base, events).with_version(version))
    }

    async fn store(
        &mut self,
        event: &WorkflowDefinitionEvent,
        definition: &WorkflowDefinition,
    ) -> Result<(), RepositoryError<WorkflowDefinitionId>> {
        // 提示する版は集約が運んできたものである (書込直前に読み直さない — BR5.3)。
        let expected_version = definition.version();
        let envelope = WorkflowDefinitionRepositoryImpl::<S>::envelope(event, definition);
        // スナップショットは**初回は必ず**書く (基底が無いとリプレイできない)。以後は設定
        // されたストラテジに従って書き直す。イベントのみの書込でも楽観 version は進む。
        let outcome = if definition.seq_nr() == FIRST_SEQ_NR
            || self.strategy.wants_snapshot(definition.seq_nr())
        {
            self.store
                .persist_event_and_snapshot(
                    envelope,
                    WorkflowDefinitionDto::of(definition),
                    expected_version,
                )
                .await
        } else {
            self.store.persist_event(envelope, expected_version).await
        };
        match outcome {
            Ok(()) => Ok(()),
            Err(error) => Err(self.write_error(error, definition, expected_version).await),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use core_command_domain::workflow_definition::{
        ExecutionKind, PhaseId, ScopeGrid, ScopeMetadata, StageGraph, StageMode, StageNodeBuilder,
        StageNumber, StageSlug,
    };

    fn id() -> WorkflowDefinitionId {
        WorkflowDefinitionId::parse("claude").expect("定義 id")
    }

    fn other_id() -> WorkflowDefinitionId {
        WorkflowDefinitionId::parse("kiro").expect("定義 id")
    }

    fn revision(fill: char) -> core_command_domain::workflow_definition::DefinitionRevision {
        core_command_domain::workflow_definition::DefinitionRevision::parse(&format!(
            "sha256:{}",
            fill.to_string().repeat(64)
        ))
        .expect("revision")
    }

    fn content(stage_count: usize) -> (StageGraph, ScopeGrid, BTreeMap<String, ScopeMetadata>) {
        let nodes = (0..stage_count)
            .map(|index| {
                StageNodeBuilder::new(
                    StageSlug::parse(&format!("stage-{index}")).expect("slug"),
                    StageNumber::parse(&format!("0.{}", index + 1)).expect("番号"),
                    "Stage".to_string(),
                    PhaseId::Initialization,
                    ExecutionKind::Always,
                    StageMode::Inline,
                )
                .scopes(vec!["classic".to_string()])
                .build()
            })
            .collect();
        let graph = StageGraph::new(nodes).expect("グラフ");
        let grid = ScopeGrid::from_graph(&graph);
        let scopes = [(
            "classic".to_string(),
            ScopeMetadata::new("classic").expect("スコープ"),
        )]
        .into_iter()
        .collect();
        (graph, grid, scopes)
    }

    fn genesis(stage_count: usize) -> (WorkflowDefinition, WorkflowDefinitionEvent) {
        let (graph, grid, scopes) = content(stage_count);
        WorkflowDefinition::define(id(), revision('0'), graph, grid, scopes, at())
    }

    fn at() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-08-31T00:00:00Z")
            .expect("固定の ISO 8601 UTC")
            .with_timezone(&chrono::Utc)
    }

    fn workflow_definition_repository()
    -> WorkflowDefinitionRepositoryImpl<WorkflowDefinitionMemoryStore> {
        WorkflowDefinitionRepositoryImpl::in_memory()
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
        let workflow_definition_repository = workflow_definition_repository();
        let corrupt = workflow_definition_repository.read_error(
            &EventStoreReadError::DeserializationError(Box::new(std::io::Error::other("x"))),
            &id(),
        );
        assert!(matches!(
            &corrupt,
            RepositoryError::Corrupt { id: got, seq_nr: None, .. } if *got == id()
        ));
        assert_eq!(
            std::error::Error::source(&corrupt)
                .expect("原因が連鎖する")
                .to_string(),
            "store deserialization failed"
        );
        assert!(matches!(
            workflow_definition_repository
                .read_error(&EventStoreReadError::IOError(boxed_busy()), &id()),
            RepositoryError::Io {
                kind: ErrorKind::WouldBlock,
                path: None,
            }
        ));
        assert!(matches!(
            workflow_definition_repository.read_error(
                &EventStoreReadError::OtherError("分類できない".to_string()),
                &id()
            ),
            RepositoryError::Io {
                kind: ErrorKind::Other,
                path: None,
            }
        ));
    }

    #[tokio::test]
    async fn a_write_failure_is_mapped_by_its_kind() {
        let workflow_definition_repository = workflow_definition_repository();
        let (definition, _) = genesis(2);

        assert!(
            matches!(
                workflow_definition_repository
                    .write_error(
                        EventStoreWriteError::OptimisticLockError(
                            "optimistic lock failed".to_string()
                        ),
                        &definition,
                        0,
                    )
                    .await,
                RepositoryError::Conflict {
                    expected: 0,
                    actual: 0,
                }
            ),
            "提示した版はそのまま、実在する version は読み直して材料にする (行が無ければ 0)"
        );
        let corrupt = workflow_definition_repository
            .write_error(
                EventStoreWriteError::SerializationError(Box::new(std::io::Error::other("x"))),
                &definition,
                0,
            )
            .await;
        assert!(matches!(
            &corrupt,
            RepositoryError::Corrupt { id: got, seq_nr: Some(1), .. } if *got == id()
        ));
        assert_eq!(
            std::error::Error::source(&corrupt)
                .expect("原因が連鎖する")
                .to_string(),
            "write contract violation"
        );
        assert!(matches!(
            workflow_definition_repository
                .write_error(
                    EventStoreWriteError::ContractViolation("BR2.6".to_string()),
                    &definition,
                    0,
                )
                .await,
            RepositoryError::Corrupt {
                seq_nr: Some(1),
                ..
            }
        ));
        assert!(matches!(
            workflow_definition_repository
                .write_error(EventStoreWriteError::IOError(boxed_busy()), &definition, 0)
                .await,
            RepositoryError::Io {
                kind: ErrorKind::WouldBlock,
                path: None,
            }
        ));
        assert!(matches!(
            workflow_definition_repository
                .write_error(
                    EventStoreWriteError::OtherError("分類できない".to_string()),
                    &definition,
                    0,
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
        let mut workflow_definition_repository = workflow_definition_repository();
        let (definition, event) = genesis(2);
        workflow_definition_repository
            .store(&event, &definition)
            .await
            .expect("genesis");
        assert_eq!(
            workflow_definition_repository.stored_version(&id()).await,
            1
        );
        assert_eq!(
            workflow_definition_repository
                .stored_version(&other_id())
                .await,
            0,
            "行が無ければ 0"
        );
    }

    #[tokio::test]
    async fn the_volatile_store_is_shared_by_the_reopened_handle() {
        let mut workflow_definition_repository = workflow_definition_repository();
        let (definition, event) = genesis(2);
        workflow_definition_repository
            .store(&event, &definition)
            .await
            .expect("genesis");

        let reopened = workflow_definition_repository.reopened();
        assert_eq!(
            reopened
                .find_by_id(&id())
                .await
                .expect("同じストアを指す")
                .revision(),
            &revision('0')
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
                CorruptDetail::Undecodable(DtoDecodeError::InvariantViolation),
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
    fn the_undecodable_detail_chains_its_dto_decode_error() {
        let detail = CorruptDetail::Undecodable(DtoDecodeError::malformed("id", "x"));
        let source = std::error::Error::source(&detail).expect("復号失敗は原因を連鎖する");
        assert_eq!(source.to_string(), "malformed field id: x");
        assert!(std::error::Error::source(&CorruptDetail::MissingSnapshot).is_none());
    }

    #[test]
    fn a_volatile_store_has_no_place_to_name_in_its_failures() {
        let workflow_definition_repository = workflow_definition_repository();
        assert_eq!(workflow_definition_repository.store_path(), None);
        assert!(workflow_definition_repository.reopened().location.is_none());
    }
}
