//! 復号の契約 — **未知・破損した保存物を成功に丸めない**ことと、書き → 読みの往復で
//! 事実が 1 つも失われないことを、変種ごとに検査する。
//!
//! 破損は「正しい DTO を JSON に落としてから 1 か所だけ壊す」形で作る。フィールド名や
//! 変種名を手で書き写さないので、ワイヤ形式の綴りが変わっても検査が追随する。

#![allow(
    clippy::panic,
    reason = "想定外ケースの即時失敗はテストの検証手段である (house style)"
)]

use crate::orchestration::dto::{
    DtoDecodeError, HookHealthDto, HookHealthEventDto, IntentExecutionDto, IntentExecutionEventDto,
    PlanApprovalEventDto, PlanApprovalRuntimeDto,
};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::*;
use core_command_domain::workflow_definition::StageSlug;
use core_command_domain::workspace::{HookHealthTarget, HookName, IntentDirName, SpaceName};
use core_infrastructure::canon_json::{SerializationProfile, serialize, to_value};
use serde_json::Value;

const EVENT: &str = "0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002";
const EXECUTION: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";

fn at() -> DateTime<Utc> {
    "2026-09-10T01:00:00Z".parse().unwrap()
}
fn event_id() -> IntentExecutionEventId {
    IntentExecutionEventId::parse(EVENT).unwrap()
}
fn execution_id() -> IntentExecutionId {
    IntentExecutionId::parse(EXECUTION).unwrap()
}
fn slug(value: &str) -> StageSlug {
    StageSlug::parse(value).unwrap()
}
fn json<T: serde::Serialize>(dto: &T) -> Value {
    serde_json::from_str(&serialize(
        &to_value(dto).unwrap(),
        SerializationProfile::ContractCompact,
    ))
    .unwrap()
}
/// JSON の `path` (ドット区切り。配列は添字) の値を差し替える。
fn patched(mut root: Value, path: &str, replacement: Value) -> Value {
    let mut cursor = &mut root;
    for segment in path.split('.') {
        cursor = match segment.parse::<usize>() {
            Ok(index) => cursor.get_mut(index).unwrap_or_else(|| panic!("{path}")),
            // 省略されていた任意欄 (`skip_serializing_if`) は、行に現れた形で足す。
            Err(_) => cursor
                .as_object_mut()
                .unwrap_or_else(|| panic!("{path}"))
                .entry(segment)
                .or_insert(Value::Null),
        };
    }
    *cursor = replacement;
    root
}
fn text(value: &str) -> Value {
    Value::String(value.to_string())
}

fn evidence() -> PlanApprovalEvidence {
    let authority = CodeGenerationAuthority::new(
        &PlanTarget::for_unit("u2-workflow-authority").unwrap(),
        &IntentId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
        format!("sha256:{}", "a".repeat(64)),
        "WORKFLOW_STARTED:2026-09-08T01:00:00Z#1".to_string(),
        "b".repeat(64),
        2,
    )
    .unwrap();
    PlanApprovalEvidence::new(
        authority,
        format!("sha256:{}", "c".repeat(64)),
        "aidlc/spaces/default/intents/example/construction/code-generation/code-generation-questions.md".to_string(),
        "d".repeat(64),
        "e".repeat(64),
    )
    .unwrap()
}
fn decision_evidence() -> PlanDecisionEvidence {
    PlanDecisionEvidence::new(evidence(), PlanSession::new("session".to_string()).unwrap())
}
fn answer_input() -> PlanAnswerInput {
    PlanAnswerInput::new(
        PlanApprovalOrigin::new(SpaceName::default(), execution_id()),
        "code-generation".to_string(),
        decision_evidence(),
        PlanChoice::ApprovePlan,
        Some("b".repeat(64)),
    )
}
fn directive(published: PublishedDirective) -> ActiveDirective {
    ActiveDirective::new(
        3,
        IntentId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
        DirectivePublication::new("p".repeat(64), "s".repeat(64), published)
            .with_approval_operation(Some(PlanApprovalOperationId::generate()))
            .with_source_floor(Some("g".repeat(64))),
        "i".repeat(64),
        "owner".to_string(),
        1,
        2,
        4,
    )
}
fn reported(result: ReportResult, validation: Option<StageValidation>) -> Reported {
    // 採取済みソースは次の工程へ進む報告 (承認・読み飛ばし) にだけ結び付く。
    let advances = matches!(
        &result,
        ReportResult::Committed {
            transition: ReportTransition::GateApproved { .. }
                | ReportTransition::StageSkipped { .. },
            ..
        }
    );
    Reported::new(
        event_id(),
        execution_id(),
        ReportId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0003").unwrap(),
        result,
        validation,
        advances.then(|| {
            SourceBaseline::new(Some(format!("repo\ta.md\t100644\t{}\n", "0".repeat(40)))).unwrap()
        }),
    )
    .unwrap()
}
fn committed(steps: &[TransitionStep], transition: ReportTransition) -> ReportResult {
    ReportResult::Committed {
        stage: slug("intent-capture"),
        scope: "feature".to_string(),
        steps: TransitionSteps::new(steps.to_vec()).unwrap(),
        transition,
    }
}

