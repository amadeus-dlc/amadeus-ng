//! 構造化面（系統 (2) — SQLite の `read_*` 表）だけを最新化する更新器。

use crate::read_tables::ReadTables;

use super::global_seq_nr::GlobalSeqNr;
use super::journal_reader::JournalReader;
use super::projection_name::ProjectionName;
use super::read_model_update_error::ReadModelUpdateError;
use super::read_model_updater::ReadModelUpdater;

/// Markdown の投影先がまだ無い初回起動で、構造化面だけを最新化する。
///
/// fresh workspace では intent の記録ディレクトリも `aidlc-state.md` もまだ存在しないが、
/// 最初の `next` は定義・scope・費用の行を読めなければ `intent-create` を名指せない。
/// その初回起動だけがこの更新器を使う。通常の実行では
/// [`super::OrchestrationReadModelUpdater`] が Markdown 面と構造化面を同じ履歴断面から
/// 一緒に描く。
///
/// 読み手は**借りる**。呼出側は更新の後も同じ読み手でチェックポイントを確かめたり、
/// 続けて別の書込を行ったりするので、所有を奪わない。
#[derive(Debug)]
pub struct StructuredReadModelUpdater<'a, R> {
    journal_reader: &'a mut R,
    projection: &'a ProjectionName,
}

impl<'a, R: JournalReader> StructuredReadModelUpdater<'a, R> {
    /// 読み手と、進める投影チェックポイントの名前を束ねる。
    pub const fn new(journal_reader: &'a mut R, projection: &'a ProjectionName) -> Self {
        Self {
            journal_reader,
            projection,
        }
    }
}

impl<R: JournalReader> ReadModelUpdater for StructuredReadModelUpdater<'_, R> {
    type Error = ReadModelUpdateError;

    /// 差分があれば全履歴から構造化面を描き直し、行とチェックポイントを同じ
    /// トランザクションで進める。差分が空なら何も書かない。
    ///
    /// # Errors
    ///
    /// ジャーナル・チェックポイントの読取、構造化投影、またはチェックポイントと行の
    /// 同一トランザクション更新に失敗した場合。
    async fn update_read_models(&mut self) -> Result<(), ReadModelUpdateError> {
        self.journal_reader.prepare_read_model()?;
        let checkpoint = self.journal_reader.checkpoint(self.projection).await?;
        if self
            .journal_reader
            .events_after(checkpoint)
            .await?
            .scanned_to()
            .is_none()
        {
            return Ok(());
        }

        let history = self.journal_reader.events_after(GlobalSeqNr::ZERO).await?;
        let last = history
            .scanned_to()
            .ok_or(ReadModelUpdateError::HistoryDisappeared)?;
        let tables = ReadTables::project(&history)?;
        self.journal_reader
            .advance_checkpoint(self.projection, last, &tables)
            .await?;
        Ok(())
    }
}
