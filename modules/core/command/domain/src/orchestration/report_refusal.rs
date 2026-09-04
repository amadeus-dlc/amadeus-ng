//! `ReportRefusal` — [`IntentExecution::report_dispatch`] の拒否 (材料のみ)。
//!
//! [`IntentExecution::report_dispatch`]: super::IntentExecution::report_dispatch

use std::fmt;

use super::verdict::Verdict;
use crate::workflow_definition::{ExecutionKind, StageSlug};
use crate::workspace::CheckboxState;

/// 報告を受理できない理由と、その拒否だけが要する材料。
///
/// `CommandError` とは**別の型**である — あちらは集約コマンド自身のガードであり、こちらは
/// コマンドを打つ前に「そもそもどの遷移も打たない」と決めた判断である。逐語文言は出す側
/// (合成ルートの `wording`) が組み、ここは材料しか運ばない
/// (`coding-rules/error-handling.md`)。
///
/// 変種の順序はピン `handleReport` の判定順 (`:5545-5860`) に合わせてある。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportRefusal {
    /// 段 8 — 名指しされた slug が解決済み計画に無い。
    ///
    /// 材料が `String` なのは、合成ルート側の「そもそも slug の文法でない `--stage`」も
    /// **同じ逐語**で拒む (設計 §2 段 7〜8) からである — 生値をそのまま運べる形に揃える。
    UnknownStage {
        /// 名指しされた生値。
        named: String,
    },
    /// 遷移をコミットしない結末 (`resume` / `resumed`) が集約まで届いた。
    ///
    /// 合成ルートが段 4 で分岐するので**通常は到達しない** (`coding-rules/use-case-rules.md`
    /// §3)。判断を全域にするために閉集合へ含める。
    RoutedVerdict {
        /// 届いた結末。
        verdict: Verdict,
    },
    /// 段 9 — CONDITIONAL でも実効 SKIP でもないステージが `skipped` を名乗った。
    SkipNotConditional {
        /// 対象ステージ。
        stage: StageSlug,
        /// そのステージの宣言 (逐語 `is execution: <ALWAYS|CONDITIONAL>` の材料)。
        execution: ExecutionKind,
    },
    /// 段 9 — `--reason` が空。
    SkipRequiresReason {
        /// 対象ステージ。
        stage: StageSlug,
    },
    /// 段 9 — 名指しがカーソルと一致しない。
    SkipMustNameCursor {
        /// 名指しされたステージ。
        named: StageSlug,
        /// 実際のカーソル。
        current: StageSlug,
    },
    /// 段 9 — checkbox が受理集合の外。
    SkipPrecondition {
        /// 対象ステージ。
        stage: StageSlug,
        /// そのステージの実測 checkbox。
        actual: CheckboxState,
    },
    /// 段 10 — 非ゲートの initialization ステージが gate 系の結末を名乗った。
    UngatedStage {
        /// 対象ステージ。
        stage: StageSlug,
        /// 報告された結末。
        verdict: Verdict,
    },
    /// 段 10 — gate 系の checkbox 前提違反 (`awaiting-approval` / `rejected` / `revised`)。
    GatePrecondition {
        /// 対象ステージ。
        stage: StageSlug,
        /// 報告された結末 (前提集合はこれで変わる)。
        verdict: Verdict,
        /// そのステージの実測 checkbox。
        actual: CheckboxState,
    },
    /// 段 10 — `rejected` に非空のフィードバックが無い。
    RejectRequiresFeedback {
        /// 対象ステージ。
        stage: StageSlug,
    },
    /// 段 13 — human presence が要るのに `--user-input` が空。
    HumanPresence {
        /// 対象ステージ。
        stage: StageSlug,
        /// 報告された結末。
        verdict: Verdict,
    },
    /// forward 表 — `[S]` / `[R]` は前進の完了ではない。
    ForwardCommitsCompletionsOnly {
        /// 対象ステージ。
        stage: StageSlug,
        /// そのステージの実測 checkbox。
        actual: CheckboxState,
    },
    /// forward 表 — `[ ]` はまだ走っていない。
    StillPending {
        /// 対象ステージ。
        stage: StageSlug,
    },
    /// forward 表 — ゲート未開放の `[-]` は明示 `--stage` を要する。
    InProgressRequiresExplicitStage {
        /// 対象ステージ。
        stage: StageSlug,
    },
}