/// 全変種の書き → 読みで事実が失われない。ここに並ぶ変種は既存のバイト固定テスト
/// (`tests.rs`) が扱っていない、実行中の観測・回答・報告の事実である。
fn round_trip_events() -> Vec<IntentExecutionEvent> {
    vec![
        IntentExecutionEvent::SingleStageRunStarted(SingleStageRunStarted::new(
            event_id(),
            execution_id(),
            slug("code-generation"),
        )),
        IntentExecutionEvent::PipelineLinkCompleted(PipelineLinkCompleted::new(
            event_id(),
            execution_id(),
            PipelineReceipt::new(
                "code-generation".to_string(),
                "u1".to_string(),
                Some("repo".to_string()),
                false,
                1,
                2,
                Some(
                    PipelineHandoff::new(
                        "handoff.md".to_string(),
                        format!("sha256:{}", "a".repeat(64)),
                        "1700000000000".to_string(),
                    )
                    .unwrap(),
                ),
            )
            .unwrap(),
        )),
        IntentExecutionEvent::TaskSynchronized(TaskSynchronized::new(
            event_id(),
            execution_id(),
            slug("domain-design"),
        )),
        IntentExecutionEvent::HealthChecked(HealthChecked::new(
            event_id(),
            execution_id(),
            HealthCheckResult::new(3, 1),
        )),
        IntentExecutionEvent::MemoryJournalsObserved(Box::new(MemoryJournalsObserved::new(
            event_id(),
            execution_id(),
            MemoryJournalSurvey::new(vec![StageMemoryJournal::new(
                slug("intent-capture"),
                MemoryJournal::new(1, 2, 3, 4),
            )]),
            EmptyMemoryStages::new(vec![slug("feasibility")]),
        ))),
        IntentExecutionEvent::DecisionRecorded(
            DecisionRecorded::new(
                event_id(),
                execution_id(),
                DecisionPrompt::new("code-generation", "Plan Approval")
                    .with_plan_approval(decision_evidence())
                    .with_options("A. Approve Plan\nB. Request Changes")
                    .with_rationale("because"),
            )
            .with_human_before(Some(event_id())),
        ),
        IntentExecutionEvent::DecisionRecorded(DecisionRecorded::new(
            event_id(),
            execution_id(),
            DecisionPrompt::new("requirements-analysis", "Summary")
                .with_summary_file("requirements-questions.md"),
        )),
        IntentExecutionEvent::PromptObserved(
            PromptObserved::new(event_id(), execution_id(), "session", "1", true)
                .with_approval_observation(Some(PlanApprovalOperationId::generate())),
        ),
        IntentExecutionEvent::PlanAnswerLogged(Box::new(PlanAnswerLogged::new(
            event_id(),
            execution_id(),
            PlanApprovalOperationId::generate(),
            answer_input(),
        ))),
        IntentExecutionEvent::AnswerRecorded(AnswerRecorded::new(
            event_id(),
            execution_id(),
            AnswerId::generate(),
            "requirements-analysis",
            "A",
            AnswerDisposition::SummaryConfirmed(
                SummaryEvidence::new("requirements-questions.md", "0".repeat(64)).unwrap(),
            ),
        )),
        IntentExecutionEvent::AnswerRecorded(AnswerRecorded::new(
            event_id(),
            execution_id(),
            AnswerId::generate(),
            "requirements-analysis",
            "A",
            AnswerDisposition::Recorded,
        )),
        IntentExecutionEvent::AnswerRecorded(AnswerRecorded::new(
            event_id(),
            execution_id(),
            AnswerId::generate(),
            "code-generation",
            "approve",
            AnswerDisposition::ApprovalGateReportOwned,
        )),
        IntentExecutionEvent::DirectiveContextInvalidated(DirectiveContextInvalidated::new(
            event_id(),
            execution_id(),
            directive(PublishedDirective::Error {
                stage: slug("intent-capture"),
            }),
        )),
        IntentExecutionEvent::DirectiveContextInvalidated(DirectiveContextInvalidated::new(
            event_id(),
            execution_id(),
            directive(PublishedDirective::LoadSteering {
                stage: slug("intent-capture"),
                part: 1,
                parts: 3,
                token: "token".to_string(),
            }),
        )),
        IntentExecutionEvent::Jumped(
            Jumped::new(
                event_id(),
                execution_id(),
                slug("intent-capture"),
                JumpDirection::Backward,
                Some(JumpObservation::new(
                    SourceBaseline::new(None).unwrap(),
                    vec![JumpArtifact::new(
                        slug("intent-capture"),
                        "intent.md".to_string(),
                        true,
                        false,
                    )],
                )),
            )
            .with_scope(Some(JumpScope::new(
                "express".to_string(),
                StageSlugSet::new(vec![slug("intent-capture"), slug("code-generation")]),
            ))),
        ),
        IntentExecutionEvent::Jumped(Jumped::new(
            event_id(),
            execution_id(),
            slug("intent-capture"),
            JumpDirection::Redo,
            None,
        )),
        IntentExecutionEvent::Reported(reported(
            committed(
                &[TransitionStep::Reject],
                ReportTransition::GateRejected {
                    feedback: Some("fix".to_string()),
                },
            ),
            Some(StageValidation::Basis("basis".to_string())),
        )),
        IntentExecutionEvent::Reported(reported(
            committed(&[TransitionStep::Revise], ReportTransition::StageRevised),
            Some(StageValidation::Warning("warning".to_string())),
        )),
        IntentExecutionEvent::Reported(reported(
            committed(
                &[TransitionStep::Skip],
                ReportTransition::StageSkipped {
                    reason: "not needed".to_string(),
                },
            ),
            None,
        )),
        IntentExecutionEvent::Reported(reported(
            committed(
                &[TransitionStep::GateStart],
                ReportTransition::GateOpened {
                    artifacts: ArtifactPaths::new(vec!["intent.md".to_string()]),
                },
            ),
            None,
        )),
        IntentExecutionEvent::Reported(reported(
            ReportResult::NoOp {
                scope: "feature".to_string(),
                no_op: ReportNoOp::AlreadyAwaiting {
                    stage: slug("intent-capture"),
                },
            },
            None,
        )),
        IntentExecutionEvent::Reported(reported(
            ReportResult::NoOp {
                scope: "feature".to_string(),
                no_op: ReportNoOp::AlreadyCompletedMovedOn {
                    stage: slug("intent-capture"),
                    current: slug("feasibility"),
                },
            },
            None,
        )),
        IntentExecutionEvent::Reported(reported(
            ReportResult::NoOp {
                scope: "feature".to_string(),
                no_op: ReportNoOp::WorkflowAlreadyCompleted {
                    stage: slug("approval-handoff"),
                },
            },
            None,
        )),
        IntentExecutionEvent::LearningsCaptured(Box::new(LearningsCaptured::new(
            event_id(),
            execution_id(),
            slug("requirements-analysis"),
            LearningProvenance::new(
                SpaceName::default(),
                IntentDirName::parse("260908-learnings").unwrap(),
            ),
            CapturedLearnings::new(vec![
                CapturedLearning::new(
                    Learning::new(
                        LearningCandidateId::parse("c1").unwrap(),
                        LearningScope::Project,
                        PracticeHeading::from_routed("Corrections"),
                        "ALWAYS record the evidence",
                        LearningSource::UserAddition,
                    ),
                    LearningDisposition::PracticeLineOnly,
                ),
                CapturedLearning::new(
                    Learning::new(
                        LearningCandidateId::parse("c2").unwrap(),
                        LearningScope::Project,
                        PracticeHeading::from_routed("Corrections"),
                        "NEVER skip the gate",
                        LearningSource::UserAddition,
                    ),
                    LearningDisposition::AuditRowOnly,
                ),
            ]),
        ))),
    ]
}

