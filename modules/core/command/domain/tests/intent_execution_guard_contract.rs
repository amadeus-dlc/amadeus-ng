//! `IntentExecution` の入口ガード — 別 intent の取り違え、未知 stage、計画承認の照合拒否。
//!
//! 集約は intent を ID で参照するので、渡された `Intent` が自分のものでなければ**状態を
//! 動かさずに** `IntentMismatch` で拒む (`coding-rules/aggregate-references.md`)。ここでは
//! pipeline・計画承認・TaskUpdate の各コマンドがその入口を共有することを検証する。
#![allow(clippy::unwrap_used)]
use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    CommandError, DirectivePublication, Intent, IntentExecution, IntentExecutionId, IntentId,
    PipelineHandoffInput, PipelineLinkError, PipelineLinkRequest, PlanApprovalDocuments,
    PlanApprovalInput, PlanApprovalOrigin, PlanChoice, PlanReceipts, PlanSession, PlanTarget,
    PublishedDirective, ReportRequest, StartRequest, TestingSections, Verdict, WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, CompiledDefinition, CompiledDefinitionId, ExecutionKind, PhaseId,
    ScopeGrid, ScopeMetadata, StageGraph, StageMode, StageNodeBuilder, StageNumber, StageSlug,
    WorkflowDefinition, WorkflowDefinitionId,
};
use core_command_domain::workspace::SpaceName;

const STAGES: [&str; 3] = ["state-init", "code-generation", "build-and-test"];

fn at() -> DateTime<Utc> {
    "2026-09-11T00:00:00Z".parse().unwrap()
}

fn slug(name: &str) -> StageSlug {
    StageSlug::parse(name).unwrap()
}

fn definition() -> WorkflowDefinition {
    let nodes = STAGES
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let phase = if index == 0 {
                PhaseId::Initialization
            } else {
                PhaseId::Construction
            };
            StageNodeBuilder::new(
                slug(name),
                StageNumber::parse(&format!("{}.{}", phase.index(), index + 1)).unwrap(),
                "Stage".to_string(),
                phase,
                ExecutionKind::Always,
                StageMode::Inline,
            )
            .scopes(vec!["classic".to_string()])
            .build()
        })
        .collect();
    let graph = StageGraph::new(nodes).unwrap();
    let grid = ScopeGrid::from_graph(&graph);
    let mut scopes = BTreeMap::new();
    scopes.insert(
        "classic".to_string(),
        ScopeMetadata::new("classic").unwrap(),
    );
    let compiled = CompiledDefinition::compile(
        CompiledDefinitionId::parse("claude").unwrap(),
        graph,
        grid,
        scopes,
    )
    .0;
    WorkflowDefinition::define(
        WorkflowDefinitionId::parse("claude").unwrap(),
        &compiled,
        at(),
    )
    .unwrap()
    .0
}

fn scan() -> WorkspaceScan {
    WorkspaceScan::new(
        BrownfieldGreenfield::Greenfield,
        "Unknown",
        "Unknown",
        "Unknown",
    )
    .unwrap()
}

fn intent(id: &str) -> Intent {
    Intent::create(
        IntentId::parse(id).unwrap(),
        &definition(),
        StartRequest::new("classic", "build it"),
        scan(),
        at(),
    )
    .unwrap()
    .0
}

const OWN: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";
const FOREIGN: &str = "018f3b2c-4d5e-7f60-8abc-def012345678";
const EXECUTION: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";
const OTHER_EXECUTION: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0001";

fn started() -> (Intent, Intent, IntentExecution) {
    let own = intent(OWN);
    let foreign = intent(FOREIGN);
    let (execution, _) =
        IntentExecution::start(IntentExecutionId::parse(EXECUTION).unwrap(), &own, at());
    (own, foreign, execution)
}

fn input(plan: &str, instructions: &str, questions: &str) -> PlanApprovalInput {
    PlanApprovalInput::new(
        PlanApprovalDocuments::new(
            plan.to_string(),
            instructions.to_string(),
            questions.to_string(),
            "construction/code-generation/code-generation-questions.md".to_string(),
        ),
        TestingSections::new(String::new(), String::new(), String::new()),
        PlanTarget::stage_level(),
        Some("b".repeat(64)),
        None,
    )
}

const MISMATCH: &str = "Code Generation approval authority does not match the active intent";

