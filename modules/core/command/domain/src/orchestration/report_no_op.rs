//! `ReportNoOp` — 何もコミットしない成功 3 形 (ピン `:5700` / `:5828-5859`)。

use crate::workflow_definition::StageSlug;

/// 遷移を 1 つも起こさずに終わる報告 (**失敗ではない**)。
///
/// upstream はこの 3 形をそれぞれ `print` / `done` / `done` で返す。区別が要るのは逐語文言が
/// 違うからであり、判断そのものは集約が済ませている。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportNoOp {
    /// 既に開いているゲートへの `awaiting-approval` 再報告 (`print`)。
    AlreadyAwaiting {
        /// 既に `[?]` だったステージ。
        stage: StageSlug,
    },
    /// カーソルが通過済みの completed への再報告 (`done` — BR1.9 の冪等)。
    AlreadyCompletedMovedOn {
        /// 報告されたステージ。
        stage: StageSlug,
        /// ワークフローが既に移っている現在地。
        current: StageSlug,
    },
    /// 完了済みワークフローの最終ステージへの再報告 (`done`)。
    WorkflowAlreadyCompleted {
        /// 報告された最終ステージ。
        stage: StageSlug,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slug(value: &str) -> StageSlug {
        StageSlug::parse(value).expect("フィクスチャの slug は文法内")
    }

    #[test]
    fn the_moved_on_arm_carries_both_the_reported_stage_and_the_current_one() {
        let no_op = ReportNoOp::AlreadyCompletedMovedOn {
            stage: slug("domain-design"),
            current: slug("contract-design"),
        };
        assert_eq!(
            no_op,
            ReportNoOp::AlreadyCompletedMovedOn {
                stage: slug("domain-design"),
                current: slug("contract-design"),
            }
        );
    }

    #[test]
    fn the_three_arms_are_distinct() {
        assert_ne!(
            ReportNoOp::AlreadyAwaiting {
                stage: slug("domain-design")
            },
            ReportNoOp::WorkflowAlreadyCompleted {
                stage: slug("domain-design")
            }
        );
    }
}
