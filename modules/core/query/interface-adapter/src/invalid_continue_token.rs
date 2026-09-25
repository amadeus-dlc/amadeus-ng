//! `InvalidContinueToken` — continue token の検証 ([`crate::verify_continue_token`]) が拒否した。

/// 無効なトークン (材料なし — 「無効」だけを約束する。fail-closed の逐語文言は呼出側の
/// wording が組む)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidContinueToken;

impl std::fmt::Display for InvalidContinueToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid continue token")
    }
}

impl std::error::Error for InvalidContinueToken {}