#[test]
fn the_pipeline_commands_refuse_a_foreign_intent_without_moving_state() {
    let (_, foreign, mut execution) = started();
    let definition = definition();
    let before = execution.clone();
    assert_eq!(
        execution
            .begin_single_stage_run(&foreign, &slug("code-generation"), at())
            .unwrap_err(),
        CommandError::IntentMismatch
    );
    assert_eq!(
        execution
            .require_pipeline_single(&foreign, &definition, &slug("code-generation"), None, false)
            .unwrap_err(),
        CommandError::IntentMismatch
    );
    let request = ReportRequest::new(Verdict::Forward, None, None, None, true);
    assert_eq!(
        execution
            .require_pipeline_for_report(&foreign, &definition, &request)
            .unwrap_err(),
        CommandError::IntentMismatch
    );
    let link = PipelineLinkRequest::new(
        "code-generation".to_string(),
        "aidlc-developer-agent".to_string(),
        None,
        false,
        PipelineHandoffInput::new(None, "handoff.json".to_string(), true, Ok(None)),
    );
    assert!(matches!(
        execution
            .record_pipeline_link(&foreign, &definition, &link, at())
            .unwrap_err(),
        PipelineLinkError::Command(CommandError::IntentMismatch)
    ));
    assert_eq!(execution, before, "拒否では状態が動かない");
}

#[test]
fn a_report_that_needs_no_completion_evidence_or_runs_with_pipeline_disabled_passes() {
    let (own, _, execution) = started();
    let definition = definition();
    let disabled = ReportRequest::new(Verdict::Forward, None, None, None, false);
    assert!(
        execution
            .require_pipeline_for_report(&own, &definition, &disabled)
            .is_ok()
    );
    let no_evidence = ReportRequest::new(Verdict::Rejected, None, None, None, true);
    assert!(
        execution
            .require_pipeline_for_report(&own, &definition, &no_evidence)
            .is_ok()
    );
}

#[test]
fn the_plan_approval_queries_refuse_a_foreign_intent() {
    let (own, foreign, execution) = started();
    let input = input(
        "plan",
        "instructions",
        "## Q1. Plan Approval\n\n[Answer]: \n",
    );
    assert_eq!(
        execution
            .plan_approval_evidence(&foreign, &input, "")
            .unwrap_err()
            .to_string(),
        MISMATCH
    );
    assert_eq!(
        execution
            .plan_fingerprint(&foreign, &input)
            .unwrap_err()
            .to_string(),
        MISMATCH
    );
    let approval = execution.code_generation_approval(&foreign, &input, &PlanReceipts::default());
    assert!(!approval.ok());
    assert_eq!(approval.reason(), MISMATCH);
    // 自分の intent でも、発行済みの指示が無ければ権限は解決できない。
    let approval = execution.code_generation_approval(&own, &input, &PlanReceipts::default());
    assert!(!approval.ok());
    assert!(
        approval.reason().contains("run a fresh `next`"),
        "{}",
        approval.reason()
    );
}

#[test]
fn a_plan_answer_for_another_execution_is_refused_before_any_document_is_read() {
    let (own, _, execution) = started();
    let input = input(
        "plan",
        "instructions",
        "## Q1. Plan Approval\n\n[Answer]: \n",
    );
    let origin = PlanApprovalOrigin::new(
        SpaceName::parse("default").unwrap(),
        IntentExecutionId::parse(OTHER_EXECUTION).unwrap(),
    );
    assert_eq!(
        execution
            .verify_plan_answer(
                &own,
                &input,
                &origin,
                "code-generation",
                &PlanSession::new("s".to_string()).unwrap(),
                PlanChoice::ApprovePlan,
            )
            .unwrap_err()
            .to_string(),
        "Plan Approval answer targets another execution"
    );
}