impl fmt::Display for ReportRefusal {
    /// 材料だけを綴る (利用者向けの逐語文言は出す側の責務)。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReportRefusal::UnknownStage { named } => write!(f, "unknown stage: {named}"),
            ReportRefusal::RoutedVerdict { verdict } => {
                write!(f, "routed verdict: {verdict:?}")
            }
            ReportRefusal::SkipNotConditional { stage, execution } => write!(
                f,
                "skip not conditional: {} is {}",
                stage.as_str(),
                execution.as_str()
            ),
            ReportRefusal::SkipRequiresReason { stage } => {
                write!(f, "skip requires reason: {}", stage.as_str())
            }
            ReportRefusal::SkipMustNameCursor { named, current } => write!(
                f,
                "skip must name the cursor: named {}, current {}",
                named.as_str(),
                current.as_str()
            ),
            ReportRefusal::SkipPrecondition { stage, actual } => write!(
                f,
                "skip precondition: {} is {}",
                stage.as_str(),
                actual.spelling()
            ),
            ReportRefusal::UngatedStage { stage, verdict } => write!(
                f,
                "ungated stage: {} cannot report {verdict:?}",
                stage.as_str()
            ),
            ReportRefusal::GatePrecondition {
                stage,
                verdict,
                actual,
            } => write!(
                f,
                "gate precondition: {} is {} for {verdict:?}",
                stage.as_str(),
                actual.spelling()
            ),
            ReportRefusal::RejectRequiresFeedback { stage } => {
                write!(f, "reject requires feedback: {}", stage.as_str())
            }
            ReportRefusal::HumanPresence { stage, verdict } => write!(
                f,
                "human presence required: {} for {verdict:?}",
                stage.as_str()
            ),
            ReportRefusal::ForwardCommitsCompletionsOnly { stage, actual } => write!(
                f,
                "forward commits completions only: {} is {}",
                stage.as_str(),
                actual.spelling()
            ),
            ReportRefusal::StillPending { stage } => {
                write!(f, "still pending: {}", stage.as_str())
            }
            ReportRefusal::InProgressRequiresExplicitStage { stage } => {
                write!(
                    f,
                    "in-progress requires an explicit stage: {}",
                    stage.as_str()
                )
            }
        }
    }
}

impl std::error::Error for ReportRefusal {}

#[cfg(test)]
mod tests {
    use super::*;

    fn slug(value: &str) -> StageSlug {
        StageSlug::parse(value).expect("フィクスチャの slug は文法内")
    }

    #[test]
    fn every_refusal_renders_its_material() {
        let cases = [
            (
                ReportRefusal::UnknownStage {
                    named: "nope".to_string(),
                },
                "unknown stage: nope",
            ),
            (
                ReportRefusal::RoutedVerdict {
                    verdict: Verdict::Resume,
                },
                "routed verdict: Resume",
            ),
            (
                ReportRefusal::SkipNotConditional {
                    stage: slug("domain-design"),
                    execution: ExecutionKind::Always,
                },
                "skip not conditional: domain-design is ALWAYS",
            ),
            (
                ReportRefusal::SkipRequiresReason {
                    stage: slug("domain-design"),
                },
                "skip requires reason: domain-design",
            ),
            (
                ReportRefusal::SkipMustNameCursor {
                    named: slug("domain-design"),
                    current: slug("contract-design"),
                },
                "skip must name the cursor: named domain-design, current contract-design",
            ),
            (
                ReportRefusal::SkipPrecondition {
                    stage: slug("domain-design"),
                    actual: CheckboxState::Pending,
                },
                "skip precondition: domain-design is pending",
            ),
            (
                ReportRefusal::UngatedStage {
                    stage: slug("state-init"),
                    verdict: Verdict::Rejected,
                },
                "ungated stage: state-init cannot report Rejected",
            ),
            (
                ReportRefusal::GatePrecondition {
                    stage: slug("domain-design"),
                    verdict: Verdict::Revised,
                    actual: CheckboxState::InProgress,
                },
                "gate precondition: domain-design is in-progress for Revised",
            ),
            (
                ReportRefusal::RejectRequiresFeedback {
                    stage: slug("domain-design"),
                },
                "reject requires feedback: domain-design",
            ),
            (
                ReportRefusal::HumanPresence {
                    stage: slug("domain-design"),
                    verdict: Verdict::Forward,
                },
                "human presence required: domain-design for Forward",
            ),
            (
                ReportRefusal::ForwardCommitsCompletionsOnly {
                    stage: slug("domain-design"),
                    actual: CheckboxState::Revising,
                },
                "forward commits completions only: domain-design is revising",
            ),
            (
                ReportRefusal::StillPending {
                    stage: slug("contract-design"),
                },
                "still pending: contract-design",
            ),
            (
                ReportRefusal::InProgressRequiresExplicitStage {
                    stage: slug("domain-design"),
                },
                "in-progress requires an explicit stage: domain-design",
            ),
        ];
        assert_eq!(cases.len(), 13, "拒否は 13 形である");
        for (refusal, rendered) in cases {
            assert_eq!(refusal.to_string(), rendered);
        }
    }

    #[test]
    fn a_refusal_owns_its_material_and_ends_the_chain() {
        let refusal = ReportRefusal::StillPending {
            stage: slug("contract-design"),
        };
        assert!(std::error::Error::source(&refusal).is_none());
    }
}
