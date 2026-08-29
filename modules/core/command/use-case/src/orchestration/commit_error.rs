//! `CommitError` — `CommitVerdictUseCase` の失敗。

use std::fmt;

use core_command_domain::orchestration::CommandError;
use core_command_domain::workflow_definition::StageSlug;

use core_command_domain::orchestration::{IntentExecutionId, IntentId};

use super::repository_error::RepositoryError;

/// `CommitVerdictUseCase` の失敗（材料のみ — 逐語文言は出す側が組む）。
///
/// 下の 2 変種は**そのまま伝播させるための封筒**である。ユースケースは集約やポートの拒否を
/// 握り潰さないし言い換えもしない。3 つ目だけがユースケース自身の失敗で、報告が名指しした
/// ステージが解決済み計画に無かったことを言う。
// `Clone` / `PartialEq` は実装しない — `Corrupt` の `source` (原因連鎖) が比較・複製不能で
// ある (裁定 6 で受容済み)。テストは `matches!` で判定する。
#[derive(Debug)]
pub enum CommitError {
    /// 実行の再構成・永続化の失敗（ポートからそのまま伝播）。
    Repository(RepositoryError<IntentExecutionId>),
    /// intent の取得の失敗（ポートからそのまま伝播）。
    IntentRepository(RepositoryError<IntentId>),
    /// 集約がコマンドを拒否した（そのまま伝播）。
    Command(CommandError),
    /// 報告が名指ししたステージが解決済み計画に無い。
    UnknownStage {
        /// 名指しされた slug。
        stage: StageSlug,
    },
}

impl fmt::Display for CommitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommitError::Repository(error) => write!(f, "repository: {error}"),
            CommitError::IntentRepository(error) => write!(f, "intent repository: {error}"),
            CommitError::Command(error) => write!(f, "command: {error}"),
            CommitError::UnknownStage { stage } => write!(f, "unknown stage: {}", stage.as_str()),
        }
    }
}

impl std::error::Error for CommitError {}

impl From<RepositoryError<IntentExecutionId>> for CommitError {
    fn from(error: RepositoryError<IntentExecutionId>) -> CommitError {
        CommitError::Repository(error)
    }
}

impl From<RepositoryError<IntentId>> for CommitError {
    fn from(error: RepositoryError<IntentId>) -> CommitError {
        CommitError::IntentRepository(error)
    }
}

impl From<CommandError> for CommitError {
    fn from(error: CommandError) -> CommitError {
        CommitError::Command(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_command_domain::orchestration::IntentExecutionId;

    fn stage() -> StageSlug {
        StageSlug::parse("practices-discovery").expect("フィクスチャの slug は文法内")
    }

    #[test]
    fn a_repository_failure_is_carried_verbatim() {
        let execution_id =
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("UUIDv7");
        let inner = RepositoryError::NotFound {
            id: execution_id.clone(),
        };
        let error = CommitError::from(inner);
        assert!(matches!(
            &error,
            CommitError::Repository(RepositoryError::NotFound { id }) if *id == execution_id
        ));
        assert_eq!(
            error.to_string(),
            format!("repository: not found: {execution_id}")
        );
    }

    #[test]
    fn a_refused_command_is_carried_verbatim() {
        let error = CommitError::from(CommandError::NotRunning);
        assert!(matches!(
            error,
            CommitError::Command(CommandError::NotRunning)
        ));
        assert!(error.to_string().starts_with("command: "));
    }

    #[test]
    fn an_unknown_stage_names_the_slug_it_could_not_resolve() {
        let error = CommitError::UnknownStage { stage: stage() };
        assert_eq!(error.to_string(), "unknown stage: practices-discovery");
    }

    #[test]
    fn the_failure_is_a_std_error() {
        let error: Box<dyn std::error::Error> =
            Box::new(CommitError::UnknownStage { stage: stage() });
        assert_eq!(error.to_string(), "unknown stage: practices-discovery");
    }

    #[test]
    fn failures_pattern_match_by_variant() {
        // `PartialEq` は持たない (裁定 6 — `Corrupt` の `source` が比較不能)。判定は
        // `matches!` で行う。
        let unknown = CommitError::UnknownStage { stage: stage() };
        assert!(matches!(&unknown, CommitError::UnknownStage { stage } if *stage == self::stage()));
        assert!(!matches!(&unknown, CommitError::Command(_)));
    }
}
