//! `ScopeTokenError` — 読み取り範囲の綴りを鋳造できない理由。
use std::fmt;

/// [`super::ScopeToken`] の構築が拒否する理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeTokenError {
    /// 空の綴り (upstream は `v.length > 0` の候補だけを集める)。
    Empty,
}
impl fmt::Display for ScopeTokenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScopeTokenError::Empty => formatter.write_str("scope token is empty"),
        }
    }
}
impl std::error::Error for ScopeTokenError {}
