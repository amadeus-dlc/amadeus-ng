//! `SkeletonStanceError` — `RecordSkeletonStanceUseCase` の失敗。

use std::fmt;

use core_command_domain::orchestration::{CommandError, IntentExecutionId, IntentId};
use core_command_domain::workflow_definition::StageSlug;

use super::port::RepositoryError;

/// [`super::RecordSkeletonStanceUseCase`] の失敗（材料のみ — 逐語文言は出す側が組む）。
///
/// 3 変種すべてが**そのまま伝播させるための封筒**である。stance の記録はステージを引数に
/// 取らず常に「そのときのカーソル」に作用するので、[`SingleStageRunError`] の
/// `UnknownStage` にあたるユースケース自身の失敗を持たない。
///
/// `Command` が現在地の slug と scope を運ぶのは、拒否の逐語
/// （`Current stage "<slug>" is not the skeleton-gate stage for scope "<scope>"`）の**材料**が
/// その 2 つだからである — 文言を組むのは出す側である。
///
/// [`SingleStageRunError`]: super::SingleStageRunError
// `Clone` / `PartialEq` は実装しない — `Corrupt` の `source` (原因連鎖) が比較・複製不能で
// ある (裁定 6 で受容済み)。テストは `matches!` で判定する。
#[derive(Debug)]
pub enum SkeletonStanceError {
    /// 実行の再構成・永続化の失敗（ポートからそのまま伝播）。
    Repository(RepositoryError<IntentExecutionId>),
    /// intent の取得の失敗（ポートからそのまま伝播）。
    IntentRepository(RepositoryError<IntentId>),
    /// 集約がコマンドを拒否した（そのまま伝播）。
    Command {
        /// 拒否された時点の現在地（添字帳が空でない限り必ず在る — 完全コンストラクタが
        /// カーソルの範囲を保証している。`Option` なのは型が知らないだけである）。
        stage: Option<StageSlug>,
        /// その実行が選んでいる scope。
        scope: String,
        /// 集約の拒否。
        error: CommandError,
    },
}

impl fmt::Display for SkeletonStanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SkeletonStanceError::Repository(error) => write!(f, "repository: {error}"),
            SkeletonStanceError::IntentRepository(error) => write!(f, "intent repository: {error}"),
            SkeletonStanceError::Command {
                stage,
                scope,
                error,
            } => write!(
                f,
                "command for {} in scope {scope}: {error}",
                stage.as_ref().map_or("(unknown stage)", StageSlug::as_str)
            ),
        }
    }
}

impl std::error::Error for SkeletonStanceError {
    /// 内包した失敗へ連鎖する（`coding-rules/error-handling.md` — 封筒は連鎖を切らない）。
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SkeletonStanceError::Repository(error) => Some(error),
            SkeletonStanceError::IntentRepository(error) => Some(error),
            SkeletonStanceError::Command { error, .. } => Some(error),
        }
    }
}

impl From<RepositoryError<IntentExecutionId>> for SkeletonStanceError {
    fn from(error: RepositoryError<IntentExecutionId>) -> SkeletonStanceError {
        SkeletonStanceError::Repository(error)
    }
}

impl From<RepositoryError<IntentId>> for SkeletonStanceError {
    fn from(error: RepositoryError<IntentId>) -> SkeletonStanceError {
        SkeletonStanceError::IntentRepository(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_repository_failure_is_carried_verbatim_and_stays_on_the_chain() {
        let execution_id =
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("UUIDv7");
        let error = SkeletonStanceError::from(RepositoryError::NotFound {
            id: execution_id.clone(),
        });
        assert_eq!(
            error.to_string(),
            format!("repository: not found: {execution_id}")
        );
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn the_intent_port_failure_keeps_its_own_face() {
        let intent_id = IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("UUIDv7");
        let error = SkeletonStanceError::from(RepositoryError::NotFound {
            id: intent_id.clone(),
        });
        assert!(matches!(
            &error,
            SkeletonStanceError::IntentRepository(RepositoryError::NotFound { id }) if *id == intent_id
        ));
        assert_eq!(
            error.to_string(),
            format!("intent repository: not found: {intent_id}")
        );
        assert_eq!(
            std::error::Error::source(&error)
                .expect("ポートの失敗へ連鎖する")
                .to_string(),
            format!("not found: {intent_id}")
        );
    }

    #[test]
    fn a_refused_command_carries_the_cursor_and_the_scope() {
        let stage = StageSlug::parse("intent-capture").expect("フィクスチャの slug");
        let error = SkeletonStanceError::Command {
            stage: Some(stage),
            scope: "classic".to_string(),
            error: CommandError::NotRunning,
        };
        assert!(
            error
                .to_string()
                .starts_with("command for intent-capture in scope classic: ")
        );
        assert_eq!(
            std::error::Error::source(&error)
                .expect("集約の拒否へ連鎖する")
                .to_string(),
            CommandError::NotRunning.to_string()
        );
    }
}
