//! `NounFamily` — 名詞トークンの族 (分岐 1b/1c/1d の先頭トークン意味論)。
//!
//! 先頭トークンだけが族を決め、残りのトークン列は [`NounToken`] が逐語で運ぶ。
//!
//! [`NounToken`]: super::NounToken

/// 名詞トークンの族 (分岐 1b/1c/1d — 先頭トークン意味論のみ)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NounFamily {
    /// `space` / `intent` (分岐 1b)。
    Workspace,
    /// `plugin` (分岐 1c)。
    Plugin,
    /// `knowledge` (分岐 1d)。
    Knowledge,
}
