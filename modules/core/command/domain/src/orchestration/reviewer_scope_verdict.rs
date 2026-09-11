//! `ReviewerScopeVerdict` — 読み取り範囲の判定の結末。
use super::ReviewerScopeBlock;

/// 呼出し 1 件に対する読み取り範囲の結末 (許可か拒否かの閉集合)。
///
/// 許可に材料は無い — 上流も許可の監査行を持たない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewerScopeVerdict {
    /// 範囲の内側なので通す。
    Allowed,
    /// 兄弟 Unit へ届くので拒否する。
    Blocked(ReviewerScopeBlock),
}
impl ReviewerScopeVerdict {
    /// 拒否したか。
    #[must_use]
    pub const fn is_blocked(&self) -> bool {
        matches!(self, ReviewerScopeVerdict::Blocked(_))
    }
}
