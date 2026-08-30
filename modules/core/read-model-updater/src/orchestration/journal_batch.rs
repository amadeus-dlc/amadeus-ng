//! `JournalBatch` — 差分読取 1 回分の結果 (実行の行・intent の行・走査済み最終位置)。

use core_command_domain::orchestration::Intent;

use super::global_seq_nr::GlobalSeqNr;
use super::journal_entry::JournalEntry;

/// 差分読取 1 回分。
///
/// ジャーナルは実行のストリームと intent のストリームが**同居する 1 本の全順序列**である
/// (issue #50)。読取はその両方をまたいで進むので、結果も 3 つ組で返す:
///
/// - `executions` — 実行のイベント行 (投影核 `project` の入力)。
/// - `intents` — intent の誕生記録 (`Created`) から検査付き再構成した [`Intent`]。状態
///   ファイルの骨格 (全ステージ行・表示属性・走査結果) を描く材料の正本である (issue #56)。
/// - `scanned_to` — 走査した最終行の global 通番 (`None` = 1 行も無かった)。チェックポイント
///   は**この値**まで進める — 実行の最終行ではなく。intent の行しか無いバッチでも前進が
///   止まらないのはこのためである (issue #56 申し送りの解消)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalBatch {
    executions: Vec<JournalEntry>,
    intents: Vec<Intent>,
    scanned_to: Option<GlobalSeqNr>,
}

impl JournalBatch {
    /// 読取結果の 3 つ組から組む (検証はしない — 行を読んだ側が既に済ませている)。
    #[must_use]
    pub const fn new(
        executions: Vec<JournalEntry>,
        intents: Vec<Intent>,
        scanned_to: Option<GlobalSeqNr>,
    ) -> JournalBatch {
        JournalBatch {
            executions,
            intents,
            scanned_to,
        }
    }

    /// 何も無かったバッチ。
    #[must_use]
    pub const fn empty() -> JournalBatch {
        JournalBatch::new(Vec::new(), Vec::new(), None)
    }

    /// 実行のイベント行 (global 通番の昇順)。
    #[must_use]
    pub fn executions(&self) -> &[JournalEntry] {
        &self.executions
    }

    /// intent の誕生記録から再構成した集約値 (出現順)。
    #[must_use]
    pub fn intents(&self) -> &[Intent] {
        &self.intents
    }

    /// 走査した最終行の global 通番 (`None` = 1 行も無かった)。
    #[must_use]
    pub const fn scanned_to(&self) -> Option<GlobalSeqNr> {
        self.scanned_to
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_batch_carries_nothing() {
        let batch = JournalBatch::empty();
        assert!(batch.executions().is_empty());
        assert!(batch.intents().is_empty());
        assert_eq!(batch.scanned_to(), None);
        assert_eq!(batch, JournalBatch::new(Vec::new(), Vec::new(), None));
    }
}
