//! ハーネスの応答観測を保存する更新ユースケース。
use super::{IntentExecutionRepository, InteractionCommandError};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::IntentExecutionId;
/// 応答の記録。人間の回答をCLI側で創作する経路には使わない。
#[derive(Debug)]
pub struct ObservePromptUseCase<R: IntentExecutionRepository> {
    repository: R,
}
impl<R: IntentExecutionRepository> ObservePromptUseCase<R> {
    /// 保存ポートを注入する。
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
    /// ハーネスが観測した応答を単一イベントへ保存する。
    ///
    /// # Errors
    /// 集約の拒否または読込み・保存失敗。
    pub async fn execute(
        &mut self,
        id: &IntentExecutionId,
        session: &str,
        response: &str,
        unattended: bool,
        at: DateTime<Utc>,
    ) -> Result<(), InteractionCommandError> {
        let mut aggregate = self
            .repository
            .find_by_id(id)
            .await
            .map_err(InteractionCommandError::Repository)?;
        let event = aggregate
            .observe_prompt(session, response, unattended, at)
            .map_err(InteractionCommandError::Command)?;
        self.repository
            .store(&event, &aggregate)
            .await
            .map_err(InteractionCommandError::Repository)
    }
}
