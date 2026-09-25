//! 計画指紋の参照面（`read_plan_fingerprint`）を最新化する更新器。

use core_command_domain::orchestration::{IntentExecutionId, PlanApprovalInput};

use super::global_seq_nr::GlobalSeqNr;
use super::journal_reader::JournalReader;
use super::read_model_update_error::ReadModelUpdateError;
use super::read_model_updater::ReadModelUpdater;

/// 計画指紋の参照面を、対象実行の履歴と現在の承認入力から再投影する。
///
/// 読み手・実行・承認入力は**借りる**。承認入力は呼出側が指紋の検証後も続けて使う
/// （計画・手順書の本文）ので、更新器が所有を奪わない。
#[derive(Debug)]
pub struct PlanFingerprintReadModelUpdater<'a, R> {
    journal_reader: &'a mut R,
    execution_id: &'a IntentExecutionId,
    input: &'a PlanApprovalInput,
}

impl<'a, R: JournalReader> PlanFingerprintReadModelUpdater<'a, R> {
    /// 読み手・対象実行・現在の承認入力を束ねる。
    pub const fn new(
        journal_reader: &'a mut R,
        execution_id: &'a IntentExecutionId,
        input: &'a PlanApprovalInput,
    ) -> Self {
        Self {
            journal_reader,
            execution_id,
            input,
        }
    }
}

impl<R: JournalReader> ReadModelUpdater for PlanFingerprintReadModelUpdater<'_, R> {
    type Error = ReadModelUpdateError;

    /// 全履歴から計画指紋の行を組み、参照面を差し替える。
    ///
    /// # Errors
    ///
    /// 履歴の再構成または参照面の書込みに失敗した場合。
    async fn update_read_models(&mut self) -> Result<(), ReadModelUpdateError> {
        self.journal_reader.prepare_read_model()?;
        let history = self.journal_reader.events_after(GlobalSeqNr::ZERO).await?;
        let row = crate::read_tables::PlanFingerprintRow::project(
            &history,
            self.execution_id,
            self.input,
        )?;
        self.journal_reader.replace_plan_fingerprint(&row).await?;
        Ok(())
    }
}
