//! `state_writers::with_field_or_insert` の拒否。

/// `with_field_or_insert` の拒否 — 挿入先の `## Heading` セクションが state ファイルに存在しない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadingNotFound(String);

impl HeadingNotFound {
    /// 見つからなかった見出し名 (`## ` を含まない裸の名前) から構成する。
    #[must_use]
    pub fn new(heading: impl Into<String>) -> HeadingNotFound {
        HeadingNotFound(heading.into())
    }

    /// 見つからなかった見出し名を逐語で持ち帰る (文言化は Presenter 側の責務)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
