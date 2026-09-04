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
    /// レビュー会計の対象 slug が実行の計画に無い (b48 / B10)。
    ///
    /// [`InvalidTarget`] とは別物である — あちらは「計画上の位置は在るが不正」であり、
    /// こちらは「その名前を計画が知らない」。
    ///
    /// [`InvalidTarget`]: CommandError::InvalidTarget
    UnknownStage(String),
    /// そのステージはレビュアーを宣言していない (実効クラスの解決が `None` を返した)。
    NoDeclaredReviewer(StageIndex),
    /// `--reviewer` が宣言と食い違う (打ち間違い、または conductor の自己認証)。
    ReviewerMismatch {
        /// 対象ステージ。
        stage: StageIndex,
        /// 定義が宣言しているレビュアー。
        declared: String,
    },
    /// 依頼がこの試行のレビュー予算を超えた (upstream `reviewBudgetMessage`)。
    ReviewBudgetExceeded {
        /// 対象ステージ。
        stage: StageIndex,
        /// 予算を超えた通し番号 (要求値、または次に来るはずだった値)。
        ordinal: u32,
        /// この試行の予算 (advisory は 1、adversarial は `reviewer_max_iterations`)。
        budget: u32,
    },
    /// 依頼の通し番号がこの試行の順序と合わない。
    ReviewOutOfSequence {
        /// 対象ステージ。
        stage: StageIndex,
        /// 要求された通し番号。
        iteration: u32,
        /// この試行が期待する通し番号 (数え上げ済みの依頼数 + 1)。
        expected: u32,
    },
    /// その通し番号の依頼が判定待ちとして残っていない (判定形と retry 形の両方が使う)。
    NoPendingReview {
        /// 対象ステージ。
        stage: StageIndex,
        /// 名指された通し番号。
        iteration: u32,
    },
    /// practices-discovery の承認に、この試行の昇格受領証が無い (段 12、b49)。
    ///
    /// 材料はステージ位置だけである — 「どのレビュアーか」に相当する材料が無く、
    /// 逐語 (`Cannot approve "practices-discovery" before practices-promote succeeds. …`) は
    /// ステージ名すら固定なので、出す側が全部持つ。
    PracticesReceiptMissing(StageIndex),
    /// レビュアーを宣言したステージの承認に、この試行の終端受領証が無い (段 11)。
    ReviewReceiptMissing {
        /// 承認しようとしたステージ。
        stage: StageIndex,
        /// 宣言されているレビュアー。
        reviewer: String,
    },
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
            CommandError::UnknownStage(slug) => write!(f, "unknown stage {slug}"),
            CommandError::NoDeclaredReviewer(stage) => {
                write!(f, "stage {stage} declares no reviewer")
            }
            CommandError::ReviewerMismatch { stage, declared } => {
                write!(f, "stage {stage} declares reviewer {declared}")
            }
            CommandError::ReviewBudgetExceeded {
                stage,
                ordinal,
                budget,
            } => write!(
                f,
                "stage {stage} review request {ordinal} exceeds budget {budget}"
            ),
            CommandError::ReviewOutOfSequence {
                stage,
                iteration,
                expected,
            } => write!(
                f,
                "stage {stage} review iteration {iteration} is out of sequence (expected {expected})"
            ),
            CommandError::NoPendingReview { stage, iteration } => write!(
                f,
                "stage {stage} has no pending review iteration {iteration}"
            ),
            CommandError::PracticesReceiptMissing(stage) => {
                write!(f, "stage {stage} has no practices promotion receipt")
            }
            CommandError::ReviewReceiptMissing { stage, reviewer } => write!(
                f,
                "stage {stage} has no terminal review receipt from {reviewer}"
            ),
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

    /// b49 の昇格受領証の拒否も**材料だけ**を綴る（逐語は app が組む）。
    #[test]
    fn the_practices_receipt_rejection_carries_material_not_wording() {
        assert_eq!(
            CommandError::PracticesReceiptMissing(StageIndex::new(1)).to_string(),
            "stage 1 has no practices promotion receipt"
        );
    }

    /// b48 のレビュー拒否 7 形も**材料だけ**を綴る（逐語は app が組む）。
    #[test]
    fn the_review_rejections_carry_material_not_wording() {
        assert_eq!(
            CommandError::UnknownStage("nowhere".to_string()).to_string(),
            "unknown stage nowhere"
        );
        assert_eq!(
            CommandError::NoDeclaredReviewer(StageIndex::new(1)).to_string(),
            "stage 1 declares no reviewer"
        );
        assert_eq!(
            CommandError::ReviewerMismatch {
                stage: StageIndex::new(1),
                declared: "aidlc-quality-agent".to_string(),
            }
            .to_string(),
            "stage 1 declares reviewer aidlc-quality-agent"
        );
        assert_eq!(
            CommandError::ReviewBudgetExceeded {
                stage: StageIndex::new(2),
                ordinal: 3,
                budget: 2,
            }
            .to_string(),
            "stage 2 review request 3 exceeds budget 2"
        );
        assert_eq!(
            CommandError::ReviewOutOfSequence {
                stage: StageIndex::new(2),
                iteration: 3,
                expected: 1,
            }
            .to_string(),
            "stage 2 review iteration 3 is out of sequence (expected 1)"
        );
        assert_eq!(
            CommandError::NoPendingReview {
                stage: StageIndex::new(1),
                iteration: 2,
            }
            .to_string(),
            "stage 1 has no pending review iteration 2"
        );
        assert_eq!(
            CommandError::ReviewReceiptMissing {
                stage: StageIndex::new(1),
                reviewer: "aidlc-quality-agent".to_string(),
            }
            .to_string(),
            "stage 1 has no terminal review receipt from aidlc-quality-agent"
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
