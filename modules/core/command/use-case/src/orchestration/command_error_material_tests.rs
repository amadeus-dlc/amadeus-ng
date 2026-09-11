//! ユースケースの失敗封筒 — `Display` は材料のみ、`From` と `Error::source` で原因連鎖を切らない。
//!
//! `RepositoryError::Corrupt` は「壊れていた」としか `Display` に書かず、実材料は
//! `Error::source` の連鎖に載せる (裁定 6)。封筒がここで `None` を返すと診断には分類だけが
//! 残るので、各封筒がポートの失敗へ連鎖することを契約として固定する。
#![allow(clippy::unwrap_used)]
use std::error::Error;

use core_command_domain::orchestration::{
    AnswerError, CommandError, ContinuationError, IntentExecutionId, IntentId, PipelineLinkError,
    PlanApprovalRuntimeId, PlanRuntimeError, PlanTarget, WorkflowContinuationId,
};
use core_command_domain::workflow_definition::WorkflowDefinitionId;
use core_command_domain::workspace::{
    ArtifactAuditId, AuditFieldKey, HookHealthError, HookHealthId, HookHealthTarget,
    SessionAuditError, SessionAuditId, SpaceName,
};

use super::{
    ArtifactAuditCommandError, ContinuationCommandError, HealthCheckError, HookHealthCommandError,
    InteractionCommandError, JumpError, LearningCaptureError, MemoryJournalError,
    PipelineLinkCommandError, PlanApprovalCommandError, RepositoryError, ReviewFreezeError,
    ReviewerScopeCause, ReviewerScopeError, SessionAuditCommandError, TaskSynchronizationError,
};

fn execution_id() -> IntentExecutionId {
    IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap()
}

fn intent_id() -> IntentId {
    IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap()
}

fn definition_id() -> WorkflowDefinitionId {
    WorkflowDefinitionId::parse("claude").unwrap()
}

fn corrupt<Id>(id: Id) -> RepositoryError<Id> {
    RepositoryError::Corrupt {
        id,
        seq_nr: Some(2),
        source: Box::new(std::io::Error::other("undecodable payload")),
    }
}

/// 封筒 → ポート → 原因、の 2 段連鎖が保たれている。
fn assert_chains_to_the_hidden_cause(error: &dyn Error) {
    let port = Error::source(error).expect("ポートの失敗へ連鎖する");
    assert_eq!(
        Error::source(port)
            .expect("ポートは原因へ連鎖する")
            .to_string(),
        "undecodable payload"
    );
}

#[test]
fn the_jump_envelope_renders_its_boundary_and_chains_to_the_port() {
    let repository = JumpError::from(corrupt(execution_id()));
    assert_eq!(
        repository.to_string(),
        "repository: corrupt: aggregate 0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000, seq_nr 2"
    );
    assert_chains_to_the_hidden_cause(&repository);

    let intent = JumpError::from(RepositoryError::NotFound { id: intent_id() });
    assert_eq!(
        intent.to_string(),
        "intent repository: not found: 01a02785-1bd8-76eb-aeea-5aa303ebd5b6"
    );
    assert!(Error::source(&intent).is_some());

    let command = JumpError::from(CommandError::NotRunning);
    assert_eq!(command.to_string(), "command: not running");
    assert_eq!(
        Error::source(&command).map(ToString::to_string),
        Some("not running".to_string())
    );

    let definition = JumpError::DefinitionRepository(corrupt(definition_id()));
    assert_eq!(
        definition.to_string(),
        "definition repository: corrupt: aggregate claude, seq_nr 2"
    );
    assert_chains_to_the_hidden_cause(&definition);

    // `Unknown scope: <name>` は本家逐語であり、材料 (scope 名) しか無いので連鎖は無い。
    let unknown = JumpError::UnknownScope("nope".to_string());
    assert_eq!(unknown.to_string(), "Unknown scope: nope");
    assert!(Error::source(&unknown).is_none());
}

#[test]
fn the_plan_approval_envelope_transparently_renders_every_cause() {
    let evidence = PlanApprovalCommandError::from(PlanTarget::for_unit("").unwrap_err());
    assert_eq!(evidence.to_string(), "Unit name is empty");
    assert!(Error::source(&evidence).is_some());

    let command = PlanApprovalCommandError::from(CommandError::PlanResponseUnavailable);
    assert_eq!(command.to_string(), "no prepared Plan Approval response");
    assert!(Error::source(&command).is_some());

    let intent = PlanApprovalCommandError::from(RepositoryError::NotFound { id: intent_id() });
    assert_eq!(
        intent.to_string(),
        "not found: 01a02785-1bd8-76eb-aeea-5aa303ebd5b6"
    );

    let domain = PlanApprovalCommandError::from(PlanRuntimeError::NoPendingAnswer);
    assert_eq!(domain.to_string(), "no pending Plan Approval answer");
    assert!(Error::source(&domain).is_some());

    let runtime = PlanApprovalCommandError::from(corrupt(PlanApprovalRuntimeId::Workspace));
    assert_eq!(
        runtime.to_string(),
        "corrupt: aggregate workspace, seq_nr 2"
    );
    assert_chains_to_the_hidden_cause(&runtime);

    let execution = PlanApprovalCommandError::from(corrupt(execution_id()));
    assert_eq!(
        execution.to_string(),
        "corrupt: aggregate 0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000, seq_nr 2"
    );
    assert_chains_to_the_hidden_cause(&execution);
}

