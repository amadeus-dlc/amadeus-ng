//! `UnknownReviewClass` — `ReviewClass::parse` の拒否経路が持ち帰る生値。

/// 閉集合外の値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownReviewClass(String);

impl UnknownReviewClass {
    /// 拒否された生値をそのまま包む (トリム・小文字化などの正規化はしない)。
    #[must_use]
    pub fn new(value: impl Into<String>) -> UnknownReviewClass {
        UnknownReviewClass(value.into())
    }

    /// 拒否された生値を逐語で持ち帰る (文言化は Presenter 側の責務)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
