//! テスト契約の参照面（`read_testing_*`）を最新化する更新器。

use super::global_seq_nr::GlobalSeqNr;
use super::journal_reader::JournalReader;
use super::read_model_update_error::ReadModelUpdateError;
use super::read_model_updater::ReadModelUpdater;
use super::steering_source::SteeringSource;

/// テスト契約の参照面を、全履歴と memory 層の規則から描き直す。
///
/// 純粋な表示や承認の入力確認の前に呼ぶ。材料の片方は人が編集する規則ファイルなので、
/// 描き直すのは規則の内容ダイジェストが保存済みの値から動いたときだけである。
///
/// 読み手と規則の読取先は**借りる**。呼出側が 1 回の更新のために開いたものを、更新器が
/// 所有する理由が無いため。
#[derive(Debug)]
pub struct TestingReadModelUpdater<'a, R> {
    journal_reader: &'a mut R,
    source: &'a SteeringSource,
}

impl<'a, R: JournalReader> TestingReadModelUpdater<'a, R> {
    /// 読み手と、テスト契約の規則を読む先を束ねる。
    pub const fn new(journal_reader: &'a mut R, source: &'a SteeringSource) -> Self {
        Self {
            journal_reader,
            source,
        }
    }
}

impl<R: JournalReader> ReadModelUpdater for TestingReadModelUpdater<'_, R> {
    type Error = ReadModelUpdateError;

    /// 全履歴と規則からテスト契約の行を組み、内容ダイジェストが変わっていれば差し替える。
    ///
    /// # Errors
    ///
    /// 履歴、規則、参照面の読書きに失敗した場合。
    async fn update_read_models(&mut self) -> Result<(), ReadModelUpdateError> {
        self.journal_reader.prepare_read_model()?;
        let history = self.journal_reader.events_after(GlobalSeqNr::ZERO).await?;
        let sections = self.source.read_testing_sections()?;
        let tables = crate::read_tables::TestingTables::project(&history, &sections);
        if self
            .journal_reader
            .testing_source_digest()
            .await?
            .as_deref()
            != Some(tables.source_digest())
        {
            self.journal_reader.replace_testing(&tables).await?;
        }
        Ok(())
    }
}
