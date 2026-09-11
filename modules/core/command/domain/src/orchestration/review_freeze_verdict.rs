//! `ReviewFreezeVerdict` — 書込み前の凍結判定の結末。
use super::ReviewFreezeBlock;
/// 終端の受領証とゲートの間の書込みを通すか拒否するか。
///
/// upstream の `FreezeVerdict`（`block` 真偽と 3 つの省略可能フィールド）を、拒否のときだけ
/// 材料が在る閉集合として持つ。許可に材料は無い。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewFreezeVerdict {
    /// 通す。凍結の対象ではない。
    Allowed,
    /// 拒否する。監査と文言の材料を運ぶ。
    Blocked(ReviewFreezeBlock),
}
impl ReviewFreezeVerdict {
    /// 拒否か。
    #[must_use]
    pub const fn is_blocked(&self) -> bool {
        matches!(self, ReviewFreezeVerdict::Blocked(_))
    }
    /// 拒否の材料 (許可なら `None`)。
    #[must_use]
    pub const fn block(&self) -> Option<&ReviewFreezeBlock> {
        match self {
            ReviewFreezeVerdict::Blocked(block) => Some(block),
            ReviewFreezeVerdict::Allowed => None,
        }
    }
}
