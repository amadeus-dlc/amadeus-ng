//! `ReportError` — `ReportUseCase` の失敗。

use std::fmt;

use core_command_domain::orchestration::CommandError;
use core_command_domain::workflow_definition::StageSlug;

use super::repository_error::RepositoryError;

/// `ReportUseCase` の失敗 (材料のみ — 逐語文言は出す側が組む)。
///
/// 下の 2 変種は**そのまま伝播させるための封筒**である。ユースケースは集約やポートの拒否を
/// 握り潰さないし言い換えもしない — 再試行の政策も持たない (`Conflict` も再試行しない。
/// ポート doc の C3 ③)。3 つ目だけがユースケース自身の失敗で、報告が名指ししたステージが
/// 解決済み計画に無かったことを言う。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportError {
    /// 再構成・永続化の失敗 (ポートからそのまま伝播)。
    Repository(RepositoryError),
    /// 集約がコマンドを拒否した (そのまま伝播)。
    Command(CommandError),
    /// 報告が名指ししたステージが解決済み計画に無い。
    UnknownStage {
        /// 名指しされた slug。
        stage: StageSlug,
    },
}

impl fmt::Display for ReportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReportError::Repository(error) => write!(f, "repository: {error}"),
            ReportError::Command(error) => write!(f, "command: {error}"),
            ReportError::UnknownStage { stage } => write!(f, "unknown stage: {}", stage.as_str()),
        }
    }
}

impl std::error::Error for ReportError {}

impl From<RepositoryError> for ReportError {
    fn from(error: RepositoryError) -> ReportError {
        ReportError::Repository(error)
    }
}

impl From<CommandError> for ReportError {
    fn from(error: CommandError) -> ReportError {
        ReportError::Command(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_command_domain::orchestration::IntentId;

    fn stage() -> StageSlug {
        StageSlug::parse("practices-discovery").expect("フィクスチャの slug は文法内")
    }

    #[test]
    fn a_repository_failure_is_carried_verbatim() {
        let intent_id = IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("UUIDv7");
        let inner = RepositoryError::NotFound {
            intent_id: intent_id.clone(),
        };
        let error = ReportError::from(inner.clone());
        assert_eq!(error, ReportError::Repository(inner));
        assert_eq!(
            error.to_string(),
            format!("repository: not found: {intent_id}")
        );
    }

    #[test]
    fn a_refused_command_is_carried_verbatim() {
        let error = ReportError::from(CommandError::NotRunning);
        assert_eq!(error, ReportError::Command(CommandError::NotRunning));
        assert!(error.to_string().starts_with("command: "));
    }

    #[test]
    fn an_unknown_stage_names_the_slug_it_could_not_resolve() {
        let error = ReportError::UnknownStage { stage: stage() };
        assert_eq!(error.to_string(), "unknown stage: practices-discovery");
    }

    #[test]
    fn the_failure_is_a_std_error() {
        let error: Box<dyn std::error::Error> =
            Box::new(ReportError::UnknownStage { stage: stage() });
        assert_eq!(error.to_string(), "unknown stage: practices-discovery");
    }

    #[test]
    fn failures_compare_by_value() {
        assert_eq!(
            ReportError::UnknownStage { stage: stage() },
            ReportError::UnknownStage { stage: stage() }
        );
        assert_ne!(
            ReportError::UnknownStage { stage: stage() },
            ReportError::Command(CommandError::NotRunning)
        );
    }
}
