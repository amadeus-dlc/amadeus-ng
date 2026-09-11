//! §13 の儀式で確定した学びを、surface の時点で固定した作業へ記録する。
use super::{IntentExecutionRepository, LearningCaptureError};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    IntentExecutionId, LearningObservations, LearningProvenance,
};
use core_command_domain::workflow_definition::StageSlug;
/// 学びの保存だけを行う更新ユースケース。
///
/// どの学びをどちらの側へ書くかは集約が決める — ここでは判定しない
/// (`coding-rules/tell-dont-ask.md`)。成功は `Result<(), E>` であり、表示材料を返す別経路を
/// 作らない (`coding-rules/command-query-separation.md`)。
#[derive(Debug)]
pub struct CaptureLearningsUseCase<R: IntentExecutionRepository> {
    repository: R,
}
impl<R: IntentExecutionRepository> CaptureLearningsUseCase<R> {
    /// 既存作業の保存ポートを注入する。
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
    /// 学びを保存する。呼出側は両側の実測を先に済ませる。
    /// # Errors
    /// 対象不在、保存失敗、集約の拒否。
    pub async fn execute(
        &mut self,
        id: &IntentExecutionId,
        stage: &StageSlug,
        provenance: LearningProvenance,
        observations: &LearningObservations,
        at: DateTime<Utc>,
    ) -> Result<(), LearningCaptureError> {
        let mut execution = self
            .repository
            .find_by_id(id)
            .await
            .map_err(LearningCaptureError::Repository)?;
        let event = execution
            .capture_learnings(stage, provenance, observations, at)
            .map_err(LearningCaptureError::Command)?;
        self.repository
            .store(&event, &execution)
            .await
            .map_err(LearningCaptureError::Repository)
    }
}
