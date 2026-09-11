//! runtime-graph の compile が読んだ日誌観測を既存の実行へ記録する。
use super::{IntentExecutionRepository, MemoryJournalError};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{IntentExecutionId, MemoryJournalSurvey};
/// 日誌観測の保存だけを行う更新ユースケース。
///
/// どの位置へ `MEMORY_EMPTY` を記録するかは集約が決める — ここでは判定しない
/// (`coding-rules/tell-dont-ask.md`)。
#[derive(Debug)]
pub struct ObserveMemoryJournalsUseCase<R: IntentExecutionRepository> {
    repository: R,
}
impl<R: IntentExecutionRepository> ObserveMemoryJournalsUseCase<R> {
    /// 既存作業の保存ポートを注入する。
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
    /// 観測を保存する。呼出側は compile の発火条件を先に確かめる。
    /// # Errors
    /// 対象不在、保存失敗、集約の採番失敗。
    pub async fn execute(
        &mut self,
        id: &IntentExecutionId,
        survey: MemoryJournalSurvey,
        at: DateTime<Utc>,
    ) -> Result<(), MemoryJournalError> {
        let mut execution = self
            .repository
            .find_by_id(id)
            .await
            .map_err(MemoryJournalError::Repository)?;
        let event = execution
            .observe_memory_journals(survey, at)
            .map_err(MemoryJournalError::Command)?;
        self.repository
            .store(&event, &execution)
            .await
            .map_err(MemoryJournalError::Repository)
    }
}