#[test]
fn the_interaction_envelope_names_the_failing_boundary() {
    let answer = InteractionCommandError::Answer(AnswerError::Dismissed);
    assert_eq!(answer.to_string(), "answer: dismissed question");
    assert!(Error::source(&answer).is_some());
    let repository = InteractionCommandError::Repository(corrupt(execution_id()));
    assert_eq!(
        repository.to_string(),
        "repository: corrupt: aggregate 0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000, seq_nr 2"
    );
    assert_chains_to_the_hidden_cause(&repository);
    let command = InteractionCommandError::Command(CommandError::IntentMismatch);
    assert_eq!(command.to_string(), "command: intent mismatch");
    assert!(Error::source(&command).is_some());
}

#[test]
fn the_continuation_envelope_converts_from_its_domain_and_port_failures() {
    let domain = ContinuationCommandError::from(ContinuationError::InvalidLimit);
    assert_eq!(domain.to_string(), "continuation: InvalidLimit");
    assert!(Error::source(&domain).is_some());
    let repository = ContinuationCommandError::from(RepositoryError::Conflict {
        expected: 1,
        actual: 2,
    });
    assert_eq!(repository.to_string(), "conflict: expected 1, actual 2");
    assert!(Error::source(&repository).is_some());
    let execution = ContinuationCommandError::ExecutionRepository(corrupt(execution_id()));
    assert_eq!(
        execution.to_string(),
        "corrupt: aggregate 0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000, seq_nr 2"
    );
    assert_chains_to_the_hidden_cause(&execution);
    let _: RepositoryError<WorkflowContinuationId> = RepositoryError::Conflict {
        expected: 0,
        actual: 0,
    };
}

#[test]
fn the_audit_envelopes_convert_from_their_domain_and_port_failures() {
    let target = HookHealthTarget::new(SpaceName::parse("default").unwrap(), None);

    let artifact = ArtifactAuditCommandError::from(HookHealthError::InvalidHookName);
    assert_eq!(artifact.to_string(), "invalid hook name");
    let artifact_repository = ArtifactAuditCommandError::from(RepositoryError::NotFound {
        id: ArtifactAuditId::for_target(&target),
    });
    assert!(artifact_repository.to_string().starts_with("not found: "));

    let session = SessionAuditCommandError::from(SessionAuditError::InvalidRecord);
    assert_eq!(session.to_string(), "session audit: InvalidRecord");
    let session_repository = SessionAuditCommandError::from(RepositoryError::NotFound {
        id: SessionAuditId::for_target(&target),
    });
    assert!(session_repository.to_string().starts_with("not found: "));

    let hook = HookHealthCommandError::Domain(HookHealthError::TargetMismatch);
    assert_eq!(hook.to_string(), "hook health target mismatch");
    let hook_repository = HookHealthCommandError::Repository(RepositoryError::NotFound {
        id: HookHealthId::for_hook(
            &target,
            &core_command_domain::workspace::HookName::parse("aidlc-session-start").unwrap(),
        ),
    });
    assert!(hook_repository.to_string().starts_with("not found: "));
}

#[test]
fn the_execution_observation_envelopes_share_one_shape() {
    let sync = TaskSynchronizationError::Command(CommandError::NotRunning);
    assert_eq!(sync.to_string(), "command: not running");
    assert!(Error::source(&sync).is_some());
    let sync = TaskSynchronizationError::Repository(corrupt(execution_id()));
    assert!(sync.to_string().starts_with("repository: corrupt: "));
    assert_chains_to_the_hidden_cause(&sync);

    let journal = MemoryJournalError::Command(CommandError::NotRunning);
    assert_eq!(journal.to_string(), "command: not running");
    assert!(Error::source(&journal).is_some());
    let journal = MemoryJournalError::Repository(corrupt(execution_id()));
    assert!(journal.to_string().starts_with("repository: corrupt: "));
    assert_chains_to_the_hidden_cause(&journal);

    let learning = LearningCaptureError::Command(CommandError::UnknownStage("x".to_string()));
    assert_eq!(learning.to_string(), "command: unknown stage x");
    assert!(Error::source(&learning).is_some());
    let learning = LearningCaptureError::Repository(corrupt(execution_id()));
    assert!(learning.to_string().starts_with("repository: corrupt: "));
    assert_chains_to_the_hidden_cause(&learning);

    let health = HealthCheckError::Command(CommandError::NotRunning);
    assert_eq!(health.to_string(), "command: not running");
    assert!(Error::source(&health).is_some());
    let health = HealthCheckError::Repository(corrupt(execution_id()));
    assert!(health.to_string().starts_with("repository: corrupt: "));
    assert_chains_to_the_hidden_cause(&health);
}

