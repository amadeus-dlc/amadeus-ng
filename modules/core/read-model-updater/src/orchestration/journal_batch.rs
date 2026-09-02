//! `JournalBatch` — 差分読取 1 回分の結果 (実行の行・intent の行・定義の行・走査済み最終位置)。

use core_command_domain::orchestration::Intent;

use super::definition_entry::DefinitionEntry;
use super::global_seq_nr::GlobalSeqNr;
use super::journal_entry::JournalEntry;

/// 差分読取 1 回分。
///
/// ジャーナルは実行・intent・定義の 3 ストリームが**同居する 1 本の全順序列**である
/// (issue #50 / 2026-08-31 の定義 ES 転換)。読取はその全部をまたいで進むので、結果も
/// 4 つ組で返す:
///
/// - `executions` — 実行のイベント行 (投影核 `project` の入力)。
/// - `intents` — intent の誕生記録 (`Created`) から検査付き再構成した [`Intent`]。状態
///   ファイルの骨格 (全ステージ行・表示属性・走査結果) を描く材料の正本である (issue #56)。
/// - `definitions` — 定義のイベント行 ([`DefinitionEntry`])。構造化リードモデル
///   (`read_definition*` 表) を描く材料であり、`Defined` を種に `Redefined` を畳めば
///   その時点の [`WorkflowDefinition`] が復元できる (b39 — `cqrs-boundaries.md` 規則 3 の
///   2026-09-02 追記「投影核は集約を `replay` で起こしてクエリメソッドを呼ぶ」)。
/// - `scanned_to` — 走査した最終行の global 通番 (`None` = 1 行も無かった)。チェックポイント
///   は**この値**まで進める — 実行の最終行ではなく。intent や定義の行しか無いバッチでも
///   前進が止まらないのはこのためである (issue #56 申し送りの解消)。
///
/// [`WorkflowDefinition`]: core_command_domain::workflow_definition::WorkflowDefinition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalBatch {
    executions: Vec<JournalEntry>,
    intents: Vec<Intent>,
    definitions: Vec<DefinitionEntry>,
    scanned_to: Option<GlobalSeqNr>,
}

impl JournalBatch {
    /// 読取結果の 4 つ組から組む (検証はしない — 行を読んだ側が既に済ませている)。
    #[must_use]
    pub const fn new(
        executions: Vec<JournalEntry>,
        intents: Vec<Intent>,
        definitions: Vec<DefinitionEntry>,
        scanned_to: Option<GlobalSeqNr>,
    ) -> JournalBatch {
        JournalBatch {
            executions,
            intents,
            definitions,
            scanned_to,
        }
    }

    /// 何も無かったバッチ。
    #[must_use]
    pub const fn empty() -> JournalBatch {
        JournalBatch::new(Vec::new(), Vec::new(), Vec::new(), None)
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

    /// 定義のイベント行 (global 通番の昇順)。
    #[must_use]
    pub fn definitions(&self) -> &[DefinitionEntry] {
        &self.definitions
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
        assert!(batch.definitions().is_empty());
        assert_eq!(batch.scanned_to(), None);
        assert_eq!(
            batch,
            JournalBatch::new(Vec::new(), Vec::new(), Vec::new(), None)
        );
    }
}
