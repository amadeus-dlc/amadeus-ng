//! 拒否型の `Display` / `Error::source` 契約 — 材料のみを綴り、逐語文言は出す側が組む。
//!
//! 境界でのエラー文言変換規則 (`coding-rules/error-handling.md`) に従い、ドメインの拒否型は
//! **材料**だけを `Display` に書く。ここではその材料の綴りを契約として固定し、原因連鎖
//! (`Error::source`) が途切れないことを検証する。
#![allow(clippy::unwrap_used)]
use core_command_domain::orchestration::{
    AnswerError, AnswerIdError, CommandError, ContinuationError, PipelineLinkError,
    PlanRuntimeError, ReportCommitError, ReportIdError, ReportRefusal, ReportResultError,
    ReviewEvidenceError, ReviewedUnitError, ReviewerScopeVerdict, RunFloorError, ScopeTokenError,
    SummaryQuestionsError, TransitionStep, UnitNameError, WriteTargetError,
};
use core_command_domain::workspace::{HookHealthError, SessionAuditError};
use std::error::Error;

#[test]
fn the_answer_rejections_carry_material_not_wording() {
    assert_eq!(
        AnswerError::InvalidSummaryChoice.to_string(),
        "invalid summary choice"
    );
    assert_eq!(
        AnswerError::SummaryQuestionMissing.to_string(),
        "summary question missing"
    );
    assert_eq!(
        AnswerError::SummaryHumanReplyMissing.to_string(),
        "human reply after summary missing"
    );
    assert_eq!(AnswerError::Dismissed.to_string(), "dismissed question");
    assert_eq!(
        AnswerError::HumanReplyMissing {
            approval_choice: true
        }
        .to_string(),
        "human reply missing (approval choice: true)"
    );
    assert_eq!(
        AnswerError::Command(CommandError::NotRunning).to_string(),
        "not running"
    );
}

#[test]
fn the_answer_rejection_chains_only_to_a_wrapped_command_rejection() {
    let wrapped = AnswerError::Command(CommandError::NotRunning);
    assert_eq!(
        Error::source(&wrapped).map(ToString::to_string),
        Some("not running".to_string())
    );
    for bare in [
        AnswerError::InvalidSummaryChoice,
        AnswerError::SummaryQuestionMissing,
        AnswerError::SummaryHumanReplyMissing,
        AnswerError::Dismissed,
        AnswerError::HumanReplyMissing {
            approval_choice: false,
        },
    ] {
        assert!(Error::source(&bare).is_none(), "{bare:?}");
    }
}

#[test]
fn the_review_evidence_rejections_carry_material_not_wording() {
    assert_eq!(
        ReviewEvidenceError::PendingIterations(vec![1, 2]).to_string(),
        "pending review iterations: [1, 2]"
    );
    assert_eq!(
        ReviewEvidenceError::RetryAlreadyUsed.to_string(),
        "review retry already used"
    );
    assert_eq!(
        ReviewEvidenceError::ArtifactsUnavailable.to_string(),
        "artifacts unavailable"
    );
    assert_eq!(
        ReviewEvidenceError::ArtifactsChanged.to_string(),
        "artifacts changed"
    );
    assert_eq!(
        ReviewEvidenceError::SourceChanged.to_string(),
        "source changed"
    );
    assert_eq!(
        ReviewEvidenceError::InvalidBinding.to_string(),
        "invalid review binding"
    );
    assert_eq!(
        ReviewEvidenceError::StaleAppendix.to_string(),
        "prior appendix retained"
    );
    assert_eq!(
        ReviewEvidenceError::InvalidAppendix("why".to_string()).to_string(),
        "why"
    );
    // 集約の拒否は証拠の不一致をそのまま透過する。
    assert_eq!(
        CommandError::ReviewEvidence(ReviewEvidenceError::SourceChanged).to_string(),
        "source changed"
    );
}

