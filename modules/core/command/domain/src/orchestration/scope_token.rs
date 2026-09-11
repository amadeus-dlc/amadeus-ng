//! `ScopeToken` — 読み取り範囲の判定にかける綴り 1 件。
use super::ScopeTokenError;

/// レビュアーの呼出しが名指した綴り 1 件 (経路・パターン・シェルの語)。
///
/// upstream は「the offending path or token」と呼び、拒否のときはこの綴りを**逐語で**
/// 監査の `Target` と拒否文言へ載せる。したがって**正規化しない** —
/// [`super::WriteTarget`] が構築時に `\` を `/` へ畳むのと対照的である。
/// reviewer-scope 側の upstream `toPosix` は POSIX ホストでは恒等写像なので、
/// `C:\r\construction\u2\a.md` は区切りを 1 つも持たない 1 語として扱われ、
/// **越境として検出されない**。畳むと upstream が通す呼出しを拒否してしまう。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScopeToken(String);
impl ScopeToken {
    /// 生の綴りを検査して綴りにする (**この型の唯一の構築経路**)。
    ///
    /// # Errors
    /// 空の綴りの場合 (upstream は空の候補を集めない)。
    pub fn parse(raw: &str) -> Result<ScopeToken, ScopeTokenError> {
        if raw.is_empty() {
            return Err(ScopeTokenError::Empty);
        }
        Ok(ScopeToken(raw.to_string()))
    }
    /// 監査項目・拒否文言へ渡す逐語の綴り。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::{ScopeToken, ScopeTokenError};
    #[test]
    fn a_spelling_is_kept_verbatim_including_backslashes() {
        let token = ScopeToken::parse(r"C:\r\construction\u2\a.md").unwrap();
        assert_eq!(token.as_str(), r"C:\r\construction\u2\a.md");
    }
    #[test]
    fn an_empty_spelling_names_nothing() {
        assert_eq!(ScopeToken::parse(""), Err(ScopeTokenError::Empty));
    }
    #[test]
    fn whitespace_is_a_spelling_because_upstream_only_drops_the_empty_string() {
        assert!(ScopeToken::parse(" ").is_ok());
    }
}