#[test]
fn every_execution_event_survives_the_write_read_round_trip() {
    for event in round_trip_events() {
        let dto = IntentExecutionEventDto::of(&event);
        let reread: IntentExecutionEventDto = serde_json::from_value(json(&dto)).unwrap();
        assert_eq!(reread, dto, "{event:?}");
        assert_eq!(reread.to_domain().unwrap(), event);
    }
}

/// 壊し方と、拒否されるべきフィールドの対。
fn corruptions() -> Vec<(IntentExecutionEvent, &'static str, Value, &'static str)> {
    let events = round_trip_events();
    let pick = |predicate: fn(&IntentExecutionEvent) -> bool| {
        events
            .iter()
            .find(|event| predicate(event))
            .unwrap()
            .clone()
    };
    let task = pick(|e| matches!(e, IntentExecutionEvent::TaskSynchronized(_)));
    let survey = pick(|e| matches!(e, IntentExecutionEvent::MemoryJournalsObserved(_)));
    let decision = pick(|e| matches!(e, IntentExecutionEvent::DecisionRecorded(_)));
    let prompt = pick(|e| matches!(e, IntentExecutionEvent::PromptObserved(_)));
    let answer = pick(|e| matches!(e, IntentExecutionEvent::AnswerRecorded(_)));
    let invalidated = pick(|e| matches!(e, IntentExecutionEvent::DirectiveContextInvalidated(_)));
    let jumped = pick(|e| matches!(e, IntentExecutionEvent::Jumped(_)));
    let reported = pick(|e| matches!(e, IntentExecutionEvent::Reported(_)));
    let learnings = pick(|e| matches!(e, IntentExecutionEvent::LearningsCaptured(_)));
    let no_op = |predicate: fn(&ReportNoOp) -> bool| {
        events
            .iter()
            .find(|e| match e {
                IntentExecutionEvent::Reported(r) => match r.result() {
                    ReportResult::NoOp { no_op, .. } => predicate(no_op),
                    ReportResult::Committed { .. } => false,
                },
                _ => false,
            })
            .unwrap()
            .clone()
    };
    let no_op_awaiting = no_op(|n| matches!(n, ReportNoOp::AlreadyAwaiting { .. }));
    let no_op_moved = no_op(|n| matches!(n, ReportNoOp::AlreadyCompletedMovedOn { .. }));
    let no_op_completed = no_op(|n| matches!(n, ReportNoOp::WorkflowAlreadyCompleted { .. }));
    let steering = events
        .iter()
        .filter(|e| matches!(e, IntentExecutionEvent::DirectiveContextInvalidated(_)))
        .nth(1)
        .unwrap()
        .clone();
    let run_stage =
        IntentExecutionEvent::DirectiveContextInvalidated(DirectiveContextInvalidated::new(
            event_id(),
            execution_id(),
            directive(PublishedDirective::RunStage {
                stage: slug("intent-capture"),
                unit: Some("u1".to_string()),
            }),
        ));
    vec![
        (
            task.clone(),
            "TaskSynchronized.id",
            text("not-a-uuid"),
            "id",
        ),
        (
            task.clone(),
            "TaskSynchronized.aggregate_id",
            text("not-a-uuid"),
            "aggregate_id",
        ),
        (task, "TaskSynchronized.stage", text("Not A Slug"), "stage"),
        (
            survey.clone(),
            "MemoryJournalsObserved.survey.0.stage",
            text("Not A Slug"),
            "survey.stage",
        ),
        (
            survey.clone(),
            "MemoryJournalsObserved.empty_stages.0",
            text("Not A Slug"),
            "empty_stages",
        ),
        (survey.clone(), "MemoryJournalsObserved.id", text("x"), "id"),
        (
            survey,
            "MemoryJournalsObserved.aggregate_id",
            text("x"),
            "aggregate_id",
        ),
        (decision.clone(), "DecisionRecorded.id", text("x"), "id"),
        (
            decision.clone(),
            "DecisionRecorded.aggregate_id",
            text("x"),
            "aggregate_id",
        ),
        (
            decision,
            "DecisionRecorded.human_before",
            text("x"),
            "human_before",
        ),
        (prompt.clone(), "PromptObserved.id", text("x"), "id"),
        (
            prompt.clone(),
            "PromptObserved.aggregate_id",
            text("x"),
            "aggregate_id",
        ),
        (
            prompt,
            "PromptObserved.approval_observation_id",
            text("x"),
            "approval_observation_id",
        ),
        (answer.clone(), "AnswerRecorded.id", text("x"), "id"),
        (
            answer.clone(),
            "AnswerRecorded.aggregate_id",
            text("x"),
            "aggregate_id",
        ),
        (
            answer.clone(),
            "AnswerRecorded.answer_id",
            text("x"),
            "answer_id",
        ),
        (
            answer.clone(),
            "AnswerRecorded.disposition",
            text("unknown"),
            "disposition",
        ),
        (
            answer.clone(),
            "AnswerRecorded.summary",
            Value::Null,
            "summary",
        ),
        (
            answer,
            "AnswerRecorded.summary.questions_sha256",
            text("short"),
            "summary",
        ),
        (
            invalidated,
            "DirectiveContextInvalidated.directive.directive.Error.stage",
            text("Not A Slug"),
            "stage",
        ),
        (
            jumped.clone(),
            "Jumped.scope.executes.0",
            text("Not A Slug"),
            "scope.executes",
        ),
        (
            jumped.clone(),
            "Jumped.direction",
            text("sideways"),
            "direction",
        ),
        (
            jumped,
            "Jumped.observation.artifacts.0.stage",
            text("Not A Slug"),
            "stage",
        ),
        (
            reported.clone(),
            "Reported.result.Committed.steps.0",
            text("Teleport"),
            "steps",
        ),
        (
            no_op_awaiting,
            "Reported.result.NoOp.no_op.AlreadyAwaiting.stage",
            text("Not A Slug"),
            "stage",
        ),
        (
            no_op_moved.clone(),
            "Reported.result.NoOp.no_op.AlreadyCompletedMovedOn.stage",
            text("Not A Slug"),
            "stage",
        ),
        (
            no_op_moved,
            "Reported.result.NoOp.no_op.AlreadyCompletedMovedOn.current",
            text("Not A Slug"),
            "stage",
        ),
        (
            no_op_completed,
            "Reported.result.NoOp.no_op.WorkflowAlreadyCompleted.stage",
            text("Not A Slug"),
            "stage",
        ),
        (
            steering,
            "DirectiveContextInvalidated.directive.directive.LoadSteering.stage",
            text("Not A Slug"),
            "stage",
        ),
        (
            run_stage,
            "DirectiveContextInvalidated.directive.directive.RunStage.stage",
            text("Not A Slug"),
            "stage",
        ),
        (
            learnings.clone(),
            "LearningsCaptured.stage",
            text("Not A Slug"),
            "stage",
        ),
        (
            learnings.clone(),
            "LearningsCaptured.space",
            text("Not A Space!"),
            "space",
        ),
        (
            learnings.clone(),
            "LearningsCaptured.intent",
            text("Not A Dir!"),
            "intent",
        ),
        (learnings.clone(), "LearningsCaptured.id", text("x"), "id"),
        (
            learnings.clone(),
            "LearningsCaptured.aggregate_id",
            text("x"),
            "aggregate_id",
        ),
        (
            reported.clone(),
            "Reported.result.Committed.stage",
            text("Not A Slug"),
            "stage",
        ),
        (reported.clone(), "Reported.id", text("x"), "id"),
        (
            reported.clone(),
            "Reported.aggregate_id",
            text("x"),
            "aggregate_id",
        ),
        (reported, "Reported.report_id", text("x"), "report_id"),
        (
            learnings.clone(),
            "LearningsCaptured.learnings.0.disposition",
            text("unknown"),
            "learnings.disposition",
        ),
        (
            learnings,
            "LearningsCaptured.learnings.0.candidate_id",
            text(""),
            "learnings.candidate_id",
        ),
    ]
}

