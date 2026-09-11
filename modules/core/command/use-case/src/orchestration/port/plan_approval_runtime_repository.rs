//! 共有承認集約の取得と、単一イベントの保存ポート。
use super::RepositoryError;
use core_command_domain::orchestration::{
    PlanApprovalEvent, PlanApprovalRuntime, PlanApprovalRuntimeId,
};
/// 保持状態の判断は集約、媒体への読み書きは実装が担当する。
#[allow(
    async_fn_in_trait,
    reason = "既存Repositoryと同じcurrent_thread契約でありSend境界を要求しない"
)]
pub trait PlanApprovalRuntimeRepository {
    /// 最新スナップショットと以後の事実から同じ集約を再構成する。
    /// # Errors
    /// 不在・破損・I/Oの失敗。
    async fn find_by_id(
        &self,
        id: &PlanApprovalRuntimeId,
    ) -> Result<PlanApprovalRuntime, RepositoryError<PlanApprovalRuntimeId>>;
    /// 集約が生成した1イベントと、その適用後の状態を保存する。
    /// # Errors
    /// 競合・破損・I/Oの失敗。
    async fn store(
        &mut self,
        event: &PlanApprovalEvent,
        aggregate: &PlanApprovalRuntime,
    ) -> Result<(), RepositoryError<PlanApprovalRuntimeId>>;
}
