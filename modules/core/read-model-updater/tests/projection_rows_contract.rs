//! 投影核 `project` の行契約 — 監査行の欄と状態ファイルの書換を、入力を与えて独立に書き
//! 下した期待値と比べる（計画に無いステージの拒否・任意欄の有無・学びの team 面書換など、
//! ゴールデンが持たない分岐）。

// テストコードでは unwrap / expect / panic を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    ArtifactPaths, CapturedLearning, CapturedLearnings, CodeGenerationAuthority, Created,
    DecisionPrompt, DecisionRecorded, GateRejected, Intent, IntentEventId, IntentExecutionEvent,
    IntentExecutionEventId, IntentExecutionId, IntentId, Learning, LearningCandidateId,
    LearningDisposition, LearningProvenance, LearningScope, LearningSource, LearningsCaptured,
    PipelineHandoff, PipelineLinkCompleted, PipelineReceipt, PlanAnswerInput, PlanAnswerLogged,
    PlanApprovalEvidence, PlanApprovalOperationId, PlanApprovalOrigin, PlanChoice,
    PlanDecisionEvidence, PlanSession, PlanTarget, PracticeHeading, PromptObserved, ReportId,
    ReportResult, ReportTransition, Reported, SingleStageRunStarted, StageDisplay, StageEntries,
    StageEntry, StageValidation, StartRequest, TaskSynchronized, TransitionStep, TransitionSteps,
    WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
    WorkflowDefinitionId,
};
use core_command_domain::workspace::{IntentDirName, SpaceName};
use core_read_model_updater::orchestration::{GlobalSeqNr, JournalEntry};
use core_read_model_updater::workspace::{ProjectionError, ReadModel, ResolvedPlan, project};

const EXECUTION: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";

