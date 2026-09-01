//! `UnknownVerdict` — `Verdict::parse` の拒否経路が持ち帰る生値。

/// 受理 10 語以外の生値 — `parse` の拒否経路 (未知語を既定 verdict へ丸めない)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownVerdict(String);

impl UnknownVerdict {
    /// 拒否された生値をそのまま包む (トリム・大小文字の正規化はしない)。
    #[must_use]
    pub fn new(value: impl Into<String>) -> UnknownVerdict {
        UnknownVerdict(value.into())
    }

    /// 拒否された生値を逐語で持ち帰る (文言化は Presenter 側の責務)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