#[test]
fn the_pipeline_and_plan_response_rejections_carry_material_not_wording() {
    assert_eq!(
        CommandError::SingleStageAttemptAlreadyOpen.to_string(),
        "pipeline attempt already open"
    );
    assert_eq!(
        CommandError::SingleStageAttemptNotOpen.to_string(),
        "pipeline attempt not open"
    );
    assert_eq!(
        CommandError::PipelineLinksMissing {
            stage: "reverse-engineering".to_string(),
            missing: "aidlc-developer-agent".to_string(),
            single: true,
        }
        .to_string(),
        "pipeline reverse-engineering: missing aidlc-developer-agent, single=true"
    );
    assert_eq!(
        CommandError::PlanResponseUnavailable.to_string(),
        "no prepared Plan Approval response"
    );
    assert_eq!(
        CommandError::PlanResponseTargetMismatch.to_string(),
        "Plan Approval response belongs to another execution"
    );
    assert_eq!(
        CommandError::PlanResponseAlreadyRecorded.to_string(),
        "Plan Approval response has already been recorded"
    );
}

#[test]
fn every_plan_runtime_rejection_has_its_own_material() {
    let cases = [
        (
            PlanRuntimeError::PendingGeneration,
            "pending generation publication must be recovered before approval operations",
        ),
        (
            PlanRuntimeError::NoPendingGeneration,
            "no pending generation publication",
        ),
        (
            PlanRuntimeError::ReceiptSourceChanged,
            "Plan Approval source changed during receipt certification; present the current plan again",
        ),
        (
            PlanRuntimeError::PendingAnswer,
            "pending Plan Approval answer must be recovered before approval operations",
        ),
        (
            PlanRuntimeError::NoPendingAnswer,
            "no pending Plan Approval answer",
        ),
        (
            PlanRuntimeError::AnswerTargetMismatch,
            "Plan Approval answer target does not match its source execution",
        ),
        (
            PlanRuntimeError::AnswerNotRecorded,
            "Plan Approval answer has not been recorded by its source execution",
        ),
        (
            PlanRuntimeError::InvalidationTargetMismatch,
            "Plan Approval invalidation target does not match the original publication",
        ),
        (
            PlanRuntimeError::NoPendingChallenge,
            "no pending Plan Approval challenge for this session",
        ),
        (
            PlanRuntimeError::NoPreparedResponse,
            "no prepared Plan Approval response",
        ),
        (
            PlanRuntimeError::ResponseTargetMismatch,
            "Plan Approval response target does not match its observation",
        ),
        (
            PlanRuntimeError::ResponseNotRecorded,
            "Plan Approval response has not been recorded by its source execution",
        ),
        (
            PlanRuntimeError::PendingResponse,
            "pending human response must be recovered before Plan Approval",
        ),
        (
            PlanRuntimeError::AlreadyInUse,
            "shared approval runtime already contains workflow operations",
        ),
        (
            PlanRuntimeError::InvalidState,
            "invalid Plan Approval runtime state",
        ),
        (
            PlanRuntimeError::PendingInvalidation,
            "pending directive publication must be recovered before Plan Approval",
        ),
        (
            PlanRuntimeError::OperationAlreadyApplied,
            "Plan Approval operation has already been applied",
        ),
        (
            PlanRuntimeError::UnknownInvalidation,
            "Plan Approval invalidation was not prepared",
        ),
    ];
    let mut seen = std::collections::BTreeSet::new();
    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected, "{error:?}");
        assert!(seen.insert(expected), "文言が重複した: {expected}");
        assert!(Error::source(&error).is_none());
    }
}

#[test]
fn the_hook_health_and_session_audit_rejections_carry_material_not_wording() {
    assert_eq!(
        HookHealthError::InvalidHookName.to_string(),
        "invalid hook name"
    );
    assert_eq!(
        HookHealthError::InvalidIdentity.to_string(),
        "invalid hook health identity"
    );
    assert_eq!(
        HookHealthError::InvalidEventIdentity.to_string(),
        "invalid hook health event identity"
    );
    assert_eq!(
        HookHealthError::TargetMismatch.to_string(),
        "hook health target mismatch"
    );
    assert_eq!(
        HookHealthError::InvalidHistory.to_string(),
        "invalid hook health history"
    );
    assert_eq!(
        HookHealthError::CounterExhausted.to_string(),
        "hook health counter exhausted"
    );
    assert_eq!(
        SessionAuditError::InvalidIdentity.to_string(),
        "session audit: InvalidIdentity"
    );
    assert_eq!(
        SessionAuditError::CounterExhausted.to_string(),
        "session audit: CounterExhausted"
    );
}

