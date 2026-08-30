//! `CommandError` — 状態遷移コマンドの拒否理由 (functional-spec §5)。

use std::fmt;

use super::stage_index::StageIndex;
use crate::workspace::CheckboxState;

/// ガード違反は「発火しないアクション」であって状態は一切動かない (モデルの enabled 条件と同型)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    /// 判断に渡された `Intent` がこの実行のものでない (識別子不一致、または計画長の不一致)。
    ///
    /// 集約は intent を ID で参照するので、この照合が書ける
    /// (coding-rules/aggregate-references.md)。
    IntentMismatch,
    /// コマンドを受理できない — Completed、または park が活性 (BR1.0)。
    NotRunning,
    /// checkbox 前提の不一致 (BR1.3 / BR1.4 / BR1.5)。
    CheckboxPrecondition {
        /// 前提を満たさなかったステージ。
        stage: StageIndex,
        /// そのステージの実測 checkbox。受理される前提集合はコマンドごとに異なるため、
        /// ここは期待値ではなく**観測値**を運ぶ。
        actual: CheckboxState,
    },
    /// skipped 受理条件の不成立 (CONDITIONAL でも実効 SKIP でもない — BR1.5)。
    NotSkippable(StageIndex),
    /// stale re-report の前提不一致 (BR1.9)。
    NotStale(StageIndex),
    /// jump / recompose / ゲート系コマンドの対象不正 (BR1.3 / BR1.6 / BR1.8)。
    InvalidTarget(StageIndex),
    /// autonomous 下で拒否されるコマンド (park / recompose — BR1.7 / BR1.8)。
    RefusedUnderAutonomy,
    /// 通番が `usize::MAX` に達しており、新しいイベントを採番できない (通番枯渇)。
    /// 実運用では到達しない規模だが、境界を暗黙の飽和にしない (NFR4.3)。
    SequenceExhausted,
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::IntentMismatch => f.write_str("intent mismatch"),
            CommandError::NotRunning => f.write_str("not running"),
            CommandError::CheckboxPrecondition { stage, actual } => write!(
                f,
                "stage {stage} checkbox precondition: actual [{}]",
                actual.marker()
            ),
            CommandError::NotSkippable(stage) => write!(f, "stage {stage} is not skippable"),
            CommandError::NotStale(stage) => write!(f, "stage {stage} is not a stale re-report"),
            CommandError::InvalidTarget(stage) => write!(f, "invalid target stage {stage}"),
            CommandError::RefusedUnderAutonomy => f.write_str("refused under autonomous mode"),
            CommandError::SequenceExhausted => {
                f.write_str("sequence exhausted: seq_nr is at usize::MAX")
            }
        }
    }
}

impl std::error::Error for CommandError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::StageIndex;
    use crate::workspace::CheckboxState;

    #[test]
    fn the_guard_rejections_carry_material_not_wording() {
        assert_eq!(CommandError::NotRunning.to_string(), "not running");
        assert_eq!(
            CommandError::RefusedUnderAutonomy.to_string(),
            "refused under autonomous mode"
        );
        assert_eq!(
            CommandError::NotSkippable(StageIndex::new(2)).to_string(),
            "stage 2 is not skippable"
        );
        assert_eq!(
            CommandError::NotStale(StageIndex::new(3)).to_string(),
            "stage 3 is not a stale re-report"
        );
        assert_eq!(
            CommandError::InvalidTarget(StageIndex::new(0)).to_string(),
            "invalid target stage 0"
        );
        assert_eq!(
            CommandError::SequenceExhausted.to_string(),
            "sequence exhausted: seq_nr is at usize::MAX"
        );
    }

    #[test]
    fn the_checkbox_precondition_carries_the_observed_state() {
        let err = CommandError::CheckboxPrecondition {
            stage: StageIndex::new(1),
            actual: CheckboxState::Pending,
        };
        assert_eq!(err.to_string(), "stage 1 checkbox precondition: actual [ ]");
    }

    #[test]
    fn the_error_is_a_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(CommandError::NotRunning);
        assert_eq!(err.to_string(), "not running");
    }

    #[test]
    fn rejections_compare_by_value() {
        assert_eq!(
            CommandError::NotSkippable(StageIndex::new(1)),
            CommandError::NotSkippable(StageIndex::new(1))
        );
        assert_ne!(
            CommandError::NotSkippable(StageIndex::new(1)),
            CommandError::NotSkippable(StageIndex::new(2))
        );
    }
}
