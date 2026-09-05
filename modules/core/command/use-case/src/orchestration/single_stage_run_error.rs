//! `SingleStageRunError` — `RecordSingleStageRunUseCase` の失敗。

use std::fmt;

use core_command_domain::orchestration::{CommandError, IntentExecutionId, IntentId};
use core_command_domain::workflow_definition::StageSlug;

use super::port::RepositoryError;

/// [`super::RecordSingleStageRunUseCase`] の失敗（材料のみ — 逐語文言は出す側が組む）。
///
/// `UnknownStage` だけがユースケース自身の失敗である — 名指しされた slug を計画の位置へ
/// 解決するのはユースケースの仕事であり、解決できなければ集約のコマンドは呼べない。
/// 残る 3 変種はポートと集約の失敗をそのまま運ぶ封筒である。
// `Clone` / `PartialEq` は実装しない — `Corrupt` の `source` (原因連鎖) が比較・複製不能で
// ある (裁定 6 で受容済み)。テストは `matches!` で判定する。
#[derive(Debug)]
pub enum SingleStageRunError {
    /// 実行の再構成・永続化の失敗（ポートからそのまま伝播）。
    Repository(RepositoryError<IntentExecutionId>),
    /// intent の取得の失敗（ポートからそのまま伝播）。
    IntentRepository(RepositoryError<IntentId>),
    /// 名指しされたステージが**この実行の計画に無い**。
    UnknownStage {
        /// 名指しされた slug（逐語文言の材料）。
        slug: StageSlug,
    },
    /// 集約がコマンドを拒否した（そのまま伝播 — initialization ステージ・取り違え）。
    Command {
        /// 拒否されたステージ。
        stage: StageSlug,
        /// 集約の拒否。
        error: CommandError,
    },
}

impl fmt::Display for SingleStageRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SingleStageRunError::Repository(error) => write!(f, "repository: {error}"),
            SingleStageRunError::IntentRepository(error) => {
                write!(f, "intent repository: {error}")
            }
            SingleStageRunError::UnknownStage { slug } => {
                write!(f, "unknown stage: {slug}")
            }
            SingleStageRunError::Command { stage, error } => {
                write!(f, "command for {stage}: {error}")
            }
        }
    }
}

impl std::error::Error for SingleStageRunError {
    /// 内包した失敗へ連鎖する。
    ///
    /// **封筒は連鎖を切ってはならない**（`coding-rules/error-handling.md`）。
    /// `UnknownStage` はこの型自身の失敗なので連鎖先を持たない。
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SingleStageRunError::Repository(error) => Some(error),
            SingleStageRunError::IntentRepository(error) => Some(error),
            SingleStageRunError::UnknownStage { .. } => None,
            SingleStageRunError::Command { error, .. } => Some(error),
        }
    }
}

impl From<RepositoryError<IntentExecutionId>> for SingleStageRunError {
    fn from(error: RepositoryError<IntentExecutionId>) -> SingleStageRunError {
        SingleStageRunError::Repository(error)
    }
}

impl From<RepositoryError<IntentId>> for SingleStageRunError {
    fn from(error: RepositoryError<IntentId>) -> SingleStageRunError {
        SingleStageRunError::IntentRepository(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slug() -> StageSlug {
        StageSlug::parse("functional-design").expect("フィクスチャの slug")
    }

    #[test]
    fn a_repository_failure_is_carried_verbatim_and_stays_on_the_chain() {
        let execution_id =
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("UUIDv7");
        let error = SingleStageRunError::from(RepositoryError::NotFound {
            id: execution_id.clone(),
        });
        assert_eq!(
            error.to_string(),
            format!("repository: not found: {execution_id}")
        );
        assert_eq!(
            std::error::Error::source(&error)
                .expect("ポートの失敗へ連鎖する")
                .to_string(),
            format!("not found: {execution_id}")
        );
    }

    #[test]
    fn the_intent_port_failure_keeps_its_own_face() {
        let intent_id = IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("UUIDv7");
        let error = SingleStageRunError::from(RepositoryError::NotFound {
            id: intent_id.clone(),
        });
        assert!(matches!(
            &error,
            SingleStageRunError::IntentRepository(RepositoryError::NotFound { id }) if *id == intent_id
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
    fn an_unknown_stage_is_this_use_cases_own_failure() {
        let error = SingleStageRunError::UnknownStage { slug: slug() };
        assert_eq!(error.to_string(), "unknown stage: functional-design");
        assert!(
            std::error::Error::source(&error).is_none(),
            "自分の失敗なので連鎖先は無い"
        );
    }

    #[test]
    fn a_refused_command_carries_the_stage_and_stays_on_the_chain() {
        let error = SingleStageRunError::Command {
            stage: slug(),
            error: CommandError::IntentMismatch,
        };
        assert!(
            error
                .to_string()
                .starts_with("command for functional-design: ")
        );
        assert_eq!(
            std::error::Error::source(&error)
                .expect("集約の拒否へ連鎖する")
                .to_string(),
            CommandError::IntentMismatch.to_string()
        );
    }
}
