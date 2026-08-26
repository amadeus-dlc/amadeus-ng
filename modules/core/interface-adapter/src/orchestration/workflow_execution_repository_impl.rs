//! `WorkflowExecutionRepository` の実 Gateway (1 trait 1 Impl — gateway-taxonomy §5)。
//!
//! 集約 `WorkflowExecution` の再構成と永続化を、**本家 event-store-adapter-rs の
//! イベントストアを内包して**行う (ADR-010 Conformist — 腐敗防止層なし)。格納形式が
//! SQLite であることも、揮発の memory であることも、この実装の内部詳細である
//! (`Sqlite...` のような技術接頭辞を型名に出さないのはそのためである)。
//!
//! 型引数 `S` は本家の [`EventStore`] を満たすバックエンドで、[`WorkflowExecutionRepositoryImpl::open`]
//! が SQLite、[`WorkflowExecutionRepositoryImpl::in_memory`] が memory を選ぶ。
//! 手順はどちらも**同一**であり、そうしておくことで契約テストが両方に同じ約束を課せる (BR2.7)。
//!
//! # 楽観 version は不透明なトークンである (BR5.3)
//!
//! `version` を採番するのはストアであり、我々は解釈も比較もしない — `seq_nr` から導く
//! 前提検査は置かない (オーナー裁定 2026-08-27 (B))。実測した本家の作法は次の 2 つで、
//! この実装はそれに合わせるだけである:
//!
//! - **genesis** (`Event::is_created()` が真) — 本家は CAS をせず、渡された集約の
//!   `version()` を**そのまま初期値として記録する**。我々の集約は「まだ永続化していない」を
//!   `version = 0` で表すので、そのまま渡すと初期値が 0 になり、以後の列が 1 つずれる。
//!   そこで**ストアへ渡す写しにだけ**最初の採番値 [`FIRST_STORED_VERSION`] を載せる
//!   (呼出側の集約は 1 ビットも動かない)。本家サンプルが genesis 集約を `version = 1` で
//!   作っているのと同じ結果になる。
//! - **更新** — 本家は `WHERE version = aggregate.version()` の CAS を張り、通れば
//!   `version + 1` を記録する。渡すのは呼出側の集約そのままでよい。

use std::io::ErrorKind;

use core_domain::orchestration::{ApplyError, IntentId, WorkflowExecution, WorkflowExecutionEvent};
use core_use_case::orchestration::{CorruptCause, RepositoryError, WorkflowExecutionRepository};
use event_store_adapter_rs::types::{
    Aggregate, Event, EventStore, EventStoreReadError, EventStoreWriteError,
};
use event_store_adapter_rs::{EventStoreForMemory, EventStoreForSqlite};

use super::store_failure::io_kind_of_source;
use super::store_path::StorePath;

/// genesis を書いたときにストアへ記録される最初の楽観 version。
///
/// 以後は本家が `+1` していく。値そのものに意味は無い (不透明なトークン — BR5.3) が、
/// 0 から始めると「行が無い」状態と区別がつかなくなるので 1 から始める。
const FIRST_STORED_VERSION: usize = 1;

/// 集約の最初の `seq_nr` (`Started` は必ず 1 — BR1.1)。
const FIRST_SEQ_NR: usize = 1;

/// SQLite ファイルを格納先にするイベントストア (本家)。
type SqliteStore = EventStoreForSqlite<IntentId, WorkflowExecution, WorkflowExecutionEvent>;

/// 揮発の格納先にするイベントストア (本家)。
type MemoryStore = EventStoreForMemory<IntentId, WorkflowExecution, WorkflowExecutionEvent>;

/// 本家のイベントストアを**単一所有**する `WorkflowExecutionRepository` の実装。
///
/// 内部可変性は持たない — 再構成 (Query) は `&self`、永続化 (Command) は `&mut self` で、
/// 本家 `EventStore` のレシーバとそのまま揃う
/// (`coding-rules/interior-mutability.md` / `coding-rules/command-query-separation.md`)。
#[derive(Debug)]
pub struct WorkflowExecutionRepositoryImpl<S> {
    store: S,
    /// 失敗の材料に添える場所 (揮発のストアには無いので `Option`)。
    location: Option<StorePath>,
}

/// `apply_event` の失敗を `Corrupt` の原因へ写す。
const fn apply_cause(error: &ApplyError) -> CorruptCause {
    match error {
        ApplyError::SequenceGap { .. } => CorruptCause::SequenceGap,
        ApplyError::UnknownStage(_) | ApplyError::InvariantViolation(_) => {
            CorruptCause::InvariantViolation
        }
    }
}

