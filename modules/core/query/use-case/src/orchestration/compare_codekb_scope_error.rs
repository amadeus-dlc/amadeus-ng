//! 走査範囲の突合が成立しなかった — 材料だけを運ぶ拒否。

use core::fmt;
use std::error::Error;

use crate::orchestration::ReadModelReadError;

/// 走査範囲を突き合わせられなかった。
///
/// **判定 (`CodekbScopeDiffView`) と拒否はここで分かれる。** ストアが無い・ストアの範囲が
/// 読めないのは判定であり exit 0 で返るが、名指された突合相手が無いのは呼び出しそのものが
/// 成立していないので拒否である (upstream もそこだけ `die()` する)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareCodekbScopeError {
    /// リードモデルを引けなかった。
    Unreadable(ReadModelReadError),
    /// 名指された突合相手が無い。
    IncomingMissing,
}

impl fmt::Display for CompareCodekbScopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompareCodekbScopeError::Unreadable(cause) => write!(f, "{cause}"),
            CompareCodekbScopeError::IncomingMissing => write!(f, "incoming scope file not found"),
        }
    }
}

impl Error for CompareCodekbScopeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CompareCodekbScopeError::Unreadable(cause) => Some(cause),
            CompareCodekbScopeError::IncomingMissing => None,
        }
    }
}
