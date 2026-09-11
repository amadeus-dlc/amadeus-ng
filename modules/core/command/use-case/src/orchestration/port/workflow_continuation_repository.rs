//! 実行ごとの停止制御の取得と保存。
use super::RepositoryError;
use core_command_domain::orchestration::{
    WorkflowContinuation, WorkflowContinuationEvent, WorkflowContinuationId,
};
#[allow(
    async_fn_in_trait,
    reason = "既存のcurrent_thread Repositoryポートと同型"
)]
/// 実行ごとの停止制御の取得と保存。
pub trait WorkflowContinuationRepository {
    /// 最新snapshotと差分イベントを読む。
    /// # Errors
    /// 不在、破損、IO障害。
    async fn find_by_id(
        &self,
        id: &WorkflowContinuationId,
    ) -> Result<WorkflowContinuation, RepositoryError<WorkflowContinuationId>>;
    /// 単一事実と適用後の集約を保存する。
    /// # Errors
    /// 競合、破損、IO障害。
    async fn store(
        &mut self,
        event: &WorkflowContinuationEvent,
        aggregate: &WorkflowContinuation,
    ) -> Result<(), RepositoryError<WorkflowContinuationId>>;
}
