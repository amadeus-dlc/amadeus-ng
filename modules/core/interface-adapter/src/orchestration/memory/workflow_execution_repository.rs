//! `InMemoryWorkflowExecutionRepository` — `WorkflowExecutionRepository` の in-memory 実装。
//!
//! 実 Gateway (`WorkflowExecutionRepositoryImpl`) と**同じ手順**で再水和と書込を行い、
//! 違うのは内包するストアだけである (BR2.7)。ユースケース (U5 / U6) のテストはこれで組む
//! (C3 ④)。テストダブルなので `Impl` 接尾辞は付けない
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md)。

use core_domain::orchestration::{ApplyError, IntentId, WorkflowExecution, WorkflowExecutionEvent};
use core_use_case::orchestration::{
    CorruptCause, EventStore, RepositoryError, WorkflowExecutionRepository,
};

use super::in_memory_event_store::InMemoryEventStore;

/// in-memory ストアを**単一所有**する `WorkflowExecutionRepository`。
///
/// 実 Gateway (`WorkflowExecutionRepositoryImpl`) と同じ形である — 内部可変性を持たず、
/// 再構成 (Query) は `&self`、永続化 (Command) は `&mut self`
/// (`coding-rules/interior-mutability.md` / `coding-rules/command-query-separation.md`)。
#[derive(Debug, Default)]
pub struct InMemoryWorkflowExecutionRepository {
    store: InMemoryEventStore,
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

impl InMemoryWorkflowExecutionRepository {
    /// 受け取ったストアを内包する Repository を作る。
    ///
    /// 空のストアから始めたい場合は `Default` を使う (`InMemoryWorkflowExecutionRepository::default()`)。
    /// 既存のストアを渡す形は「別プロセスからの再オープン相当」を表す。
    /// SQLite 実装 `WorkflowExecutionRepositoryImpl::new(store)` と同じ形にしてある
    /// (coding-rules/factory-naming.md — コンストラクタ相当は `new` に統一)。
    #[must_use]
    pub const fn new(store: InMemoryEventStore) -> InMemoryWorkflowExecutionRepository {
        InMemoryWorkflowExecutionRepository { store }
    }

    /// 内包しているストアへの参照 (`JournalReader` の読取に使う Query)。
    ///
    /// 所有権も別ハンドルも配らない — 実 Gateway と同じ形である
    /// (`coding-rules/interior-mutability.md`)。
    #[must_use]
    pub const fn event_store(&self) -> &InMemoryEventStore {
        &self.store
    }

    /// 内包しているストアへの可変参照 (`advance_checkpoint` など書込に使う Command 側の口)。
    ///
    /// Query (`event_store`) と分けてあるのは CQS のためである
    /// (`coding-rules/command-query-separation.md`)。
    pub const fn event_store_mut(&mut self) -> &mut InMemoryEventStore {
        &mut self.store
    }

    /// 呼出側の不整合 (BR1.3 の前提検査)。破ったら `Corrupt(SequenceGap)`。
    fn check_preconditions(
        event: &WorkflowExecutionEvent,
        aggregate: &WorkflowExecution,
    ) -> Result<(), RepositoryError> {
        let sequence_gap = RepositoryError::Corrupt {
            aggregate_id: event.intent_id().clone(),
            seq_nr: Some(event.seq_nr()),
            cause: CorruptCause::SequenceGap,
        };
        if event.intent_id() != aggregate.intent_id()
            || event.seq_nr() != aggregate.seq_nr()
            || event.seq_nr() < 1
        {
            return Err(sequence_gap);
        }
        // `seq_nr >= 1` を先に確かめてから引く (u64 のアンダーフロー防止 — NFR4.3)。
        if aggregate.version() != event.seq_nr() - 1 {
            return Err(sequence_gap);
        }
        Ok(())
    }
}

impl WorkflowExecutionRepository for InMemoryWorkflowExecutionRepository {
    async fn find_by_id(&self, id: &IntentId) -> Result<WorkflowExecution, RepositoryError> {
        let store = &self.store;
        let snapshot = store
            .get_latest_snapshot_by_id(id)
            .await
            .map_err(|error| RepositoryError::from_event_store(error, id))?;
        let Some(mut aggregate) = snapshot else {
            // ジャーナル行が 1 件も無ければ「まだ無い」、あるなら「壊れている」(BR1.2)。
            return Err(if store.journal_is_empty(id) {
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
        let events = store
            .get_events_by_id_since_seq_nr(id, aggregate.seq_nr())
            .await
            .map_err(|error| RepositoryError::from_event_store(error, id))?;
        let mut version = aggregate.version();
        for event in &events {
            aggregate
                .apply_event(event)
                .map_err(|error| RepositoryError::Corrupt {
                    aggregate_id: id.clone(),
                    seq_nr: Some(event.seq_nr()),
                    cause: apply_cause(&error),
                })?;
            version = event.seq_nr();
        }
        // replay の後に Repository が明示的に版を載せる (`apply_event` は版を動かさない)。
        Ok(aggregate.with_version(version))
    }

    async fn store(
        &mut self,
        event: &WorkflowExecutionEvent,
        aggregate: &WorkflowExecution,
    ) -> Result<(), RepositoryError> {
        InMemoryWorkflowExecutionRepository::check_preconditions(event, aggregate)?;
        self.store
            .persist_event_and_snapshot(event, aggregate)
            .await
            .map_err(|error| RepositoryError::from_event_store(error, event.intent_id()))
    }
}
