//! `TokenVersion` — continue_token の版語彙 (`v`)。
//!
//! 版は算術の対象ではない — 語彙は「現行か」「互換か」だけである (fail-closed: 非互換の
//! 版を運ぶトークンは codec が拒否する)。

/// トークンの版。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenVersion(u32);

impl TokenVersion {
    /// 現行の版 (v1)。
    pub const CURRENT: TokenVersion = TokenVersion(1);

    /// ワイヤ生値から復元する (互換判定は `is_supported`)。
    #[must_use]
    pub const fn from_raw(raw: u32) -> TokenVersion {
        TokenVersion(raw)
    }

    /// この版のトークンを本エンジンが読めるか。
    #[must_use]
    pub const fn is_supported(self) -> bool {
        self.0 == TokenVersion::CURRENT.0
    }

    /// ワイヤ・表示用の生値。
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_current_version_is_supported() {
        assert!(TokenVersion::CURRENT.is_supported());
        assert_eq!(TokenVersion::CURRENT.as_u32(), 1);
        assert!(!TokenVersion::from_raw(2).is_supported());
        assert!(!TokenVersion::from_raw(0).is_supported());
    }
}
