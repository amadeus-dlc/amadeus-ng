//! `RuleContent` — `load-steering` の `rules_content[]` の 1 要素。
//!
//! ルールの**テキスト**が必須 steering で、パスはルーティングメタデータである
//! (02 §10「No rule is downgraded to a discretionary path read」)。

/// `load-steering` の `rules_content[]` の 1 要素 — ルールの**テキスト**が必須 steering で、
/// パスはルーティングメタデータである (02 §10 「No rule is downgraded to a discretionary
/// path read」)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleContent {
    path: String,
    text: String,
}

impl RuleContent {
    /// パスとテキストを束ねる。
    #[must_use]
    pub const fn new(path: String, text: String) -> RuleContent {
        RuleContent { path, text }
    }

    /// ルールファイルのパス (ルーティングメタデータ)。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// ルールのテキスト (必須 steering)。
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}