#[test]
fn the_continuation_rejection_renders_its_variant() {
    for error in [
        ContinuationError::InvalidIdentity,
        ContinuationError::InvalidSignature,
        ContinuationError::InvalidLimit,
        ContinuationError::InvalidHistory,
        ContinuationError::CounterExhausted,
    ] {
        assert_eq!(error.to_string(), format!("continuation: {error:?}"));
    }
}

#[test]
fn the_single_variant_parse_rejections_carry_material_not_wording() {
    assert_eq!(UnitNameError::Empty.to_string(), "unit name is empty");
    assert_eq!(
        UnitNameError::Separated.to_string(),
        "unit name contains a path separator"
    );
    assert_eq!(
        WriteTargetError::Empty.to_string(),
        "write target names no path"
    );
    assert_eq!(ScopeTokenError::Empty.to_string(), "scope token is empty");
    assert_eq!(
        ReviewedUnitError::Empty.to_string(),
        "reviewed unit is empty"
    );
    assert_eq!(
        ReportIdError::NotCanonicalUuidV7.to_string(),
        "not a canonical UUIDv7 (expected lowercase 8-4-4-4-12)"
    );
    assert_eq!(
        AnswerIdError::NotCanonicalUuidV7.to_string(),
        "not a canonical UUIDv7 (expected lowercase 8-4-4-4-12)"
    );
    assert_eq!(
        RunFloorError.to_string(),
        "run boundary and occurrence counts do not agree"
    );
}

#[test]
fn the_summary_and_report_result_rejections_carry_material_not_wording() {
    assert_eq!(
        SummaryQuestionsError::InvalidAnswer.to_string(),
        "summary answer mismatch"
    );
    assert_eq!(
        SummaryQuestionsError::InvalidStructure("bad heading".to_string()).to_string(),
        "bad heading"
    );
    assert_eq!(
        ReportResultError::TransitionMismatch.to_string(),
        "report transition mismatch"
    );
    assert_eq!(
        ReportResultError::BaselineWithoutAdvance.to_string(),
        "source baseline without stage advance"
    );
}

#[test]
fn the_report_commit_rejection_chains_to_its_cause_except_when_unwired() {
    let invalid = ReportCommitError::InvalidResult(ReportResultError::TransitionMismatch);
    assert_eq!(invalid.to_string(), "report transition mismatch");
    assert!(Error::source(&invalid).is_some());

    let refused = ReportCommitError::Refused(ReportRefusal::UnknownStage {
        named: "nowhere".to_string(),
    });
    assert_eq!(refused.to_string(), "unknown stage: nowhere");
    assert!(Error::source(&refused).is_some());

    let transition = ReportCommitError::Transition {
        step: TransitionStep::Approve,
        error: CommandError::NotRunning,
    };
    assert_eq!(transition.to_string(), "Approve: not running");
    assert_eq!(
        Error::source(&transition).map(ToString::to_string),
        Some("not running".to_string())
    );

    let unwired = ReportCommitError::Unwired {
        step: TransitionStep::Skip,
    };
    assert_eq!(unwired.to_string(), "unwired transition: Skip");
    assert!(Error::source(&unwired).is_none());
}

#[test]
fn the_pipeline_link_rejection_renders_its_debug_material() {
    // `Display` は材料 (`Debug` 表現) を運ぶだけで、利用者向け逐語は出す側が組む。
    // 兄弟の `ContinuationError` (`"continuation: {self:?}"`) と同じ 1 行の形に固定する。
    assert_eq!(
        PipelineLinkError::ArtifactRequired.to_string(),
        "pipeline link: ArtifactRequired"
    );
    assert_eq!(
        PipelineLinkError::Command(CommandError::IntentMismatch).to_string(),
        "pipeline link: Command(IntentMismatch)"
    );
    let unknown = PipelineLinkError::UnknownLink {
        stage: "code-generation".to_string(),
        link: "aidlc-quality-agent".to_string(),
        declared: "aidlc-developer-agent".to_string(),
    }
    .to_string();
    assert!(!unknown.contains('\n'), "改行が混入している: {unknown:?}");
    assert!(
        !unknown.contains("///"),
        "doc コメントが混入している: {unknown:?}"
    );
    assert!(
        unknown.starts_with("pipeline link: UnknownLink {"),
        "{unknown}"
    );
}

#[test]
fn a_reviewer_scope_verdict_knows_whether_it_blocked() {
    assert!(!ReviewerScopeVerdict::Allowed.is_blocked());
}
