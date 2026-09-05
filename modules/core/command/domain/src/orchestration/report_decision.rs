//! `ReportDecision` — [`IntentExecution::report_dispatch`] の答え (書込なし)。
//!
//! [`IntentExecution::report_dispatch`]: super::IntentExecution::report_dispatch

use super::report_no_op::ReportNoOp;
use super::transition_step::TransitionStep;
use crate::workflow_definition::StageSlug;

/// 報告に対して**何をコミットするか**の閉集合。
///
/// 判断は集約が済ませており、ユースケースはこの答えのとおりに集約コマンドを打つだけである
/// (`coding-rules/tell-dont-ask.md` — 判断は状態の所有者へ)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportDecision {
    /// 名指しのステージに遷移を打つ。
    ///
    /// # 対象は必ずカーソルである
    ///
    /// 作用対象は「明示 `--stage` かカーソル」だが、`Commit` に至る経路ではその 2 つが必ず
    /// 一致する — `[-]` / `[?]` はカーソルにしか立たず (不変条件 `at_most_one_active`)、
    /// `skipped` は名指しとカーソルの一致を受理条件に持つからである。したがって集約コマンドが
    /// 暗黙にカーソルへ作用しても対象はずれない。運ぶのが位置ではなく slug なのは、
    /// 呼出側が要るのが**逐語文言の材料**だからである。
    Commit {
        /// 作用対象のステージ。
        stage: StageSlug,
        /// upstream の `sequence` に対応する段の列 (1 段か、復旧の 2 段)。
        steps: Vec<TransitionStep>,
        /// 判断に使用した計画のスコープ。
        scope: String,
    },
    /// 何もコミットしない成功。
    NoOp {
        /// 状態を変更しない理由。
        no_op: ReportNoOp,
        /// 判断に使用した計画のスコープ。
        scope: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slug() -> StageSlug {
        StageSlug::parse("domain-design").expect("slug は文法内")
    }

    #[test]
    fn a_recovery_commit_names_two_steps_in_order() {
        let decision = ReportDecision::Commit {
            scope: "classic".into(),
            stage: slug(),
            steps: vec![TransitionStep::GateStartRecovered, TransitionStep::Approve],
        };
        assert_eq!(
            decision,
            ReportDecision::Commit {
                scope: "classic".into(),
                stage: slug(),
                steps: vec![TransitionStep::GateStartRecovered, TransitionStep::Approve],
            }
        );
    }

    #[test]
    fn a_no_op_is_not_a_commit() {
        assert_ne!(
            ReportDecision::NoOp {
                no_op: ReportNoOp::AlreadyAwaiting { stage: slug() },
                scope: "classic".into()
            },
            ReportDecision::Commit {
                scope: "classic".into(),
                stage: slug(),
                steps: Vec::new(),
            }
        );
    }
}
