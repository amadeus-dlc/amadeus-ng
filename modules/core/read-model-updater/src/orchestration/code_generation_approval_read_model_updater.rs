//! Code Generation の開始可否の参照面（`read_code_generation_approval`）を最新化する更新器。

use core_command_domain::orchestration::{IntentExecutionId, PlanApprovalInput, PlanReceipts};

use super::global_seq_nr::GlobalSeqNr;
use super::journal_reader::JournalReader;
use super::read_model_update_error::ReadModelUpdateError;
use super::read_model_updater::ReadModelUpdater;

/// 対象ごとの開始可否の参照面を、空間側の履歴と共有側の受領から再投影する。
///
/// 判断の材料は 2 つのジャーナルにまたがる — 空間側の集約（`IntentExecution` /
/// `Intent`）はここで `replay` し、共有ランタイム側の受領は
/// [`super::plan_approval_receipts`] が読んだものを受け取る。投影核
/// （[`crate::read_tables::CodeGenerationApprovalRow::project`]）はどちらの読み手も
/// 知らず、材料だけを受け取る。
///
/// 読み手・実行・承認入力・受領は**借りる**。承認入力は呼出側が開始可否の判定後も
/// 続けて使う（計画・手順書の本文を返す）ので、更新器が所有を奪わない。
#[derive(Debug)]
pub struct CodeGenerationApprovalReadModelUpdater<'a, R> {
    journal_reader: &'a mut R,
    execution_id: &'a IntentExecutionId,
    input: &'a PlanApprovalInput,
    receipts: &'a PlanReceipts,
}

impl<'a, R: JournalReader> CodeGenerationApprovalReadModelUpdater<'a, R> {
    /// 読み手・対象実行・現在の承認入力・共有側の受領を束ねる。
    pub const fn new(
        journal_reader: &'a mut R,
        execution_id: &'a IntentExecutionId,
        input: &'a PlanApprovalInput,
        receipts: &'a PlanReceipts,
    ) -> Self {
        Self {
            journal_reader,
            execution_id,
            input,
            receipts,
        }
    }
}

impl<R: JournalReader> ReadModelUpdater for CodeGenerationApprovalReadModelUpdater<'_, R> {
    type Error = ReadModelUpdateError;

    /// 全履歴と受領から開始可否の行を組み、参照面を差し替える。
    ///
    /// # Errors
    ///
    /// 履歴の再構成、または参照面の書込みに失敗した場合。
    async fn update_read_models(&mut self) -> Result<(), ReadModelUpdateError> {
        self.journal_reader.prepare_read_model()?;
        let history = self.journal_reader.events_after(GlobalSeqNr::ZERO).await?;
        let row = crate::read_tables::CodeGenerationApprovalRow::project(
            &history,
            self.execution_id,
            self.input,
            self.receipts,
        )?;
        self.journal_reader
            .replace_code_generation_approval(&row)
            .await?;
        Ok(())
    }
}
