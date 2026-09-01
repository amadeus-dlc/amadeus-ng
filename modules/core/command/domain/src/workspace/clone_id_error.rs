//! `CloneIdError` — `CloneId::parse` の拒否理由。

/// `parse` の拒否理由。正規化 (小文字化・切り詰め) は一切しない — 受理か拒否のみ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloneIdError {
    /// 空文字列。
    Empty,
    /// 32 文字上限の超過 (値は拒否された文字列のバイト長)。
    TooLong(usize),
    /// `[a-z0-9]` 以外 — 大文字も拒否対象 (値は走査順に最初に見つかった 1 文字)。
    InvalidChar(char),
}
