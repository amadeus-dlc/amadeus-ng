//! `WorkflowExecutionRepository` の実 Gateway (1 trait 1 Impl — gateway-taxonomy §5)。
//!
//! 集約 `WorkflowExecution` の再構成と永続化を、[`EventStoreImpl`] を内包して行う。
//! **格納形式が SQLite であることはこの実装の内部詳細**であり、ポート名にも facade にも
//! 現れない (`Sqlite...` のような技術接頭辞を型名に出さないのはそのためである)。
//!
//! 実装が触れるのはドメイン型の公開 API (`state` / `from_state` / `apply_event` / `version` /
//! `with_version`) だけで、集約の内部へ抜ける逃げ道 (`pub(crate)` 昇格・serde derive) は
//! 作らない (BR2.8)。
//!
//! 手順は `InMemoryWorkflowExecutionRepository` と**同一**である。違うのは内包するストア
//! だけで、そうしておくことで契約テスト (`tests/support/contract.rs`) が両方に同じ約束を
//! 課せる (BR2.7)。

use core_domain::orchestration::{ApplyError, IntentId, WorkflowExecution, WorkflowExecutionEvent};
use core_use_case::orchestration::{
    CorruptCause, EventStore, RepositoryError, WorkflowExecutionRepository,
};
use event_store_adapter_rs::types::{Aggregate, Event};

use crate::Clock;

use super::event_store_impl::EventStoreImpl;

/// SQLite ストアを**単一所有**する `WorkflowExecutionRepository` の実装。
///
/// 内部可変性は持たない — 再構成 (Query) は `&self`、永続化 (Command) は `&mut self` で、
/// ストアの `EventStore` 実装のレシーバとそのまま揃う
/// (`coding-rules/interior-mutability.md` / `coding-rules/command-query-separation.md`)。
/// 書込中の排他は借用チェッカが保証するので、`RefCell` の実行時借用検査も
/// `clippy::await_holding_refcell_ref` の心配も要らない。
#[derive(Debug)]
pub struct WorkflowExecutionRepositoryImpl<C> {
    store: EventStoreImpl<C>,
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

impl<C: Clock> WorkflowExecutionRepositoryImpl<C> {
    /// 開いたストアを内包する Repository を作る。
    #[must_use]
    pub const fn new(store: EventStoreImpl<C>) -> WorkflowExecutionRepositoryImpl<C> {
        WorkflowExecutionRepositoryImpl { store }
    }

    /// 内包しているストアへの参照 (`JournalReader` の読取に使う Query)。
    ///
    /// 所有権も別ハンドルも配らない — 配ると同じ接続を指す 2 つ目の口ができ、
    /// `&mut self` の排他性が嘘になる (`coding-rules/interior-mutability.md`)。
    #[must_use]
    pub const fn event_store(&self) -> &EventStoreImpl<C> {
        &self.store
    }

    /// 内包しているストアへの可変参照 (`advance_checkpoint` など書込に使う Command 側の口)。
    ///
    /// Query (`event_store`) と分けてあるのは CQS のためである
    /// (`coding-rules/command-query-separation.md`)。
    pub const fn event_store_mut(&mut self) -> &mut EventStoreImpl<C> {
        &mut self.store
    }

    /// 呼出側の不整合 (BR1.3 の前提検査)。破ったら `Corrupt(SequenceGap)`。
    ///
    /// Tx を開く前に弾くのは、これが**呼出側のバグ**であってストアの状態とは無関係だから
    /// である — 開いてから弾くと、無関係な書き手を `busy_timeout` のあいだ待たせる。
    fn check_preconditions(
        event: &WorkflowExecutionEvent,
        aggregate: &WorkflowExecution,
    ) -> Result<(), RepositoryError> {
        let sequence_gap = RepositoryError::Corrupt {
            aggregate_id: event.aggregate_id().clone(),
            seq_nr: Some(event.seq_nr()),
            cause: CorruptCause::SequenceGap,
        };
        if event.aggregate_id() != aggregate.id()
            || event.seq_nr() != aggregate.seq_nr()
            || event.seq_nr() < 1
        {
            return Err(sequence_gap);
        }
        // `seq_nr >= 1` を先に確かめてから引く (usize のアンダーフロー防止 — NFR4.3)。
        if aggregate.version() != event.seq_nr() - 1 {
            return Err(sequence_gap);
        }
        Ok(())
    }
}

impl<C: Clock> WorkflowExecutionRepository for WorkflowExecutionRepositoryImpl<C> {
    async fn find_by_id(&self, id: &IntentId) -> Result<WorkflowExecution, RepositoryError> {
        let store = &self.store;
        let snapshot = store
            .get_latest_snapshot_by_id(id)
            .await
            .map_err(|error| RepositoryError::from_event_store(error, id))?;
        let Some(mut aggregate) = snapshot else {
            // ジャーナル行が 1 件も無ければ「まだ無い」、あるなら「壊れている」(BR1.2)。
            let empty = store
                .journal_is_empty(id)
                .map_err(|error| RepositoryError::from_event_store(error, id))?;
            return Err(if empty {
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
        aggregate.set_version(version);
        Ok(aggregate)
    }

    async fn store(
        &mut self,
        event: &WorkflowExecutionEvent,
        aggregate: &WorkflowExecution,
    ) -> Result<(), RepositoryError> {
        WorkflowExecutionRepositoryImpl::<C>::check_preconditions(event, aggregate)?;
        self.store
            .persist_event_and_snapshot(event, aggregate)
            .await
            .map_err(|error| RepositoryError::from_event_store(error, event.aggregate_id()))
    }
}