impl WorkflowExecutionRepositoryImpl<SqliteStore> {
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
    ) -> Result<WorkflowExecutionRepositoryImpl<SqliteStore>, RepositoryError> {
        let store = SqliteStore::new(path.as_path()).map_err(|error| RepositoryError::Io {
            kind: match &error {
                EventStoreWriteError::IOError(source) => io_kind_of_source(source.as_ref()),
                _ => ErrorKind::Other,
            },
            path: Some(path.as_path().to_path_buf()),
        })?;
        Ok(WorkflowExecutionRepositoryImpl {
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

impl WorkflowExecutionRepositoryImpl<MemoryStore> {
    /// 揮発のストアを持つ Repository を作る (テストとユースケース試験の足場)。
    ///
    /// テストダブルではなく**本家の memory バックエンド**であり、手順は SQLite と 1 行も
    /// 違わない。だからこそ契約テストが両方に同じ約束を課せる (BR2.7)。
    #[must_use]
    pub fn in_memory() -> WorkflowExecutionRepositoryImpl<MemoryStore> {
        WorkflowExecutionRepositoryImpl {
            store: MemoryStore::new(),
            location: None,
        }
    }
}

impl<S: Clone> WorkflowExecutionRepositoryImpl<S> {
    /// **同じストアを指す**別インスタンスを開き直す (別プロセスからの再オープン相当)。
    ///
    /// 本家のストアはどのバックエンドでも `Clone` が基底状態 (SQLite なら接続、memory なら
    /// 表) を共有する設計なので、写しではなく同じストアを指す別の口が得られる。
    #[must_use]
    pub fn reopened(&self) -> WorkflowExecutionRepositoryImpl<S> {
        WorkflowExecutionRepositoryImpl {
            store: self.store.clone(),
            location: self.location.clone(),
        }
    }
}

impl<S> WorkflowExecutionRepositoryImpl<S>
where
    S: EventStore<AID = IntentId, AG = WorkflowExecution, EV = WorkflowExecutionEvent>,
{
    /// 呼出側の不整合 (BR1.3 の前提検査)。破ったら `Corrupt(SequenceGap)`。
    ///
    /// 検査するのは**ドメインの関心**だけである — イベントと集約が同じ集約を指し、
    /// イベントの `seq_nr` が「適用後の集約の `seq_nr`」と一致すること (1 コマンド 1 イベント)。
    /// 楽観 version はストアの関心なので、ここでは見ない (BR5.3)。
    fn check_preconditions(
        event: &WorkflowExecutionEvent,
        aggregate: &WorkflowExecution,
    ) -> Result<(), RepositoryError> {
        if event.aggregate_id() != aggregate.id()
            || event.seq_nr() != aggregate.seq_nr()
            || event.seq_nr() < FIRST_SEQ_NR
        {
            return Err(RepositoryError::Corrupt {
                aggregate_id: event.aggregate_id().clone(),
                seq_nr: Some(event.seq_nr()),
                cause: CorruptCause::SequenceGap,
            });
        }
        Ok(())
    }

    /// 本家の読取失敗を Repository 面へ写す (BR1.5)。
    fn read_error(&self, error: &EventStoreReadError, id: &IntentId) -> RepositoryError {
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
    async fn write_error(
        &self,
        error: EventStoreWriteError,
        aggregate: &WorkflowExecution,
    ) -> RepositoryError {
        match error {
            EventStoreWriteError::OptimisticLockError(_) => RepositoryError::Conflict {
                expected: aggregate.version(),
                actual: self.stored_version(aggregate.id()).await,
            },
            EventStoreWriteError::SerializationError(_) => RepositoryError::Corrupt {
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
    async fn stored_version(&self, id: &IntentId) -> usize {
        self.store
            .get_latest_snapshot_by_id(id)
            .await
            .ok()
            .flatten()
            .map_or(0, |aggregate| aggregate.version())
    }
}

impl<S> WorkflowExecutionRepository for WorkflowExecutionRepositoryImpl<S>
where
    S: EventStore<AID = IntentId, AG = WorkflowExecution, EV = WorkflowExecutionEvent>,
{
    async fn find_by_id(&self, id: &IntentId) -> Result<WorkflowExecution, RepositoryError> {
        let snapshot = self
            .store
            .get_latest_snapshot_by_id(id)
            .await
            .map_err(|error| self.read_error(&error, id))?;
        let Some(mut aggregate) = snapshot else {
            // ジャーナル行が 1 件も無ければ「まだ無い」、あるなら「壊れている」(BR1.2)。
            let journal = self
                .store
                .get_events_by_id_since_seq_nr(id, FIRST_SEQ_NR)
                .await
                .map_err(|error| self.read_error(&error, id))?;
            return Err(if journal.is_empty() {
                RepositoryError::NotFound {
                    intent_id: id.clone(),
                }
            } else {
                RepositoryError::Corrupt {
                    aggregate_id: id.clone(),
                    seq_nr: None,
                    cause: CorruptCause::MissingSnapshot,
                }
            });
        };
        // 本家の差分読取は「その `seq_nr` を**含む**」ので、写しの次から読む。
        let events = self
            .store
            .get_events_by_id_since_seq_nr(id, aggregate.seq_nr() + 1)
            .await
            .map_err(|error| self.read_error(&error, id))?;
        for event in &events {
            aggregate
                .apply_event(event)
                .map_err(|error| RepositoryError::Corrupt {
                    aggregate_id: id.clone(),
                    seq_nr: Some(event.seq_nr()),
                    cause: apply_cause(&error),
                })?;
        }
        // 楽観 version はストアが載せた値のまま — replay では動かさない (BR5.3)。
        Ok(aggregate)
    }

    async fn store(
        &mut self,
        event: &WorkflowExecutionEvent,
        aggregate: &WorkflowExecution,
    ) -> Result<(), RepositoryError> {
        WorkflowExecutionRepositoryImpl::<S>::check_preconditions(event, aggregate)?;
        let outcome = if event.is_created() {
            // genesis のときだけ、ストアへ渡す写しに最初の採番値を載せる (冒頭の doc を参照)。
            let mut first = aggregate.clone();
            first.set_version(FIRST_STORED_VERSION);
            self.store.persist_event_and_snapshot(event, &first).await
        } else {
            self.store
                .persist_event_and_snapshot(event, aggregate)
                .await
        };
        match outcome {
            Ok(()) => Ok(()),
            Err(error) => Err(self.write_error(error, aggregate).await),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use core_domain::orchestration::{StageEntry, StartRequest, WorkflowExecutionEventPayload};
    use core_domain::workflow_definition::{
        DefinitionRevision, PhaseId, PlanAction, StageSlug, WorkflowDefinitionId,
    };

    const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";
    const OTHER_INTENT: &str = "018f3b2c-4d5e-7f60-8abc-def012345678";

    fn at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-08-23T00:00:00Z")
            .expect("固定の ISO 8601 UTC")
            .with_timezone(&Utc)
    }

    fn intent() -> IntentId {
        IntentId::parse(INTENT).expect("UUIDv7")
    }

    fn other_intent() -> IntentId {
        IntentId::parse(OTHER_INTENT).expect("UUIDv7")
    }

    fn genesis() -> (WorkflowExecution, WorkflowExecutionEvent) {
        WorkflowExecution::start_from_plan_unchecked(
            intent(),
            WorkflowDefinitionId::parse("claude").expect("定義 id"),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision"),
            &StartRequest::new("classic", "unit"),
            vec![StageEntry::new(
                StageSlug::parse("state-init").expect("slug"),
                PhaseId::Initialization,
                PlanAction::Execute,
                false,
            )],
            at(),
        )
        .expect("合成計画は start の前提を満たす")
    }

    fn repository() -> WorkflowExecutionRepositoryImpl<MemoryStore> {
        WorkflowExecutionRepositoryImpl::in_memory()
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
    fn an_event_of_another_aggregate_fails_the_precondition() {
        let (aggregate, event) = genesis();
        let foreign = WorkflowExecutionEvent::new(
            other_intent(),
            event.seq_nr(),
            at(),
            WorkflowExecutionEventPayload::Unparked,
        );
        assert_eq!(
            WorkflowExecutionRepositoryImpl::<MemoryStore>::check_preconditions(
                &foreign, &aggregate
            ),
            Err(RepositoryError::Corrupt {
                aggregate_id: other_intent(),
                seq_nr: Some(1),
                cause: CorruptCause::SequenceGap,
            })
        );
    }

    #[test]
    fn a_sequence_below_the_first_one_fails_the_precondition() {
        // `seq_nr` = 0 の封筒は「まだ 1 件も適用していない集約の写し」を名乗る — 書込経路
        // からは決して生まれない (BR1.1)。
        let (aggregate, _) = genesis();
        let zero =
            WorkflowExecutionEvent::new(intent(), 0, at(), WorkflowExecutionEventPayload::Unparked);
        assert_eq!(
            WorkflowExecutionRepositoryImpl::<MemoryStore>::check_preconditions(&zero, &aggregate),
            Err(RepositoryError::Corrupt {
                aggregate_id: intent(),
                seq_nr: Some(0),
                cause: CorruptCause::SequenceGap,
            })
        );
    }

    #[test]
    fn the_precondition_passes_for_the_event_the_command_just_produced() {
        let (aggregate, event) = genesis();
        assert_eq!(
            WorkflowExecutionRepositoryImpl::<MemoryStore>::check_preconditions(&event, &aggregate),
            Ok(())
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
                    &aggregate
                )
                .await,
            RepositoryError::Conflict {
                expected: 0,
                actual: 0,
            },
            "実在する version は読み直して材料にする (行が無ければ 0)"
        );
        assert_eq!(
            repository
                .write_error(
                    EventStoreWriteError::SerializationError(Box::new(std::io::Error::other("x"))),
                    &aggregate
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
                .write_error(EventStoreWriteError::IOError(boxed_busy()), &aggregate)
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
                    &aggregate
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
        repository.store(&event, &aggregate).await.expect("genesis");
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
        repository.store(&event, &aggregate).await.expect("genesis");

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
