//! `SwitchAutonomyError` — `SwitchAutonomyUseCase` の失敗。

use std::fmt;

use core_command_domain::orchestration::{CommandError, IntentExecutionId, IntentId};

use super::port::RepositoryError;

/// [`super::SwitchAutonomyUseCase`] の失敗（材料のみ — 逐語文言は出す側が組む）。
// `Clone` / `PartialEq` は実装しない — `Corrupt` の `source` (原因連鎖) が比較・複製不能で
// ある ([`super::PromotePracticesError`] と同じ理由)。テストは `matches!` で判定する。
#[derive(Debug)]
pub enum SwitchAutonomyError {
    /// 実行の再構成・永続化の失敗（ポートからそのまま伝播）。
    Repository(RepositoryError<IntentExecutionId>),
    /// intent の取得の失敗（ポートからそのまま伝播）。
    IntentRepository(RepositoryError<IntentId>),
    /// 集約がコマンドを拒否した（そのまま伝播 — 昇格の presence ガードもここに来る）。
    Command(CommandError),
}

impl fmt::Display for SwitchAutonomyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SwitchAutonomyError::Repository(error) => write!(f, "repository: {error}"),
            SwitchAutonomyError::IntentRepository(error) => {
                write!(f, "intent repository: {error}")
            }
            SwitchAutonomyError::Command(error) => write!(f, "command: {error}"),
        }
    }
}

impl std::error::Error for SwitchAutonomyError {
    /// 内包した失敗へ連鎖する（`coding-rules/error-handling.md` — 封筒は連鎖を切らない）。
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SwitchAutonomyError::Repository(error) => Some(error),
            SwitchAutonomyError::IntentRepository(error) => Some(error),
            SwitchAutonomyError::Command(error) => Some(error),
        }
    }
}

impl From<RepositoryError<IntentExecutionId>> for SwitchAutonomyError {
    fn from(error: RepositoryError<IntentExecutionId>) -> SwitchAutonomyError {
        SwitchAutonomyError::Repository(error)
    }
}

impl From<RepositoryError<IntentId>> for SwitchAutonomyError {
    fn from(error: RepositoryError<IntentId>) -> SwitchAutonomyError {
        SwitchAutonomyError::IntentRepository(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_port_failures_keep_their_own_face_and_chain() {
        let execution_id =
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("UUIDv7");
        let error = SwitchAutonomyError::from(RepositoryError::NotFound {
            id: execution_id.clone(),
        });
        assert_eq!(
            error.to_string(),
            format!("repository: not found: {execution_id}")
        );
        assert!(std::error::Error::source(&error).is_some());

        let intent_id = IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("UUIDv7");
        let error = SwitchAutonomyError::from(RepositoryError::NotFound {
            id: intent_id.clone(),
        });
        assert_eq!(
            error.to_string(),
            format!("intent repository: not found: {intent_id}")
        );
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn a_refused_command_chains_to_the_guard() {
        let error = SwitchAutonomyError::Command(CommandError::HumanPresenceRequired);
        assert_eq!(error.to_string(), "command: human presence required");
        assert!(std::error::Error::source(&error).is_some());
    }
}
