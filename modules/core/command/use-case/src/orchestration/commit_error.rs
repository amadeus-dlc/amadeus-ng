//! `CommitError` — `CommitVerdictUseCase` の失敗。

use std::fmt;

use core_command_domain::orchestration::{CommandError, ReportRefusal, TransitionStep};
use core_command_domain::workflow_definition::{StageSlug, WorkflowDefinitionId};

use core_command_domain::orchestration::{IntentExecutionId, IntentId};

use super::port::RepositoryError;

/// `CommitVerdictUseCase` の失敗（材料のみ — 逐語文言は出す側が組む）。
///
/// 最初の 2 変種は**そのまま伝播させるための封筒**である。ユースケースはポートの拒否を
/// 握り潰さないし言い換えもしない。`Refused` は集約の判断 (`report_dispatch`) が
/// 「どの遷移も打たない」と決めた結果、`Transition` はその判断が名指しした段を実際に
/// 打って集約に拒まれた結果、`UnwiredTransition` は名指しされた段に対応する集約コマンドが
/// この build に無い場合である。
// `Clone` / `PartialEq` は実装しない — `Corrupt` の `source` (原因連鎖) が比較・複製不能で
// ある (裁定 6 で受容済み)。テストは `matches!` で判定する。
#[derive(Debug)]
pub enum CommitError {
    /// 実行の再構成・永続化の失敗（ポートからそのまま伝播）。
    Repository(RepositoryError<IntentExecutionId>),
    /// intent の取得の失敗（ポートからそのまま伝播）。
    IntentRepository(RepositoryError<IntentId>),
    /// 集約の判断が報告を受理しなかった（そのまま伝播）。
    Refused(ReportRefusal),
    /// 判断が名指しした段を打ったが、集約がそのコマンドを拒否した。
    ///
    /// upstream の「spawn 先が非ゼロ終了した」に対応する — 逐語
    /// `Transition rejected by aidlc-state.ts <sub> for "<slug>"` を組むために、どの段の
    /// どのステージだったかを添える。
    Transition {
        /// 打とうとした段。
        step: TransitionStep,
        /// その段の対象ステージ。
        stage: StageSlug,
        /// 集約の拒否（そのまま伝播）。
        error: CommandError,
    },
    /// 名指しされた段に対応する集約コマンドが**この build に無い**。
    ///
    /// `advance` / `complete-workflow` の 2 段が該当する — 非ゲート完了のパイプラインは
    /// b42 で撤去した（#85 = A）。初期化ステージだけが in-scope の縮退計画でだけ到達する。
    UnwiredTransition {
        /// 打てなかった段。
        step: TransitionStep,
        /// その段の対象ステージ。
        stage: StageSlug,
    },
    /// 定義の再構成の失敗（ポートからそのまま伝播 — 段 11 のレビュー方針を引くとき）。
    DefinitionRepository(RepositoryError<WorkflowDefinitionId>),
    /// 承認しようとしたステージを**定義が知らない**（b48 / 段 11）。
    ///
    /// 実行の計画は定義から作られるので通常は起きない。定義が改訂されて slug が消えた
    /// 歴史でだけ到達する。
    UnknownDefinitionStage {
        /// 定義に無かったステージ。
        stage: StageSlug,
    },
    /// intent が運ぶ `--review` の値が閉集合の外だった（壊れた歴史）。
    ///
    /// `--review` は鋳造時に閉集合で受けているので、ここに来るのは記録の破損である。
    CorruptReviewOverride(String),
    /// 関連付けた定義と依頼が一致しない等、レビュー方針の解決拒否。
    ReviewPolicy(core_command_domain::orchestration::IntentReviewError),
}

impl fmt::Display for CommitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommitError::Repository(error) => write!(f, "repository: {error}"),
            CommitError::IntentRepository(error) => write!(f, "intent repository: {error}"),
            CommitError::Refused(refusal) => write!(f, "refused: {refusal}"),
            CommitError::Transition { step, stage, error } => {
                write!(f, "transition {} for {}: {error}", step.subcommand(), stage)
            }
            CommitError::UnwiredTransition { step, stage } => {
                write!(f, "unwired transition {} for {}", step.subcommand(), stage)
            }
            CommitError::DefinitionRepository(error) => {
                write!(f, "workflow definition repository: {error}")
            }
            CommitError::UnknownDefinitionStage { stage } => {
                write!(f, "the definition has no stage {stage}")
            }
            CommitError::CorruptReviewOverride(raw) => {
                write!(f, "corrupt review override: {raw}")
            }
            CommitError::ReviewPolicy(error) => write!(f, "review policy: {error}"),
        }
    }
}

impl std::error::Error for CommitError {
    /// 内包した失敗へ連鎖する（材料を自分で持つ 2 変種だけは連鎖しない）。
    ///
    /// **封筒は連鎖を切ってはならない。** `RepositoryError::Corrupt` は「壊れていた」としか
    /// `Display` に書かず、実材料は `Error::source` の連鎖に載せる（裁定 6）。ここで `None` を
    /// 返すと、その材料はこの型で行き止まりになり、診断には分類だけが残る。
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CommitError::Repository(error) => Some(error),
            CommitError::IntentRepository(error) => Some(error),
            CommitError::Refused(refusal) => Some(refusal),
            CommitError::Transition { error, .. } => Some(error),
            CommitError::DefinitionRepository(error) => Some(error),
            CommitError::ReviewPolicy(error) => Some(error),
            // ユースケース自身の失敗 — 材料 (段と slug) は自分の `Display` にある。
            CommitError::UnwiredTransition { .. }
            | CommitError::UnknownDefinitionStage { .. }
            | CommitError::CorruptReviewOverride(_) => None,
        }
    }
}

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

