//! `ReviewLogError` — `RecordReviewUseCase` の失敗。

use std::fmt;

use core_command_domain::orchestration::{CommandError, IntentExecutionId, IntentId};
use core_command_domain::workflow_definition::{StageSlug, WorkflowDefinitionId};

use super::port::RepositoryError;

/// [`super::RecordReviewUseCase`] の失敗（材料のみ — 逐語文言は出す側が組む）。
///
/// 拒否の逐語は `--verdict` の有無で言い回しが分かれる（`Refusing REVIEW_COMPLETED …` /
/// `Refusing review retry …`）ので、ここは**どの拒否だったか**という材料だけを運ぶ。
// `Clone` / `PartialEq` は実装しない — `Corrupt` の `source` (原因連鎖) が比較・複製不能で
// ある (裁定 6 で受容済み)。テストは `matches!` で判定する。
#[derive(Debug)]
pub enum ReviewLogError {
    /// 実行の再構成・永続化の失敗（ポートからそのまま伝播）。
    Repository(RepositoryError<IntentExecutionId>),
    /// intent の取得の失敗（ポートからそのまま伝播）。
    IntentRepository(RepositoryError<IntentId>),
    /// 定義の再構成の失敗（ポートからそのまま伝播）。
    DefinitionRepository(RepositoryError<WorkflowDefinitionId>),
    /// **定義**がその slug を知らない。
    ///
    /// 集約の `UnknownStage`（実行の計画に無い）とは別の事実である — upstream は
    /// どちらも同じ文言で断るが、材料としては分ける。
    UnknownStage(StageSlug),
    /// intent が運ぶ `--review` の値が閉集合の外だった（壊れた歴史）。
    CorruptReviewOverride(String),
    /// 関連付けた定義と依頼が一致しない等、レビュー方針の解決拒否。
    ReviewPolicy(core_command_domain::orchestration::IntentReviewError),
    /// 集約がコマンドを拒否した（そのまま伝播）。
    Command {
        /// 拒否の対象ステージ。
        stage: StageSlug,
        /// 集約の拒否。
        error: CommandError,
    },
}

impl fmt::Display for ReviewLogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReviewLogError::Repository(error) => write!(f, "repository: {error}"),
            ReviewLogError::IntentRepository(error) => write!(f, "intent repository: {error}"),
            ReviewLogError::DefinitionRepository(error) => {
                write!(f, "workflow definition repository: {error}")
            }
            ReviewLogError::UnknownStage(stage) => {
                write!(f, "the definition has no stage {stage}")
            }
            ReviewLogError::CorruptReviewOverride(raw) => {
                write!(f, "corrupt review override: {raw}")
            }
            ReviewLogError::ReviewPolicy(error) => write!(f, "review policy: {error}"),
            ReviewLogError::Command { stage, error } => {
                write!(f, "command for {stage}: {error}")
            }
        }
    }
}

impl std::error::Error for ReviewLogError {
    /// 内包した失敗へ連鎖する（`coding-rules/error-handling.md` — 封筒は連鎖を切らない）。
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ReviewLogError::Repository(error) => Some(error),
            ReviewLogError::IntentRepository(error) => Some(error),
            ReviewLogError::DefinitionRepository(error) => Some(error),
            ReviewLogError::ReviewPolicy(error) => Some(error),
            ReviewLogError::Command { error, .. } => Some(error),
            // ユースケース自身の失敗 — 材料は自分の `Display` にある。
            ReviewLogError::UnknownStage(_) | ReviewLogError::CorruptReviewOverride(_) => None,
        }
    }
}

impl From<RepositoryError<IntentExecutionId>> for ReviewLogError {
    fn from(error: RepositoryError<IntentExecutionId>) -> ReviewLogError {
        ReviewLogError::Repository(error)
    }
}

impl From<RepositoryError<IntentId>> for ReviewLogError {
    fn from(error: RepositoryError<IntentId>) -> ReviewLogError {
        ReviewLogError::IntentRepository(error)
    }
}

impl From<RepositoryError<WorkflowDefinitionId>> for ReviewLogError {
    fn from(error: RepositoryError<WorkflowDefinitionId>) -> ReviewLogError {
        ReviewLogError::DefinitionRepository(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_repository_failure_is_carried_verbatim_and_stays_on_the_chain() {
        let execution_id =
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("UUIDv7");
        let error = ReviewLogError::from(RepositoryError::NotFound {
            id: execution_id.clone(),
        });
        assert_eq!(
            error.to_string(),
            format!("repository: not found: {execution_id}")
        );
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn the_definition_port_failure_keeps_its_own_face() {
        let definition_id = WorkflowDefinitionId::parse("claude").expect("系譜名");
        let error = ReviewLogError::from(RepositoryError::NotFound {
            id: definition_id.clone(),
        });
        assert!(matches!(
            &error,
            ReviewLogError::DefinitionRepository(RepositoryError::NotFound { id })
                if *id == definition_id
        ));
        assert_eq!(
            error.to_string(),
            format!("workflow definition repository: not found: {definition_id}")
        );
        assert!(std::error::Error::source(&error).is_some());
    }

    /// intent ポートの失敗も自分の顔を保ち、連鎖を切らない。
    #[test]
    fn the_intent_port_failure_keeps_its_own_face() {
        let intent_id = IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("UUIDv7");
        let error = ReviewLogError::from(RepositoryError::NotFound {
            id: intent_id.clone(),
        });
        assert_eq!(
            error.to_string(),
            format!("intent repository: not found: {intent_id}")
        );
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn a_refused_command_carries_the_stage_and_chains_to_the_guard() {
        let stage = StageSlug::parse("functional-design").expect("フィクスチャの slug");
        let error = ReviewLogError::Command {
            stage,
            error: CommandError::IntentMismatch,
        };
        assert!(
            error
                .to_string()
                .starts_with("command for functional-design: ")
        );
        assert!(std::error::Error::source(&error).is_some());
    }

    /// ユースケース自身の失敗は連鎖の末端である（材料は `Display` にある）。
    #[test]
    fn the_use_case_own_failures_end_the_chain() {
        let unknown =
            ReviewLogError::UnknownStage(StageSlug::parse("nowhere").expect("文法内の slug"));
        assert_eq!(unknown.to_string(), "the definition has no stage nowhere");
        assert!(std::error::Error::source(&unknown).is_none());

        let corrupt = ReviewLogError::CorruptReviewOverride("Adversarial".to_string());
        assert_eq!(corrupt.to_string(), "corrupt review override: Adversarial");
        assert!(std::error::Error::source(&corrupt).is_none());
    }
}
