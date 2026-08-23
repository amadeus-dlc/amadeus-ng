//! `WorkflowExecutionRepository` ポート — 集約 `WorkflowExecution` の ES 形 Repository (C3 / ADR-006)。

use core_domain::orchestration::{IntentId, WorkflowExecution, WorkflowExecutionEvent};

use super::repository_error::RepositoryError;

/// 集約 `WorkflowExecution` の Repository (イベントソーシング形 — ADR-006 / C3)。
///
/// 動詞は本家ライブラリ (event-store-adapter-rs) の語彙に従い `store` / `find_by_id`。
/// ステートソーシング Repository の `save` は持たない
/// (`coding-rules/gateway-taxonomy.md` §2b の ES 拡張語彙)。
///
/// トランザクションの所有は**実装側**であり、ユースケースは Tx を持たない (C3 ②)。
/// `Conflict` 以外は再試行しない — 再試行の政策はユースケースの責務である (C3 ③)。
/// メソッドは `&self` の `async fn` で、書込のための内部可変性 (`RefCell`) は実装が持つ。
///
/// 実装は `core-interface-adapter`
/// (`orchestration::WorkflowExecutionRepositoryImpl` が実 I/O、
/// `orchestration::InMemoryWorkflowExecutionRepository` がテストダブル)。
#[allow(
    async_fn_in_trait,
    reason = "Send 境界を意図的に要求しない設計 (C3 / Q3 = A — tokio current_thread)。\
              自動 trait 境界を書けないという注意喚起は本 trait では設計どおりである。"
)]
pub trait WorkflowExecutionRepository {
    /// 集約を**完全に**再構成して返す (部分データを返さない — C3 ①)。
    ///
    /// 最新スナップショットを復元し、その `seq_nr` より後のイベントを昇順に適用したうえで、
    /// 永続化済みの最後の `seq_nr` を楽観 version として載せる (BR1.2)。
    ///
    /// # Errors
    ///
    /// 集約が無い (`NotFound`)、ストア I/O (`Io`)、スナップショット欠落・復号不能・
    /// 不変条件違反・`seq_nr` の不連続 (`Corrupt`) を返す。
    async fn find_by_id(&self, id: &IntentId) -> Result<WorkflowExecution, RepositoryError>;

    /// 1 コマンドが返した単一イベントと適用後の集約を、同一トランザクションで永続化する。
    ///
    /// 期待 version は `aggregate.version()` (= `event.seq_nr() - 1`)。一致しなければ
    /// `Conflict` で、ストアの状態は変わらない (BR1.3)。引数は `&` なので呼出側の集約は
    /// 変更されない — 続けて書くには再水和が要る。
    ///
    /// # Errors
    ///
    /// 楽観 version の不一致 (`Conflict`)、ストア I/O (`Io`)、呼出側の不整合・符号化の失敗
    /// (`Corrupt`) を返す。
    async fn store(
        &self,
        event: &WorkflowExecutionEvent,
        aggregate: &WorkflowExecution,
    ) -> Result<(), RepositoryError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::RepositoryError;
    use core_domain::orchestration::{
        IntentId, StageEntry, StartRequest, WorkflowExecution, WorkflowExecutionEvent,
    };
    use core_domain::workflow_definition::{
        DefinitionRevision, PhaseId, PlanAction, StageSlug, WorkflowDefinitionId,
    };
    use std::cell::RefCell;

    const RAW_ID: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

    fn intent() -> IntentId {
        IntentId::parse(RAW_ID).unwrap()
    }

    fn genesis() -> (WorkflowExecution, WorkflowExecutionEvent) {
        WorkflowExecution::start_with_entries(
            intent(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            &StartRequest::new("classic", "port shape"),
            vec![StageEntry::new(
                StageSlug::parse("state-init").unwrap(),
                PhaseId::Initialization,
                PlanAction::Execute,
                false,
            )],
            "2026-08-23T00:00:00Z",
        )
        .unwrap()
    }

    /// trait の形 (`&self` の async fn・`dyn` なし) を固定するための最小実装。
    /// 内部可変性は設計どおり `RefCell` で橋渡しする。
    #[derive(Debug, Default)]
    struct FakeRepository {
        stored: RefCell<Option<WorkflowExecution>>,
    }

    impl WorkflowExecutionRepository for FakeRepository {
        async fn find_by_id(&self, id: &IntentId) -> Result<WorkflowExecution, RepositoryError> {
            self.stored
                .borrow()
                .clone()
                .ok_or_else(|| RepositoryError::NotFound {
                    intent_id: id.clone(),
                })
        }

        async fn store(
            &self,
            event: &WorkflowExecutionEvent,
            aggregate: &WorkflowExecution,
        ) -> Result<(), RepositoryError> {
            *self.stored.borrow_mut() = Some(aggregate.clone().with_version(event.seq_nr()));
            Ok(())
        }
    }

    /// ジェネリック関数からポート越しに使えること (静的束縛 — ユースケースはこの形で組む)。
    async fn rehydrate<R: WorkflowExecutionRepository>(
        repository: &R,
        id: &IntentId,
    ) -> Result<WorkflowExecution, RepositoryError> {
        repository.find_by_id(id).await
    }

    #[tokio::test]
    async fn an_unknown_aggregate_is_not_found() {
        let repository = FakeRepository::default();
        let err = rehydrate(&repository, &intent()).await.unwrap_err();
        assert_eq!(
            err,
            RepositoryError::NotFound {
                intent_id: intent()
            }
        );
    }

    #[tokio::test]
    async fn a_stored_aggregate_is_rehydrated_by_its_identifier() {
        let repository = FakeRepository::default();
        let (aggregate, event) = genesis();
        repository.store(&event, &aggregate).await.unwrap();
        let found = rehydrate(&repository, &intent()).await.unwrap();
        assert_eq!(found.intent_id(), &intent());
    }

    #[tokio::test]
    async fn the_store_leaves_the_optimistic_version_at_the_last_persisted_sequence() {
        let repository = FakeRepository::default();
        let (aggregate, event) = genesis();
        assert_eq!(aggregate.version(), 0);
        repository.store(&event, &aggregate).await.unwrap();
        let found = rehydrate(&repository, &intent()).await.unwrap();
        assert_eq!(found.version(), event.seq_nr());
    }

    #[tokio::test]
    async fn the_port_takes_the_aggregate_by_reference_so_the_caller_keeps_it() {
        let repository = FakeRepository::default();
        let (aggregate, event) = genesis();
        repository.store(&event, &aggregate).await.unwrap();
        // 引数は `&` — store は呼出側の集約を変更しない (BR1.3)。
        assert_eq!(aggregate.version(), 0);
        assert_eq!(aggregate.seq_nr(), 1);
    }

    #[tokio::test]
    async fn the_repository_face_reports_its_failures_as_repository_errors() {
        let repository = FakeRepository::default();
        let err = repository.find_by_id(&intent()).await.unwrap_err();
        assert!(matches!(err, RepositoryError::NotFound { .. }));
    }
}