#[test]
fn a_plan_fingerprint_names_what_is_missing_once_the_directive_is_issued() {
    let (own, _, mut execution) = started();
    execution
        .issue_directive(
            &DirectivePublication::new(
                "a".repeat(64),
                "b".repeat(64),
                PublishedDirective::RunStage {
                    stage: slug("code-generation"),
                    unit: None,
                },
            ),
            at(),
        )
        .unwrap();
    let cases = [
        (
            input(
                "plan",
                "instructions",
                "## Q1. Plan Approval\n\n[Answer]: Approve Plan\n",
            ),
            "reset the Plan Approval [Answer]: to blank before regenerating its fingerprint",
        ),
        (
            input("  ", "instructions", "## Q1. Plan Approval\n\n[Answer]: \n"),
            "code-generation-plan.md is missing or empty",
        ),
        (
            input("plan", " \n", "## Q1. Plan Approval\n\n[Answer]: \n"),
            "unit-test-instructions.md is missing or empty",
        ),
        (
            input(
                "plan without a contract",
                "instructions",
                "## Q1. Plan Approval\n\n[Answer]: \n",
            ),
            "code-generation-plan.md has no valid ## Testing Contract JSON block",
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(
            execution
                .plan_fingerprint(&own, &input)
                .unwrap_err()
                .to_string(),
            expected
        );
    }
}

#[test]
fn a_task_update_for_a_stage_outside_the_plan_is_refused() {
    let (_, _, mut execution) = started();
    assert_eq!(
        execution
            .synchronize_task(&slug("nowhere"), at())
            .unwrap_err(),
        CommandError::UnknownStage("nowhere".to_string())
    );
    assert!(
        execution
            .synchronize_task(&slug("build-and-test"), at())
            .is_ok()
    );
}

#[test]
fn a_single_stage_run_that_was_never_opened_cannot_be_recorded() {
    use core_command_domain::orchestration::SingleStageRunRefusal;
    let (own, _, mut execution) = started();
    let before = execution.clone();
    assert!(matches!(
        execution
            .record_single_stage_run(&own, &slug("code-generation"), at())
            .unwrap_err(),
        SingleStageRunRefusal::Command(CommandError::SingleStageAttemptNotOpen)
    ));
    assert!(matches!(
        execution
            .record_single_stage_run(&own, &slug("nowhere"), at())
            .unwrap_err(),
        SingleStageRunRefusal::UnknownStage
    ));
    assert_eq!(execution, before);
}

#[test]
fn the_plan_approval_queries_need_an_issued_directive_even_for_the_own_intent() {
    let (own, _, execution) = started();
    let input = input(
        "plan",
        "instructions",
        "## Q1. Plan Approval\n\n[Answer]: \n",
    );
    for error in [
        execution
            .plan_approval_evidence(&own, &input, "")
            .unwrap_err(),
        execution.plan_fingerprint(&own, &input).unwrap_err(),
    ] {
        assert!(error.to_string().contains("run a fresh `next`"), "{error}");
    }
}

/// 完全コンストラクタは、別の実行に属する対話・pipeline 履歴や、記録の外を指す進捗通番を拒む。
#[test]
fn a_reconstructed_execution_refuses_history_that_belongs_to_another_execution() {
    use core_command_domain::orchestration::{
        AutonomyMode, IntentExecutionEventId, InteractionState, PendingDecisions,
        PendingSummaryDecisions, PipelineHistory, PipelineLinkCompleted, PipelineReceipt,
        PipelineRecord, PlanAppliedOperations, PromptObserved, StageSlots, Status,
    };
    let (own, _, _) = started();
    let id = IntentExecutionId::parse(EXECUTION).unwrap();
    let other = IntentExecutionId::parse(OTHER_EXECUTION).unwrap();
    let reconstruct =
        |interactions: InteractionState, history: PipelineHistory, progress_seq_nr: usize| {
            IntentExecution::new(
                history,
                PlanAppliedOperations::default(),
                None,
                None,
                interactions,
                id.clone(),
                own.id().clone(),
                StageSlots::genesis(own.stages()),
                1,
                false,
                false,
                Status::Running,
                None,
                AutonomyMode::Gated,
                None,
                None,
                progress_seq_nr,
                3,
                at(),
            )
        };
    let foreign_interactions = InteractionState::new(
        PlanAppliedOperations::default(),
        PlanAppliedOperations::default(),
        Some(PromptObserved::new(
            IntentExecutionEventId::generate(),
            other.clone(),
            "s",
            "1",
            false,
        )),
        None,
        PendingDecisions::new(Vec::new()),
        PendingSummaryDecisions::new(Vec::new()),
    );
    assert_eq!(
        reconstruct(foreign_interactions, PipelineHistory::default(), 1)
            .unwrap_err()
            .to_string(),
        "invalid intent execution: interaction state belongs to another execution"
    );
    for progress in [0, 4] {
        assert_eq!(
            reconstruct(
                InteractionState::default(),
                PipelineHistory::default(),
                progress
            )
            .unwrap_err()
            .to_string(),
            "invalid intent execution: progress sequence is outside the recorded history"
        );
    }
    let foreign_history =
        PipelineHistory::new(vec![PipelineRecord::Completed(PipelineLinkCompleted::new(
            IntentExecutionEventId::generate(),
            other,
            PipelineReceipt::new(
                "code-generation".to_string(),
                "aidlc-developer-agent".to_string(),
                None,
                false,
                1,
                1,
                None,
            )
            .unwrap(),
        ))]);
    assert_eq!(
        reconstruct(InteractionState::default(), foreign_history, 1)
            .unwrap_err()
            .to_string(),
        "invalid intent execution: pipeline history belongs to another execution"
    );
    assert!(reconstruct(InteractionState::default(), PipelineHistory::default(), 2).is_ok());
}
