//! 指示発行をイベントとして保存する。
use super::{IntentExecutionRepository, InteractionCommandError};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{DirectivePublication, IntentExecutionId};
/// 表示前の発行記録。成功戻り値に表示材料は含めない。
#[derive(Debug)]
pub struct IssueDirectiveUseCase<R: IntentExecutionRepository> {
    repository: R,
}
impl<R: IntentExecutionRepository> IssueDirectiveUseCase<R> {
    /// 保存ポートを注入する。
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
    /// 指示を集約で確定して保存する。
    /// # Errors
    /// 集約の拒否または読込み・保存失敗。
    pub async fn execute(
        &mut self,
        id: &IntentExecutionId,
        publication: &DirectivePublication,
        at: DateTime<Utc>,
    ) -> Result<(), InteractionCommandError> {
        let mut aggregate = self
            .repository
            .find_by_id(id)
            .await
            .map_err(InteractionCommandError::Repository)?;
        let event = aggregate
            .issue_directive(publication, at)
            .map_err(InteractionCommandError::Command)?;
        self.repository
            .store(&event, &aggregate)
            .await
            .map_err(InteractionCommandError::Repository)
    }
}
