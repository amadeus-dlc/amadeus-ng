//! 計画承認の試行を区切る事実の種類。
/// 同種の境界の通し番号が、同じ秒に起きた再試行を区別する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunBoundaryKind {
    /// 作業の開始。
    WorkflowStarted,
    /// ステージの開始。
    StageStarted,
    /// 明示的な移動。
    StageJumped,
    /// 人間による差戻し。
    GateRejected,
}
impl RunBoundaryKind {
    /// 公開契約の綴り。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WorkflowStarted => "WORKFLOW_STARTED",
            Self::StageStarted => "STAGE_STARTED",
            Self::StageJumped => "STAGE_JUMPED",
            Self::GateRejected => "GATE_REJECTED",
        }
    }
}