#[test]
fn a_corrupt_field_is_rejected_by_name_instead_of_being_rounded_to_success() {
    for (event, path, replacement, field) in corruptions() {
        let dto: IntentExecutionEventDto = serde_json::from_value(patched(
            json(&IntentExecutionEventDto::of(&event)),
            path,
            replacement,
        ))
        .unwrap_or_else(|error| panic!("{path}: {error}"));
        match dto.to_domain() {
            Err(DtoDecodeError::Malformed { field: found, .. }) => {
                assert_eq!(found, field, "{path}");
            }
            other => panic!("{path}: {other:?}"),
        }
    }
}

#[test]
fn invariant_breaking_rows_are_rejected_as_invariant_violations() {
    let events = round_trip_events();
    let reported = events
        .iter()
        .find(|e| matches!(e, IntentExecutionEvent::Reported(_)))
        .unwrap();
    // 根拠と警告の両方を持つ報告は、片方を捨てて読まない。
    let both = patched(
        json(&IntentExecutionEventDto::of(reported)),
        "Reported.validation_warning",
        text("warning"),
    );
    let dto: IntentExecutionEventDto = serde_json::from_value(both).unwrap();
    assert_eq!(dto.to_domain(), Err(DtoDecodeError::InvariantViolation));
    // 遷移が受け付けない手順列は、報告として組み上げない。
    let mismatched = patched(
        json(&IntentExecutionEventDto::of(reported)),
        "Reported.result.Committed.steps.0",
        text("Approve"),
    );
    let dto: IntentExecutionEventDto = serde_json::from_value(mismatched).unwrap();
    assert_eq!(dto.to_domain(), Err(DtoDecodeError::InvariantViolation));

    // 計画承認の根拠と要約ファイルを同時に持つ決定は、不変条件違反である。
    let decision = events
        .iter()
        .find(|e| matches!(e, IntentExecutionEvent::DecisionRecorded(_)))
        .unwrap();
    let both = patched(
        json(&IntentExecutionEventDto::of(decision)),
        "DecisionRecorded.summary_file",
        text("questions.md"),
    );
    let dto: IntentExecutionEventDto = serde_json::from_value(both).unwrap();
    assert_eq!(dto.to_domain(), Err(DtoDecodeError::InvariantViolation));

    // 単位名の文法外の対象は、根拠として組み上げない。
    let bad_unit = patched(
        json(&IntentExecutionEventDto::of(decision)),
        "DecisionRecorded.plan_approval.evidence.authority.unit",
        text("Not A Unit!"),
    );
    let dto: IntentExecutionEventDto = serde_json::from_value(bad_unit).unwrap();
    assert_eq!(dto.to_domain(), Err(DtoDecodeError::InvariantViolation));
}

