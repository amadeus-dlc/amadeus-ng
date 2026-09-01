//! `CloneId` — `/^[a-z0-9]{1,32}$/`。machine-local (gitignore) が本質: コミットで旅をしない
//! ことが clone 間の相異性を保証し、並行監査追記の git 衝突を除去する (upstream 03 §3.3)。
//! 鋳造 (12 hex mint + 再読収束) は I/O を伴うためユースケース層の責務 — ここは検証のみ。

use std::fmt;

use super::clone_id_error::CloneIdError;

/// パース済みの clone 識別子 (Always Valid — `/^[a-z0-9]{1,32}$/` を満たさない値はこの型に
/// 存在しない)。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CloneId(String);

impl CloneId {
    /// # Errors
    ///
    /// 空・33 文字以上・`[a-z0-9]` 以外を拒否する。
    pub fn parse(s: &str) -> Result<CloneId, CloneIdError> {
        if s.is_empty() {
            return Err(CloneIdError::Empty);
        }
        if s.len() > 32 {
            return Err(CloneIdError::TooLong(s.len()));
        }
        for c in s.chars() {
            if !(c.is_ascii_lowercase() || c.is_ascii_digit()) {
                return Err(CloneIdError::InvalidChar(c));
            }
        }
        Ok(CloneId(s.to_string()))
    }

    /// 検証済みトークン — `ShardName` の接尾辞になる形そのもの。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CloneId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_minted_12_hex_tokens_and_up_to_32_chars() {
        assert!(CloneId::parse("a1b2c3d4e5f6").is_ok());
        assert!(CloneId::parse(&"a".repeat(32)).is_ok());
    }

    /// 表示は検証済みトークンそのもの — 監査シャード名 `<host>-<cloneId>.md` の接尾辞は
    /// この綴りで組まれるので、`Display` と `as_str` がずれるとシャードが割れる。
    #[test]
    fn the_rendering_is_the_token_itself() {
        let id = CloneId::parse("8fc90228c64e").expect("固定のトークン");
        assert_eq!(id.to_string(), id.as_str());
        assert_eq!(id.to_string(), "8fc90228c64e");
    }

    #[test]
    fn rejects_empty_uppercase_and_overlong_tokens() {
        assert!(CloneId::parse("").is_err());
        assert!(CloneId::parse("A1B2").is_err());
        assert!(CloneId::parse("a-b").is_err());
        assert!(CloneId::parse(&"a".repeat(33)).is_err());
    }
}
