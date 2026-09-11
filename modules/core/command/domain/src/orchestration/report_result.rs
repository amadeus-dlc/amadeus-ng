//! 報告を受理した結果。永続化や表示の形式は持たない。
use super::{ReportNoOp, ReportTransition, TransitionSteps};
use crate::workflow_definition::StageSlug;

/// 報告された事実に属する結果。現在状態から再判定しない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportResult {
    /// 報告によって段階を変更した。
    Committed {
        /// 適用対象。
        stage: StageSlug,
        /// 報告時のスコープ。
        scope: String,
        /// 適用順序。
        steps: TransitionSteps,
        /// 実際に起きた遷移。
        transition: ReportTransition,
    },
    /// 報告は受理したが段階を変更しなかった。
    NoOp {
        /// 報告時のスコープ。
        scope: String,
        /// 変更しなかった理由。
        no_op: ReportNoOp,
    },
}