fn genesis_snapshot() -> Value {
    serde_json::from_str(super::GENESIS_SNAPSHOT).unwrap()
}
fn decode_snapshot(value: Value) -> Result<IntentExecution, DtoDecodeError> {
    serde_json::from_value::<IntentExecutionDto>(value)
        .unwrap()
        .to_domain()
}

#[test]
fn the_run_floor_restores_every_known_boundary_and_rejects_unknown_ones() {
    for (kind, counter) in [
        ("STAGE_STARTED", "stage_started"),
        ("STAGE_JUMPED", "stage_jumped"),
        ("GATE_REJECTED", "gate_rejected"),
    ] {
        let restored = decode_snapshot(patched(
            patched(
                genesis_snapshot(),
                "code_generation_run_floor.latest.kind",
                text(kind),
            ),
            &format!("code_generation_run_floor.{counter}"),
            Value::from(1),
        ))
        .unwrap_or_else(|error| panic!("{kind}: {error}"));
        let floor = restored.code_generation_run_floor().unwrap();
        assert_eq!(floor.latest().map(|(kind, _)| kind.as_str()), Some(kind));
    }
    assert_eq!(
        decode_snapshot(patched(
            genesis_snapshot(),
            "code_generation_run_floor.latest.kind",
            text("SESSION_STARTED"),
        )),
        Err(DtoDecodeError::malformed(
            "run_floor.kind",
            "SESSION_STARTED"
        ))
    );
}

#[test]
fn interaction_identifiers_outside_their_grammar_are_rejected_by_name() {
    let mut base = genesis_snapshot();
    base["interactions"]["plan_answers"] = Value::Array(vec![text("x")]);
    assert_eq!(
        decode_snapshot(base),
        Err(DtoDecodeError::malformed("plan_answer", "x"))
    );
    let mut base = genesis_snapshot();
    base["interactions"]["approval_observations"] = Value::Array(vec![text("x")]);
    assert_eq!(
        decode_snapshot(base),
        Err(DtoDecodeError::malformed("approval_observations", "x"))
    );
    assert_eq!(
        decode_snapshot(patched(
            genesis_snapshot(),
            "interactions.consumed_human",
            text("x"),
        )),
        Err(DtoDecodeError::malformed(
            "interactions.consumed_human",
            "x"
        ))
    );
    // 列の長さが食い違う行は、足りない列を捏造せず不変条件違反にする。
    assert_eq!(
        decode_snapshot(patched(
            genesis_snapshot(),
            "practices_affirmed",
            Value::Array(vec![Value::Bool(false)]),
        )),
        Err(DtoDecodeError::InvariantViolation)
    );
}

#[test]
fn a_recorded_directive_survives_the_snapshot_round_trip() {
    for published in [
        PublishedDirective::Error {
            stage: slug("intent-capture"),
        },
        PublishedDirective::LoadSteering {
            stage: slug("intent-capture"),
            part: 2,
            parts: 3,
            token: "token".to_string(),
        },
    ] {
        let event = IntentExecutionEvent::DirectiveContextInvalidated(
            DirectiveContextInvalidated::new(event_id(), execution_id(), directive(published)),
        );
        let dto = IntentExecutionEventDto::of(&event);
        assert_eq!(dto.to_domain().unwrap(), event);
    }
}

