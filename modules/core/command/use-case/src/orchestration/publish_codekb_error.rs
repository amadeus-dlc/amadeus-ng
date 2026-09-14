//! `PublishCodekbError` — `PublishCodekbUseCase` の失敗。

use std::fmt;

use core_command_domain::workspace::{CodekbPublishRefusal, CodekbRepoId};

use super::port::RepositoryError;

/// [`super::PublishCodekbUseCase`] の失敗（材料のみ — 逐語文言は出す側が組む）。
///
/// **拒否と失敗を分けている**のが要点である。[`PublishCodekbError::Refused`] は
/// compare-and-swap が噛み合わなかったという**正常な観測**で、直し方が決まっている
/// (写しを取り直して再試行する)。[`PublishCodekbError::Repository`] は媒体そのものの失敗で、
/// 直し方が違う。
#[derive(Debug)]
pub enum PublishCodekbError {
    /// ストアの再構成・永続化の失敗（ポートからそのまま伝播）。
    Repository(RepositoryError<CodekbRepoId>),
    /// 集約が公開を拒んだ（そのまま伝播）。
    Refused(CodekbPublishRefusal),
}

impl fmt::Display for PublishCodekbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PublishCodekbError::Repository(error) => write!(f, "repository: {error}"),
            PublishCodekbError::Refused(refusal) => write!(f, "refused: {refusal}"),
        }
    }
}

impl std::error::Error for PublishCodekbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PublishCodekbError::Repository(error) => Some(error),
            PublishCodekbError::Refused(refusal) => Some(refusal),
        }
    }
}

impl From<RepositoryError<CodekbRepoId>> for PublishCodekbError {
    fn from(error: RepositoryError<CodekbRepoId>) -> PublishCodekbError {
        PublishCodekbError::Repository(error)
    }
}
