//! 保護された計画承認の、実装開始前後の状態。
/// 保護された受領の状態。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanGenerationStatus {
    /// 計画の受領後、実装開始前。
    Approved,
    /// 実装開始の境界を通過した。
    Generation,
}
impl PlanGenerationStatus {
    /// 本家互換ファイルの状態語彙。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Generation => "generation",
        }
    }
}