// ---- 共有承認のイベント ---------------------------------------------------------------

fn plan_approval_events() -> Vec<PlanApprovalEvent> {
    let id = || PlanApprovalEventId::generate();
    let aggregate = PlanApprovalRuntimeId::Workspace;
    let challenge = PlanChallenge::issue(
        evidence(),
        PlanSession::new("session".to_string()).unwrap(),
        ["Approve Plan".to_string(), "Request Changes".to_string()],
        false,
    );
    let (mut runtime, created) = PlanApprovalRuntime::create(at());
    let issuance = PlanApprovalOperationId::generate();
    let issued = runtime
        .issue_challenge(issuance.clone(), challenge.clone(), at())
        .unwrap();
    let observed = runtime
        .observe_response(
            PlanApprovalOperationId::generate(),
            &issuance,
            challenge.session(),
            "1",
            at(),
        )
        .unwrap();
    // 応答の固定は回答の受領と排他なので、同じ観測から分岐した別の履歴で作る。
    let mut branched = runtime.clone();
    let prepared = branched
        .prepare_response(
            PlanApprovalOperationId::generate(),
            PlanApprovalOrigin::new(SpaceName::default(), execution_id()),
            challenge.session().clone(),
            "1",
            at(),
        )
        .unwrap();
    let answer_id = PlanApprovalOperationId::generate();
    let recorded = runtime
        .record_answer(answer_id.clone(), answer_input(), at())
        .unwrap();
    vec![
        created,
        issued,
        observed,
        prepared,
        recorded,
        PlanApprovalEvent::AnswerCompleted(PlanAnswerCompleted::new(
            id(),
            aggregate,
            answer_id.clone(),
        )),
        PlanApprovalEvent::AnswerAborted(PlanAnswerAborted::new(id(), aggregate, answer_id)),
        PlanApprovalEvent::GenerationCertified(PlanGenerationCertified::new(
            id(),
            aggregate,
            PlanApprovalOperationId::generate(),
        )),
        PlanApprovalEvent::GenerationRevoked(PlanGenerationRevoked::new(
            id(),
            aggregate,
            PlanApprovalOperationId::generate(),
        )),
        PlanApprovalEvent::InvalidationResolved(PlanInvalidationResolved::new(
            id(),
            aggregate,
            PlanApprovalOperationId::generate(),
            true,
        )),
    ]
}

#[test]
fn every_plan_approval_event_survives_the_write_read_round_trip() {
    for event in plan_approval_events() {
        let dto = PlanApprovalEventDto::of(&event);
        let reread: PlanApprovalEventDto = serde_json::from_value(json(&dto)).unwrap();
        assert_eq!(reread.to_domain().unwrap(), event, "{event:?}");
    }
}

