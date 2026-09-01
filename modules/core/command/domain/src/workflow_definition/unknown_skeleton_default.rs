//! `UnknownSkeletonDefault` — `SkeletonDefault::parse` の拒否経路が持ち帰る生値。

/// 閉集合外の値 (upstream: `has invalid skeleton value "..." . Expected "on" or "off".`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownSkeletonDefault(String);

impl UnknownSkeletonDefault {
    /// 拒否された生値をそのまま包む。トリムも小文字化もしない。
    #[must_use]
    pub fn new(value: impl Into<String>) -> UnknownSkeletonDefault {
        UnknownSkeletonDefault(value.into())
    }

    /// 拒否された生値を逐語で持ち帰る (文言化は Presenter 側の責務)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
