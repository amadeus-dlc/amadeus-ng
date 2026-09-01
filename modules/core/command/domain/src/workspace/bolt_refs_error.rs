//! `BoltRefsError` — `BoltRefs` の追記が拒否する形。

/// `BoltRefs` の拒否理由 (重複・不在を無言 no-op にしないための閉集合)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoltRefsError {
    /// ブラケットで始まらない・閉じない等の不正形。
    Malformed(String),
    /// append 対象が既に存在する。
    DuplicateSlug(String),
    /// remove 対象が存在しない。
    MissingSlug(String),
}