#[test]
fn plan_approval_events_reject_identifiers_and_choices_outside_the_closed_sets() {
    let events = plan_approval_events();
    let created = &events[0];
    let observed = &events[2];
    let cases: [(&PlanApprovalEvent, &str, &str, &'static str); 11] = [
        (created, "id", "x", "event_id"),
        (created, "aggregate_id", "other", "runtime_id"),
        (observed, "payload.value.choice", "Maybe", "choice"),
        (observed, "payload.value.occurrence_id", "x", "operation_id"),
        (
            observed,
            "payload.value.observation_id",
            "x",
            "operation_id",
        ),
        (observed, "payload.value.session", "   ", "session"),
        (
            &events[5],
            "payload.value.operation_id",
            "x",
            "operation_id",
        ),
        (
            &events[6],
            "payload.value.operation_id",
            "x",
            "operation_id",
        ),
        (
            &events[7],
            "payload.value.operation_id",
            "x",
            "operation_id",
        ),
        (
            &events[8],
            "payload.value.operation_id",
            "x",
            "operation_id",
        ),
        (
            &events[9],
            "payload.value.operation_id",
            "x",
            "operation_id",
        ),
    ];
    for (event, path, replacement, field) in cases {
        let dto: PlanApprovalEventDto = serde_json::from_value(patched(
            json(&PlanApprovalEventDto::of(event)),
            path,
            text(replacement),
        ))
        .unwrap();
        assert_eq!(
            dto.to_domain(),
            Err(DtoDecodeError::malformed(field, replacement)),
            "{path}"
        );
    }
    // 「Request Changes」は閉集合の内側であり、そのまま戻る。
    let dto: PlanApprovalEventDto = serde_json::from_value(patched(
        json(&PlanApprovalEventDto::of(observed)),
        "payload.value.choice",
        text("Request Changes"),
    ))
    .unwrap();
    match dto.to_domain().unwrap() {
        PlanApprovalEvent::ResponseObserved(value) => {
            assert_eq!(*value.choice(), Some(PlanChoice::RequestChanges));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_challenge_whose_stored_id_disagrees_with_its_evidence_is_rejected() {
    let (mut runtime, _) = PlanApprovalRuntime::create(at());
    let challenge = PlanChallenge::issue(
        evidence(),
        PlanSession::new("session".to_string()).unwrap(),
        ["Approve Plan".to_string(), "Request Changes".to_string()],
        false,
    );
    runtime
        .issue_challenge(PlanApprovalOperationId::generate(), challenge, at())
        .unwrap();
    let dto = PlanApprovalRuntimeDto::of(&runtime);
    assert_eq!(dto.to_domain().unwrap(), runtime);
    let mut broken = json(&dto);
    let challenges = broken["challenges"].as_array_mut().unwrap();
    challenges[0]["challenge"]["id"] = text("h".repeat(64).as_str());
    let dto: PlanApprovalRuntimeDto = serde_json::from_value(broken).unwrap();
    assert_eq!(dto.to_domain(), Err(DtoDecodeError::InvariantViolation));
}

// ---- フック健全性 ---------------------------------------------------------------------

fn hook_health_events() -> Vec<core_command_domain::workspace::HookHealthEvent> {
    use core_command_domain::workspace::HookHealth;
    let target = HookHealthTarget::new(SpaceName::default(), None);
    let hook = HookName::parse("write-audit-log").unwrap();
    let (mut health, started) = HookHealth::start(target.clone(), hook.clone(), at()).unwrap();
    let heartbeat = health
        .observe_heartbeat(at() + chrono::Duration::seconds(1))
        .unwrap();
    let dropped = health
        .record_drop("EISDIR", at() + chrono::Duration::seconds(2))
        .unwrap();
    let (_, first_drop) = HookHealth::start_with_drop(target, hook, "EACCES", at()).unwrap();
    vec![started, heartbeat, dropped, first_drop]
}

#[test]
fn every_hook_health_event_survives_the_write_read_round_trip() {
    for event in hook_health_events() {
        let dto = HookHealthEventDto::of(&event);
        assert_eq!(dto.to_domain().unwrap(), event, "{event:?}");
    }
}

#[test]
fn hook_health_events_missing_their_material_are_rejected() {
    let events = hook_health_events();
    let cases: [(usize, &str, Value, &str); 12] = [
        (3, "target", text("nowhere"), "invalid hook health identity"),
        (3, "hook", text("Not A Hook!"), "invalid hook name"),
        (3, "reason", text("   "), "invalid hook health history"),
        (0, "target", Value::Null, "missing target"),
        (0, "hook", Value::Null, "missing hook"),
        (0, "target", text("nowhere"), "invalid hook health identity"),
        (0, "hook", text("Not A Hook!"), "invalid hook name"),
        (2, "reason", Value::Null, "missing reason"),
        (3, "target", Value::Null, "missing target"),
        (3, "hook", Value::Null, "missing hook"),
        (3, "reason", Value::Null, "missing reason"),
        (3, "kind", text("unknown"), "unknown hook health event"),
    ];
    for (index, path, replacement, expected) in cases {
        let dto: HookHealthEventDto = serde_json::from_value(patched(
            json(&HookHealthEventDto::of(&events[index])),
            path,
            replacement,
        ))
        .unwrap();
        match dto.to_domain() {
            Err(DtoDecodeError::Malformed { field, found }) => {
                assert_eq!(field, "hook_health", "{index}/{path}");
                assert!(found.contains(expected), "{index}/{path}: {found}");
            }
            other => panic!("{index}/{path}: {other:?}"),
        }
    }
}

#[test]
fn a_hook_health_snapshot_round_trips_and_rejects_an_impossible_drop_summary() {
    use core_command_domain::workspace::HookHealth;
    let (mut health, _) = HookHealth::start(
        HookHealthTarget::new(SpaceName::default(), None),
        HookName::parse("write-audit-log").unwrap(),
        at(),
    )
    .unwrap();
    health.record_drop("EISDIR", at()).unwrap();
    let dto = HookHealthDto::of(&health);
    assert_eq!(dto.to_domain().unwrap(), health);
    let dto: HookHealthDto =
        serde_json::from_value(patched(json(&dto), "latest_drop", Value::Null)).unwrap();
    assert!(matches!(
        dto.to_domain(),
        Err(DtoDecodeError::Malformed {
            field: "hook_health",
            ..
        })
    ));
}

// ---- intent の要求 ---------------------------------------------------------------------

#[test]
fn a_named_record_and_a_source_baseline_survive_the_intent_snapshot_round_trip() {
    use core_command_domain::orchestration::{Created, Intent, StartRequest, WorkspaceScan};
    use core_command_domain::workflow_definition::{
        BrownfieldGreenfield, DefinitionRevision, WorkflowDefinitionId,
    };
    let request = StartRequest::new("classic", "contract")
        .with_record_name(IntentRecordName::new(
            IntentDirName::parse("260910-named").unwrap(),
            "named",
        ))
        .with_source_baseline(
            SourceBaseline::new(Some(format!("repo\ta.md\t100644\t{}\n", "0".repeat(40)))).unwrap(),
        );
    let intent = Intent::from((
        Created::new(
            IntentEventId::generate(),
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            request,
            super::stages(),
            WorkspaceScan::new(
                BrownfieldGreenfield::Greenfield,
                "Unknown",
                "Unknown",
                "Unknown",
            )
            .unwrap(),
        ),
        at(),
    ));
    let dto = crate::orchestration::dto::IntentDto::of(&intent);
    let restored = dto.to_domain().unwrap();
    assert_eq!(restored.record_name(), intent.record_name());
    assert_eq!(restored.source_baseline(), intent.source_baseline());

    // 記録名の文法外・一覧の文法外はどちらも復号を止める。
    let broken: crate::orchestration::dto::IntentDto = serde_json::from_value(patched(
        json(&dto),
        "start_request.record_name.directory",
        text("Not A Dir!"),
    ))
    .unwrap();
    assert_eq!(
        broken.to_domain(),
        Err(DtoDecodeError::malformed(
            "record_name.directory",
            "Not A Dir!"
        ))
    );
    let broken: crate::orchestration::dto::IntentDto = serde_json::from_value(patched(
        json(&dto),
        "start_request.source_baseline.listing",
        text("no trailing newline"),
    ))
    .unwrap();
    assert_eq!(broken.to_domain(), Err(DtoDecodeError::InvariantViolation));
}

#[test]
fn a_greenfield_adjusted_stage_must_have_been_folded_to_skip() {
    let dto = crate::orchestration::dto::IntentDto::of(&super::intent());
    let row = json(&dto);
    let last = row["stages"].as_array().unwrap().len() - 1;
    // 末尾のステージを「greenfield 調整済み」と名乗らせるが、plan_action は EXECUTE のまま。
    let row = patched(
        row,
        &format!("stages.{last}.greenfield_adjusted"),
        Value::Bool(true),
    );
    let broken: crate::orchestration::dto::IntentDto = serde_json::from_value(row.clone()).unwrap();
    assert_eq!(broken.to_domain(), Err(DtoDecodeError::InvariantViolation));
    // SKIP に畳まれていれば調整済みの事実として戻る。
    let row = patched(row, &format!("stages.{last}.plan_action"), text("Skip"));
    let folded: crate::orchestration::dto::IntentDto = serde_json::from_value(row).unwrap();
    let restored = folded.to_domain().unwrap();
    let adjusted = restored.stages().fold_left(0, |count, entry| {
        count + usize::from(entry.is_greenfield_adjusted())
    });
    assert_eq!(adjusted, 1);
}

// ---- 実行スナップショットの対話状態 ------------------------------------------------------

#[test]
fn pending_and_summary_prompts_survive_the_snapshot_round_trip() {
    use core_command_domain::orchestration::{
        AutonomyMode, IntentExecution, InteractionState, PendingDecisions, PendingSummaryDecisions,
        PipelineHistory, PlanAppliedOperations, ReviewAttempt, Status,
    };
    let (started, _) = IntentExecution::start(execution_id(), &super::intent(), at());
    let pending = DecisionRecorded::new(
        event_id(),
        execution_id(),
        DecisionPrompt::new("intent-capture", "Approve?").with_options("A. Yes"),
    );
    let summary = DecisionRecorded::new(
        event_id(),
        execution_id(),
        DecisionPrompt::new("intent-capture", "Summary").with_summary_file("questions.md"),
    );
    let interactions = InteractionState::new(
        PlanAppliedOperations::default(),
        PlanAppliedOperations::default(),
        Some(PromptObserved::new(
            event_id(),
            execution_id(),
            "s",
            "1",
            false,
        )),
        Some(event_id()),
        PendingDecisions::new(vec![pending]),
        PendingSummaryDecisions::new(vec![summary]),
    );
    let execution = IntentExecution::new(
        PipelineHistory::default(),
        PlanAppliedOperations::default(),
        None,
        None,
        interactions,
        started.id().clone(),
        started.intent_id().clone(),
        super::synthetic_slots(
            &started,
            &[
                ReviewAttempt::default(),
                ReviewAttempt::default(),
                ReviewAttempt::default(),
            ],
        ),
        1,
        false,
        false,
        Status::Running,
        None,
        AutonomyMode::Gated,
        None,
        None,
        1,
        1,
        at(),
    )
    .unwrap();
    let dto = IntentExecutionDto::of(&execution);
    let row = json(&dto);
    assert_eq!(row["interactions"]["pending"].as_array().unwrap().len(), 1);
    assert_eq!(
        row["interactions"]["summary_prompts"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let reread: IntentExecutionDto = serde_json::from_value(row).unwrap();
    assert_eq!(reread.to_domain().unwrap(), execution);
}

#[test]
fn rows_written_before_optional_columns_existed_are_read_with_their_defaults() {
    // 対話状態と実践確定の列が無い行は「未記録」であって破損ではない。
    let mut row = genesis_snapshot();
    let object = row.as_object_mut().unwrap();
    object.remove("interactions");
    object.remove("practices_affirmed");
    object.remove("memory_empty_reported");
    let restored = decode_snapshot(row).unwrap();
    let (expected, _) = IntentExecution::start(execution_id(), &super::intent(), super::at());
    assert_eq!(restored, expected);
}

#[test]
fn a_requested_generation_survives_the_plan_approval_round_trip() {
    let receipt = PlanApprovalReceipt::new(
        decision_evidence(),
        format!("sha256:{}", "c".repeat(64)),
        "b".repeat(64),
        PlanGenerationStatus::Generation,
    )
    .unwrap();
    let generation = PlanGeneration::new(
        PlanApprovalOperationId::generate(),
        receipt,
        PlanGenerationState::Pending,
    )
    .unwrap();
    let event = PlanApprovalEvent::GenerationRequested(Box::new(PlanGenerationRequested::new(
        PlanApprovalEventId::generate(),
        PlanApprovalRuntimeId::Workspace,
        generation,
    )));
    let dto = PlanApprovalEventDto::of(&event);
    assert_eq!(dto.to_domain().unwrap(), event);
}