impl From<ReportRefusal> for CommitError {
    fn from(refusal: ReportRefusal) -> CommitError {
        CommitError::Refused(refusal)
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

    /// b48 の 3 変種も材料だけを綴り、ポートの失敗だけが連鎖を続ける。
    #[test]
    fn the_definition_side_failures_carry_their_material() {
        let definition_id =
            core_command_domain::workflow_definition::WorkflowDefinitionId::parse("claude")
                .expect("系譜名");
        let repository = CommitError::DefinitionRepository(RepositoryError::NotFound {
            id: definition_id.clone(),
        });
        assert_eq!(
            repository.to_string(),
            format!("workflow definition repository: not found: {definition_id}")
        );
        assert!(std::error::Error::source(&repository).is_some());

        let unknown = CommitError::UnknownDefinitionStage { stage: stage() };
        assert_eq!(
            unknown.to_string(),
            "the definition has no stage practices-discovery"
        );
        assert!(std::error::Error::source(&unknown).is_none());

        let corrupt = CommitError::CorruptReviewOverride("Adversarial".to_string());
        assert_eq!(corrupt.to_string(), "corrupt review override: Adversarial");
        assert!(std::error::Error::source(&corrupt).is_none());
    }

    #[test]
    fn a_refused_report_is_carried_verbatim() {
        let error = CommitError::from(ReportRefusal::StillPending { stage: stage() });
        assert!(matches!(
            error,
            CommitError::Refused(ReportRefusal::StillPending { .. })
        ));
        assert_eq!(
            error.to_string(),
            "refused: still pending: practices-discovery"
        );
    }

    #[test]
    fn an_intent_lookup_failure_names_its_own_port() {
        // 実行は読めたが計画が引けない場合。封筒は面を取り違えず、連鎖も切らない。
        let intent_id = IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("UUIDv7");
        let error = CommitError::from(RepositoryError::NotFound {
            id: intent_id.clone(),
        });
        assert_eq!(
            error.to_string(),
            format!("intent repository: not found: {intent_id}")
        );
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn a_refusal_chains_to_the_material_the_aggregate_carried() {
        let error = CommitError::from(ReportRefusal::StillPending { stage: stage() });
        assert_eq!(
            std::error::Error::source(&error)
                .expect("判断の拒否へ連鎖する")
                .to_string(),
            "still pending: practices-discovery"
        );
    }

    #[test]
    fn a_refused_transition_names_the_step_and_the_stage() {
        let error = CommitError::Transition {
            step: TransitionStep::Approve,
            stage: stage(),
            error: CommandError::NotRunning,
        };
        assert_eq!(
            error.to_string(),
            "transition approve for practices-discovery: not running"
        );
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn the_envelope_chains_to_the_material_the_port_hid_in_its_source() {
        // `RepositoryError::Corrupt` は分類しか `Display` に書かない (裁定 6) — 実材料は
        // `source` の連鎖に載る。封筒がそこで連鎖を切ると、診断には分類しか残らない。
        let execution_id =
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("UUIDv7");
        let error = CommitError::Repository(RepositoryError::Corrupt {
            id: execution_id,
            seq_nr: Some(3),
            source: Box::new(std::io::Error::other("undecodable payload")),
        });

        let port = std::error::Error::source(&error).expect("ポートの失敗へ連鎖する");
        assert_eq!(
            std::error::Error::source(port)
                .expect("ポートは原因へ連鎖する")
                .to_string(),
            "undecodable payload"
        );
    }

    #[test]
    fn a_failure_that_owns_its_material_ends_the_chain() {
        // 未配線の段はユースケース自身の失敗で、材料 (段と slug) は自分の `Display` にある。
        let error = CommitError::UnwiredTransition {
            step: TransitionStep::CompleteWorkflow,
            stage: stage(),
        };
        assert_eq!(
            error.to_string(),
            "unwired transition complete-workflow for practices-discovery"
        );
        assert!(std::error::Error::source(&error).is_none());
    }

    #[test]
    fn the_failure_is_a_std_error() {
        let error: Box<dyn std::error::Error> = Box::new(CommitError::UnwiredTransition {
            step: TransitionStep::Advance,
            stage: stage(),
        });
        assert_eq!(
            error.to_string(),
            "unwired transition advance for practices-discovery"
        );
    }

    #[test]
    fn failures_pattern_match_by_variant() {
        // `PartialEq` は持たない (裁定 6 — `Corrupt` の `source` が比較不能)。判定は
        // `matches!` で行う。
        let refused = CommitError::Refused(ReportRefusal::UnknownStage {
            named: "nope".to_string(),
        });
        assert!(matches!(
            &refused,
            CommitError::Refused(ReportRefusal::UnknownStage { named }) if named == "nope"
        ));
        assert!(!matches!(&refused, CommitError::Repository(_)));
    }
}
