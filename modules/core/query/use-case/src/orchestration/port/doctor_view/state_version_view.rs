//! 状態版の分類結果の写し — 分類は U2 の分類器が行い、ここは答えを運ぶだけ。

use super::StateVersionKindView;

/// 分類と、読めた版の生トークン (`Past` / `Future` のときだけ)、本家分類の message
/// (`Ok` 以外のときだけ — 逐語は出す側の wording が組み、ここは運ぶだけ)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateVersionView {
    kind: StateVersionKindView,
    version: Option<String>,
    message: Option<String>,
}

impl StateVersionView {
    /// 分類結果を束ねる。
    #[must_use]
    pub const fn new(
        kind: StateVersionKindView,
        version: Option<String>,
        message: Option<String>,
    ) -> Self {
        Self {
            kind,
            version,
            message,
        }
    }

    /// 4 分類。
    #[must_use]
    pub const fn kind(&self) -> StateVersionKindView {
        self.kind
    }

    /// 読めた版の綴り。
    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// 本家分類の message (`Ok` では `None`)。
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}