fn event_id() -> IntentExecutionEventId {
    IntentExecutionEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002").unwrap()
}
fn execution_id() -> IntentExecutionId {
    IntentExecutionId::parse(EXECUTION).unwrap()
}
fn intent_id() -> IntentId {
    IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap()
}
fn at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-08-21T09:14:07Z")
        .unwrap()
        .with_timezone(&Utc)
}
fn slug(value: &str) -> StageSlug {
    StageSlug::parse(value).unwrap()
}
fn stage(name: &str, number: &str, phase: PhaseId, action: PlanAction) -> StageEntry {
    StageEntry::new(
        slug(name),
        phase,
        action,
        false,
        StageDisplay::new(
            StageNumber::parse(number).unwrap(),
            "Some Title",
            "orchestrator",
        )
        .unwrap(),
    )
}
fn genesis_intent() -> Intent {
    Intent::from((
        Created::new(
            IntentEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0001").unwrap(),
            intent_id(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            StartRequest::new("classic", "build it"),
            StageEntries::new(vec![
                stage(
                    "state-init",
                    "0.1",
                    PhaseId::Initialization,
                    PlanAction::Execute,
                ),
                stage("first", "2.1", PhaseId::Inception, PlanAction::Execute),
                stage("second", "2.2", PhaseId::Inception, PlanAction::Execute),
                stage("late", "4.1", PhaseId::Operation, PlanAction::Skip),
            ])
            .unwrap(),
            WorkspaceScan::new(
                BrownfieldGreenfield::Greenfield,
                "Unknown",
                "Unknown",
                "Unknown",
            )
            .unwrap(),
        ),
        at(),
    ))
}
fn plan() -> ResolvedPlan {
    ResolvedPlan::of(&genesis_intent())
}

const SKELETON: &str = "\
## Project Information
- **Active Agent**: orchestrator

## Scope Configuration
- **Stages to Execute**: 0.1, 2.1, 2.2
- **Stages to Skip**: 4.1 (late)

## Execution Plan Summary
- **Total Stages**: 3
- **Completed**: 0
- **In Progress**: state-init

## Runtime State
- **Revision Count**: 0
- **Construction Autonomy Mode**: gated

## Stage Progress
- [-] state-init — EXECUTE
- [ ] first — EXECUTE
- [ ] second — EXECUTE
- [ ] late — SKIP

## Phase Progress
- **Initialization**: Active
- **Ideation**: Pending
- **Inception**: Pending
- **Construction**: Pending
- **Operation**: Pending

## Current Status
- **Lifecycle Phase**: INITIALIZATION
- **Current Stage**: state-init
- **Next Stage**: first
- **Status**: Running
- **Last Updated**: 2026-08-20T00:00:00Z

## Session Resume Point
- **Last Completed Stage**: 
- **Next Action**: Execute Stage
";

fn entry(event: IntentExecutionEvent) -> JournalEntry {
    JournalEntry::new(GlobalSeqNr::new(1), execution_id(), 1, at(), event)
}

/// 1 イベントを骨格へ投影する。
fn run(event: IntentExecutionEvent) -> Result<ReadModel, ProjectionError> {
    let mut model = ReadModel::new(SKELETON);
    project(&[entry(event)], &plan(), &mut model)?;
    Ok(model)
}

fn unknown_stage(error: ProjectionError) -> String {
    match error {
        ProjectionError::UnknownStage { stage } => stage,
        other => panic!("UnknownStage を期待した: {other:?}"),
    }
}

fn sha(fill: char) -> String {
    fill.to_string().repeat(64)
}

fn plan_answer(choice: PlanChoice) -> IntentExecutionEvent {
    let authority = CodeGenerationAuthority::new(
        &PlanTarget::stage_level(),
        &intent_id(),
        format!("sha256:{}", sha('a')),
        "WORKFLOW_STARTED:2026-09-08T01:00:00Z#1".to_string(),
        sha('b'),
        2,
    )
    .unwrap();
    let evidence = PlanApprovalEvidence::new(
        authority,
        format!("sha256:{}", sha('c')),
        "questions.md".to_string(),
        sha('d'),
        sha('e'),
    )
    .unwrap();
    IntentExecutionEvent::PlanAnswerLogged(Box::new(PlanAnswerLogged::new(
        event_id(),
        execution_id(),
        PlanApprovalOperationId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0999").unwrap(),
        PlanAnswerInput::new(
            PlanApprovalOrigin::new(SpaceName::default(), execution_id()),
            "code-generation".to_string(),
            PlanDecisionEvidence::new(evidence, PlanSession::new("session".to_string()).unwrap()),
            choice,
            None,
        ),
    )))
}

#[test]
fn a_plan_answer_names_its_checkpoint_by_the_choice_and_leaves_the_state_alone() {
    for (choice, kind) in [
        (PlanChoice::RequestChanges, "QUESTION_ANSWERED"),
        (PlanChoice::ApprovePlan, "PLAN_APPROVAL_RECORDED"),
    ] {
        let model = run(plan_answer(choice)).unwrap();
        let audit = model.appended_audit();
        assert!(
            audit.contains(&format!("**Event**: {kind}")),
            "{kind}: {audit}"
        );
        assert!(audit.contains("**Details**: "), "{audit}");
        assert!(
            audit.contains(&format!("**Details**: {}", choice.as_str())),
            "{audit}"
        );
        assert!(audit.contains("**Checkpoint**: Code Generation Plan Approval"));
        assert!(audit.contains("**Session**: session"));
        assert_eq!(model.state(), SKELETON, "状態ファイルは触らない");
    }
}

#[test]
fn a_pipeline_link_row_carries_the_repo_and_the_single_stage_workflow_when_present() {
    let receipt = |repo: Option<&str>, single: bool| {
        IntentExecutionEvent::PipelineLinkCompleted(PipelineLinkCompleted::new(
            event_id(),
            execution_id(),
            PipelineReceipt::new(
                "reverse-engineering".to_string(),
                "aidlc-architect-agent".to_string(),
                repo.map(str::to_string),
                single,
                2,
                3,
                Some(
                    PipelineHandoff::new(
                        "aidlc/handoff.json".to_string(),
                        format!("sha256:{}", sha('9')),
                        "1700000000000".to_string(),
                    )
                    .unwrap(),
                ),
            )
            .unwrap(),
        ))
    };
    let with = run(receipt(Some("modules/app"), true)).unwrap();
    let audit = with.appended_audit();
    assert!(
        audit.contains("**Event**: PIPELINE_LINK_COMPLETED"),
        "{audit}"
    );
    assert!(audit.contains("**Position**: 2/3"));
    assert!(audit.contains("**Artifact Path**: aidlc/handoff.json"));
    assert!(audit.contains("**Repo**: modules/app"));
    assert!(audit.contains("**Workflow**: single-stage:reverse-engineering"));
    let without = run(receipt(None, false)).unwrap();
    let audit = without.appended_audit();
    assert!(!audit.contains("**Repo**"), "{audit}");
    assert!(!audit.contains("**Workflow**"), "{audit}");
    assert_eq!(with.state(), SKELETON);
}

#[test]
fn an_attended_prompt_writes_a_human_turn_row_with_its_session_and_an_unattended_one_writes_nothing()
 {
    let prompt = |session: &str, unattended: bool| {
        IntentExecutionEvent::PromptObserved(PromptObserved::new(
            event_id(),
            execution_id(),
            session,
            "A",
            unattended,
        ))
    };
    let attended = run(prompt("session-1", false)).unwrap();
    let audit = attended.appended_audit();
    assert!(audit.contains("**Event**: HUMAN_TURN"), "{audit}");
    assert!(audit.contains("**Session**: session-1"), "{audit}");
    let sessionless = run(prompt("", false)).unwrap();
    let audit = sessionless.appended_audit();
    assert!(audit.contains("**Event**: HUMAN_TURN"), "{audit}");
    assert!(!audit.contains("**Session**"), "{audit}");
    let unattended = run(prompt("session-1", true)).unwrap();
    assert_eq!(unattended.appended_audit(), "", "無人応答は行を描かない");
}

#[test]
fn a_single_stage_run_start_names_the_lead_agent_and_refuses_a_stage_outside_the_plan() {
    let start = |name: &str| {
        IntentExecutionEvent::SingleStageRunStarted(SingleStageRunStarted::new(
            event_id(),
            execution_id(),
            slug(name),
        ))
    };
    let model = run(start("first")).unwrap();
    let audit = model.appended_audit();
    assert!(audit.contains("**Event**: STAGE_STARTED"), "{audit}");
    assert!(audit.contains("**Stage**: first"));
    assert!(audit.contains("**Agent**: orchestrator"));
    assert!(audit.contains("**Workflow**: single-stage:first"));
    assert_eq!(model.state(), SKELETON);
    assert_eq!(unknown_stage(run(start("ghost")).unwrap_err()), "ghost");
}

#[test]
fn task_synchronization_moves_the_cursor_and_refuses_a_stage_outside_the_plan() {
    let sync = |name: &str| {
        IntentExecutionEvent::TaskSynchronized(TaskSynchronized::new(
            event_id(),
            execution_id(),
            slug(name),
        ))
    };
    let model = run(sync("second")).unwrap();
    let state = model.state();
    assert!(state.contains("- **Current Stage**: second"), "{state}");
    assert!(state.contains("- **In Progress**: second"), "{state}");
    assert!(
        state.contains("- **Lifecycle Phase**: INCEPTION"),
        "{state}"
    );
    assert!(state.contains("- [-] second — EXECUTE"), "{state}");
    assert!(
        state.contains("- **Last Updated**: 2026-08-21T09:14:07Z"),
        "{state}"
    );
    assert_eq!(unknown_stage(run(sync("ghost")).unwrap_err()), "ghost");
}

#[test]
fn a_decision_row_carries_options_rationale_and_summary_file_only_when_present() {
    let bare = run(IntentExecutionEvent::DecisionRecorded(
        DecisionRecorded::new(
            event_id(),
            execution_id(),
            DecisionPrompt::new("first", "Approve"),
        ),
    ))
    .unwrap();
    let audit = bare.appended_audit();
    assert!(audit.contains("**Event**: DECISION_RECORDED"), "{audit}");
    assert!(!audit.contains("**Options**"), "{audit}");
    assert!(!audit.contains("**Rationale**"), "{audit}");
    let full = run(IntentExecutionEvent::DecisionRecorded(
        DecisionRecorded::new(
            event_id(),
            execution_id(),
            DecisionPrompt::new("first", "Approve")
                .with_options("A. Approve,B. Reject")
                .with_rationale("looks complete")
                .with_summary_file("first-questions.md"),
        ),
    ))
    .unwrap();
    let audit = full.appended_audit();
    assert!(
        audit.contains("**Options**: A. Approve,B. Reject"),
        "{audit}"
    );
    assert!(audit.contains("**Rationale**: looks complete"), "{audit}");
    assert!(audit.contains("first-questions.md"), "{audit}");
}

#[test]
fn a_rejection_with_feedback_writes_the_feedback_on_both_rows_and_bumps_the_revision_count() {
    let reject = |feedback: Option<&str>| {
        IntentExecutionEvent::GateRejected(GateRejected::new(
            event_id(),
            execution_id(),
            slug("first"),
            feedback.map(str::to_string),
        ))
    };
    let with = run(reject(Some("more detail"))).unwrap();
    let audit = with.appended_audit();
    assert_eq!(
        audit.matches("**Feedback**: more detail").count(),
        2,
        "{audit}"
    );
    assert!(audit.contains("**Event**: GATE_REJECTED"), "{audit}");
    assert!(audit.contains("**Event**: STAGE_REVISING"), "{audit}");
    assert!(audit.contains("**Revision count**: 1"), "{audit}");
    assert!(
        with.state().contains("- **Revision Count**: 1"),
        "{}",
        with.state()
    );
    let without = run(reject(None)).unwrap();
    assert!(!without.appended_audit().contains("**Feedback**"));
}

#[test]
fn a_reported_approval_puts_the_validation_warning_on_the_completion_row() {
    let approve = |validation: Option<StageValidation>| {
        IntentExecutionEvent::Reported(
            Reported::new(
                event_id(),
                execution_id(),
                ReportId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0555").unwrap(),
                ReportResult::Committed {
                    stage: slug("first"),
                    scope: "classic".to_string(),
                    steps: TransitionSteps::new(vec![TransitionStep::Approve]).unwrap(),
                    transition: ReportTransition::GateApproved {
                        user_input: Some("Approve".to_string()),
                    },
                },
                validation,
                None,
            )
            .unwrap(),
        )
    };
    let warned = run(approve(Some(StageValidation::Warning(
        "receipt unavailable".to_string(),
    ))))
    .unwrap();
    let audit = warned.appended_audit();
    assert!(
        audit.contains("**Validation Warning**: receipt unavailable"),
        "{audit}"
    );
    let based = run(approve(Some(StageValidation::Basis(
        "{\"schema\":3}".to_string(),
    ))))
    .unwrap();
    assert!(
        based
            .appended_audit()
            .contains("**Validation Basis**: {\"schema\":3}")
    );
    let plain = run(approve(None)).unwrap();
    assert!(!plain.appended_audit().contains("**Validation"));
    assert!(
        plain.state().contains("- [x] first — EXECUTE"),
        "{}",
        plain.state()
    );
}

#[test]
fn a_team_scoped_learning_is_appended_under_its_heading_in_team_md() {
    let captured = IntentExecutionEvent::LearningsCaptured(Box::new(LearningsCaptured::new(
        event_id(),
        execution_id(),
        slug("first"),
        LearningProvenance::new(
            SpaceName::default(),
            IntentDirName::parse("260908-learnings").unwrap(),
        ),
        CapturedLearnings::new(vec![
            CapturedLearning::new(
                Learning::new(
                    LearningCandidateId::parse("c1").unwrap(),
                    LearningScope::Team,
                    PracticeHeading::from_routed("Testing Posture"),
                    "ALWAYS run the suite",
                    LearningSource::UserAddition,
                ),
                LearningDisposition::Fresh,
            ),
            CapturedLearning::new(
                Learning::new(
                    LearningCandidateId::parse("c2").unwrap(),
                    LearningScope::Project,
                    PracticeHeading::from_routed("Way of Working"),
                    "NEVER force-push",
                    LearningSource::Orchestrator,
                ),
                LearningDisposition::PracticeLineOnly,
            ),
        ]),
    )));
    let mut model =
        ReadModel::new(SKELETON).with_memory("# Team\n\n## Testing Posture\n", "# Project\n");
    project(&[entry(captured)], &plan(), &mut model).unwrap();
    let memory = model.memory().unwrap();
    assert!(
        memory.team().contains("## Testing Posture\n")
            && memory.team().contains("ALWAYS run the suite"),
        "team.md: {}",
        memory.team()
    );
    assert!(
        memory.project().contains("## Way of Working")
            && memory.project().contains("NEVER force-push"),
        "project.md (見出しは足りなければ作る): {}",
        memory.project()
    );
    assert!(
        !memory.team().contains("NEVER force-push"),
        "project 向けの学びは team.md に混ざらない"
    );
    // 実践行を書く学びが 1 つでもあるのにメモリ層が載っていなければ描けない。
    let mut without = ReadModel::new(SKELETON);
    let error = project(
        &[entry(IntentExecutionEvent::LearningsCaptured(Box::new(
            LearningsCaptured::new(
                event_id(),
                execution_id(),
                slug("first"),
                LearningProvenance::new(
                    SpaceName::default(),
                    IntentDirName::parse("260908-learnings").unwrap(),
                ),
                CapturedLearnings::new(vec![CapturedLearning::new(
                    Learning::new(
                        LearningCandidateId::parse("c1").unwrap(),
                        LearningScope::Team,
                        PracticeHeading::from_routed("Testing Posture"),
                        "ALWAYS run the suite",
                        LearningSource::UserAddition,
                    ),
                    LearningDisposition::Fresh,
                )]),
            ),
        )))],
        &plan(),
        &mut without,
    )
    .unwrap_err();
    assert!(
        matches!(error, ProjectionError::MemoryFilesMissing),
        "{error:?}"
    );
}

#[test]
fn a_gate_opening_for_a_stage_outside_the_plan_is_refused_by_name() {
    let error = run(IntentExecutionEvent::GateOpened(
        core_command_domain::orchestration::GateOpened::new(
            event_id(),
            execution_id(),
            slug("ghost"),
            ArtifactPaths::empty(),
        ),
    ))
    .unwrap_err();
    // ゲート開始は計画の表示属性を要しないので、状態ファイルのチェックボックス欠落で止まる。
    assert!(
        matches!(error, ProjectionError::Checkbox(_)),
        "計画外のステージは骨格に行を持たない: {error:?}"
    );
}
