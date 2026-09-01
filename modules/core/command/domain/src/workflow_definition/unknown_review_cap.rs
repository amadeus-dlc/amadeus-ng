//! `UnknownReviewCap` — `ReviewCapValue::parse` の拒否経路が持ち帰る生値。

/// 閉集合外の値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownReviewCap(String);

impl UnknownReviewCap {
    /// 拒否された生値をそのまま包む。空文字列も既定へ畳まずそのまま保つ。
    #[must_use]
    pub fn new(value: impl Into<String>) -> UnknownReviewCap {
        UnknownReviewCap(value.into())
    }

    /// 拒否された生値を逐語で持ち帰る (文言化は Presenter 側の責務)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
