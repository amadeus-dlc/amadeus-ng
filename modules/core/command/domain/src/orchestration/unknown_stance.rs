//! `UnknownStance` — `SkeletonStance::parse` の拒否経路が持ち帰る生値。

/// 3 値以外の生値 — `parse` の拒否経路 (エンジンは未知 stance を既定へ丸めない)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownStance(String);

impl UnknownStance {
    /// 拒否された生値をそのまま包む (トリム・大小文字の正規化はしない)。
    #[must_use]
    pub fn new(value: impl Into<String>) -> UnknownStance {
        UnknownStance(value.into())
    }

    /// 拒否された生値を逐語で持ち帰る (文言化は Presenter 側の責務)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
