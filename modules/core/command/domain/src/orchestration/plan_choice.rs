//! 計画承認で提示する2つの意味上の選択。
/// 表記の別名は提示側の照合規則を通してからこの値へ写す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanChoice {
    /// 提示した計画を承認する。
    ApprovePlan,
    /// 計画の修正を求める。
    RequestChanges,
}
impl PlanChoice {
    /// 正規の公開文言。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ApprovePlan => "Approve Plan",
            Self::RequestChanges => "Request Changes",
        }
    }
}
