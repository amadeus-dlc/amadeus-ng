//! `InMemoryWorkflowExecutionRepository` — `WorkflowExecutionRepository` の in-memory 実装。
//!
//! 実 Gateway (`WorkflowExecutionRepositoryImpl`) と**同じ手順**で再水和と書込を行い、
//! 違うのは内包するストアだけである (BR2.7)。ユースケース (U5 / U6) のテストはこれで組む
//! (C3 ④)。テストダブルなので `Impl` 接尾辞は付けない
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md)。

use std::cell::RefCell;

use core_domain::orchestration::{ApplyError, IntentId, WorkflowExecution, WorkflowExecutionEvent};
use core_use_case::orchestration::{
    CorruptCause, EventStore, RepositoryError, WorkflowExecutionRepository,
};

use super::in_memory_event_store::InMemoryEventStore;

/// in-memory ストアを内包する `WorkflowExecutionRepository`。
///
/// `WorkflowExecutionRepository` のメソッドは `&self` (C3) だが、`EventStore` の書込は
/// `&mut self` なので `RefCell` で橋渡しする — 実 Gateway と同じ内部可変性である
/// (tokio current_thread・`Send` 不要なので `Mutex` ではなく `RefCell` — Q3 = A)。
///
/// 借用は **await をまたがない** (設計 functional-spec §2 の約束であり、
/// `clippy::await_holding_refcell_ref` が機械強制する)。in-memory ストアは共有ハンドルなので、
/// 借用はハンドルを複製する一瞬だけで済み、以降の `await` は複製に対して行う。
#[derive(Debug, Default)]
pub struct InMemoryWorkflowExecutionRepository {
    store: RefCell<InMemoryEventStore>,
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
    /// 空のストアを内包する Repository を作る。
    #[must_use]
    pub fn new() -> InMemoryWorkflowExecutionRepository {
        InMemoryWorkflowExecutionRepository::default()
    }

    /// 既存のストアを共有する Repository を作る (別プロセスからの再オープン相当)。
    #[must_use]
    pub const fn with_store(store: InMemoryEventStore) -> InMemoryWorkflowExecutionRepository {
        InMemoryWorkflowExecutionRepository {
            store: RefCell::new(store),
        }
    }

    /// 内包しているストアへのハンドル (`JournalReader` として使うため)。
    #[must_use]
    pub fn event_store(&self) -> InMemoryEventStore {
        self.store.borrow().clone()
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
        // 借用はハンドルの複製までで閉じる (await をまたがない)。
        let store = self.store.borrow().clone();
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
        &self,
        event: &WorkflowExecutionEvent,
        aggregate: &WorkflowExecution,
    ) -> Result<(), RepositoryError> {
        InMemoryWorkflowExecutionRepository::check_preconditions(event, aggregate)?;
        // 借用はハンドルの複製までで閉じる (await をまたがない)。書込は同じ 3 表へ届く。
        let mut store = self.store.borrow().clone();
        store
            .persist_event_and_snapshot(event, aggregate)
            .await
            .map_err(|error| RepositoryError::from_event_store(error, event.intent_id()))
    }
}