#[test]
fn the_review_freeze_envelope_names_the_boundary_it_could_not_read() {
    assert_eq!(
        ReviewFreezeError::Execution(RepositoryError::NotFound { id: execution_id() }).to_string(),
        "execution repository: not found: 0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000"
    );
    assert_eq!(
        ReviewFreezeError::Intent(RepositoryError::NotFound { id: intent_id() }).to_string(),
        "intent repository: not found: 01a02785-1bd8-76eb-aeea-5aa303ebd5b6"
    );
    assert_eq!(
        ReviewFreezeError::Definition(RepositoryError::NotFound {
            id: definition_id()
        })
        .to_string(),
        "definition repository: not found: claude"
    );
    assert_eq!(
        ReviewFreezeError::Audit(SessionAuditCommandError::Domain(
            SessionAuditError::InvalidRecord
        ))
        .to_string(),
        "audit: session audit: InvalidRecord"
    );
    let field = AuditFieldKey::parse("bad key\n").unwrap_err();
    let rendered = ReviewFreezeError::AuditField(field).to_string();
    assert!(rendered.starts_with("audit field: "), "{rendered}");
}

#[test]
fn the_pipeline_link_envelope_names_the_boundary_and_passes_the_rejection_through() {
    assert_eq!(
        PipelineLinkCommandError::Execution(RepositoryError::NotFound { id: execution_id() })
            .to_string(),
        "execution: not found: 0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000"
    );
    assert_eq!(
        PipelineLinkCommandError::Intent(RepositoryError::NotFound { id: intent_id() }).to_string(),
        "intent: not found: 01a02785-1bd8-76eb-aeea-5aa303ebd5b6"
    );
    assert_eq!(
        PipelineLinkCommandError::Definition(RepositoryError::NotFound {
            id: definition_id()
        })
        .to_string(),
        "definition: not found: claude"
    );
    let rejected = PipelineLinkCommandError::Rejected(PipelineLinkError::ArtifactRequired);
    assert_eq!(
        rejected.to_string(),
        PipelineLinkError::ArtifactRequired.to_string()
    );
}

#[test]
fn the_reviewer_scope_error_keeps_the_refusal_and_names_why_it_was_not_recorded() {
    use core_command_domain::orchestration::{
        InspectedTool, ReviewedStage, ReviewedUnit, ReviewerScopeBlock, ScopeToken,
    };
    let block = ReviewerScopeBlock::new(
        InspectedTool::Read,
        ScopeToken::parse("construction/u1/plan.md").unwrap(),
        ReviewedStage::new("code-generation".to_string()),
        ReviewedUnit::parse("u2").unwrap(),
    );
    let error = ReviewerScopeError::new(
        block.clone(),
        ReviewerScopeCause::Audit(SessionAuditCommandError::Domain(
            SessionAuditError::InvalidRecord,
        )),
    );
    assert_eq!(error.block(), &block);
    assert_eq!(
        error.to_string(),
        "the reviewer-scope refusal stands but was not recorded: audit: session audit: InvalidRecord"
    );
    let field = ReviewerScopeError::new(
        block,
        ReviewerScopeCause::AuditField(AuditFieldKey::parse("").unwrap_err()),
    );
    assert!(
        field.cause().to_string().starts_with("audit field: "),
        "{}",
        field.cause()
    );
}

#[test]
fn the_commit_envelope_names_the_pipeline_and_report_result_causes() {
    use core_command_domain::orchestration::ReportResultError;
    let pipeline = super::CommitError::Pipeline(CommandError::SingleStageAttemptNotOpen);
    assert_eq!(pipeline.to_string(), "pipeline: pipeline attempt not open");
    assert_eq!(
        Error::source(&pipeline).map(ToString::to_string),
        Some("pipeline attempt not open".to_string())
    );
    let result = super::CommitError::ReportResult(ReportResultError::TransitionMismatch);
    assert_eq!(
        result.to_string(),
        "report result: report transition mismatch"
    );
    assert_eq!(
        Error::source(&result).map(ToString::to_string),
        Some("report transition mismatch".to_string())
    );
}
