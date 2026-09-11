//! 実行イベント DTO（読む側）の契約 — 自分の綴り（`of`）で書いた行を自分で読み戻せること、
//! そして壊れた行を成功に丸めず拒否すること。
// テストコードでは unwrap / expect / panic を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::needless_pass_by_value,
    clippy::type_complexity
)]
// ジャーナル行の payload は**契約 JSON ではなくワイヤ形式そのもの**であり、行を用意する
// テストは本家のシリアライザと同じ素の serde で書く (BR1.7 の射程外)。
#![allow(clippy::disallowed_methods)]
mod support;

/// 合成レビュー証跡（ワークスペース共有の試験装置。実ワークフローの受領には使わない）。
#[path = "../../../../tests/support/review_fixture.rs"]
mod review_fixture;

use core_command_domain::orchestration::{
    ActiveDirective, AnswerDisposition, AnswerId, AnswerRecorded, ArtifactPaths, AutonomyMode,
    AutonomyModeSet, CapturedLearning, CapturedLearnings, CodeGenerationAuthority, CommandFailed,
    CommandFailure, DecisionPrompt, DecisionRecorded, DirectiveContextInvalidated, DirectiveIssued,
    DirectivePublication, EmptyMemoryStages, GateApproved, GateOpened, GateRejected,
    HealthCheckResult, HealthChecked, IntentExecutionEvent, IntentExecutionEventId, JumpArtifact,
    JumpDirection, JumpObservation, JumpScope, Jumped, Learning, LearningCandidateId,
    LearningDisposition, LearningProvenance, LearningScope, LearningSource, LearningsCaptured,
    MemoryJournal, MemoryJournalSurvey, MemoryJournalsObserved, Parked, PipelineHandoff,
    PipelineLinkCompleted, PipelineReceipt, PlanAnswerInput, PlanAnswerLogged,
    PlanApprovalEvidence, PlanApprovalOperationId, PlanApprovalOrigin, PlanChoice,
    PlanDecisionEvidence, PlanSession, PlanTarget, PracticeHeading, PracticesAffirmed,
    PromptObserved, PublishedDirective, Recomposed, ReportId, ReportNoOp, ReportResult,
    ReportTransition, Reported, ReviewCompleted, ReviewRequested, ReviewVerdict,
    SingleStageRunCommitted, SingleStageRunStarted, SkeletonStance, SkeletonStanceRecorded,
    SourceBaseline, StageMemoryJournal, StageRevised, StageSkipped, StageSlugSet, StageValidation,
    Started, SummaryEvidence, TaskSynchronized, TransitionStep, TransitionSteps, Unparked,
};
use core_command_domain::workflow_definition::StageSlug;
use core_command_domain::workspace::{
    IntentDirName, PromotedSection, PromotedSections, RuleLines, SpaceName,
};
use core_read_model_updater::orchestration::{DtoDecodeError, IntentExecutionEventDto};
use serde_json::Value;

fn ev() -> IntentExecutionEventId {
    IntentExecutionEventId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff1234").unwrap()
}
fn slug(value: &str) -> StageSlug {
    StageSlug::parse(value).unwrap()
}
/// ソース一覧の 1 行（`repo\tpath\tmode\thash\n`）。
fn listing() -> String {
    format!("-\tsrc/lib.rs\t100644\t{}\n", sha('0'))
}
fn sha(fill: char) -> String {
    fill.to_string().repeat(64)
}
fn evidence() -> PlanApprovalEvidence {
    PlanApprovalEvidence::new(
        CodeGenerationAuthority::new(
            &PlanTarget::for_unit("u2-workflow-authority").unwrap(),
            support::intent().id(),
            format!("sha256:{}", sha('a')),
            "WORKFLOW_STARTED:2026-09-08T01:00:00Z#1".to_string(),
            sha('b'),
            2,
        )
        .unwrap(),
        format!("sha256:{}", sha('c')),
        "code-generation-questions.md".to_string(),
        sha('d'),
        sha('e'),
    )
    .unwrap()
}
fn directive(directive: PublishedDirective) -> ActiveDirective {
    ActiveDirective::new(
        3,
        support::intent().id().clone(),
        DirectivePublication::new(sha('1'), sha('2'), directive)
            .with_source_floor(Some("sha256:listing".to_string()))
            .with_approval_operation(Some(
                PlanApprovalOperationId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0777").unwrap(),
            )),
        sha('3'),
        "sessionless:1111111111111111".to_string(),
        4,
        5,
        6,
    )
}
fn observation() -> JumpObservation {
    JumpObservation::new(
        SourceBaseline::new(Some(listing())).unwrap(),
        vec![JumpArtifact::new(
            slug("intent-capture"),
            "ideation/intent-capture/intent.md".to_string(),
            true,
            false,
        )],
    )
}
fn committed(transition: ReportTransition, steps: Vec<TransitionStep>) -> ReportResult {
    ReportResult::Committed {
        stage: slug("intent-capture"),
        scope: "classic".to_string(),
        steps: TransitionSteps::new(steps).unwrap(),
        transition,
    }
}
fn reported(
    result: ReportResult,
    validation: Option<StageValidation>,
    baseline: Option<SourceBaseline>,
) -> IntentExecutionEvent {
    IntentExecutionEvent::Reported(
        Reported::new(
            ev(),
            support::execution_id(),
            ReportId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0555").unwrap(),
            result,
            validation,
            baseline,
        )
        .unwrap(),
    )
}

/// 読む側の `of` が運ぶ全変種（`to_domain` の逆写像として検収する対象）。
fn events() -> Vec<(&'static str, IntentExecutionEvent)> {
    let agg = support::execution_id;
    vec![
        (
            "SingleStageRunStarted",
            IntentExecutionEvent::SingleStageRunStarted(SingleStageRunStarted::new(
                ev(),
                agg(),
                slug("contract-design"),
            )),
        ),
        (
            "PipelineLinkCompleted",
            IntentExecutionEvent::PipelineLinkCompleted(PipelineLinkCompleted::new(
                ev(),
                agg(),
                PipelineReceipt::new(
                    "reverse-engineering".to_string(),
                    "aidlc-architect-agent".to_string(),
                    Some("modules/app".to_string()),
                    true,
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
            )),
        ),
        (
            "TaskSynchronized",
            IntentExecutionEvent::TaskSynchronized(TaskSynchronized::new(
                ev(),
                agg(),
                slug("scope-definition"),
            )),
        ),
        (
            "HealthChecked",
            IntentExecutionEvent::HealthChecked(HealthChecked::new(
                ev(),
                agg(),
                HealthCheckResult::new(3, 1),
            )),
        ),
        (
            "LearningsCaptured",
            IntentExecutionEvent::LearningsCaptured(Box::new(LearningsCaptured::new(
                ev(),
                agg(),
                slug("intent-capture"),
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
                    CapturedLearning::new(
                        Learning::new(
                            LearningCandidateId::parse("c3").unwrap(),
                            LearningScope::Team,
                            PracticeHeading::from_routed("Deployment"),
                            "ALWAYS tag releases",
                            LearningSource::Orchestrator,
                        ),
                        LearningDisposition::AuditRowOnly,
                    ),
                ]),
            ))),
        ),
        (
            "MemoryJournalsObserved",
            IntentExecutionEvent::MemoryJournalsObserved(Box::new(MemoryJournalsObserved::new(
                ev(),
                agg(),
                MemoryJournalSurvey::new(vec![
                    StageMemoryJournal::new(slug("intent-capture"), MemoryJournal::new(1, 2, 3, 4)),
                    StageMemoryJournal::new(
                        slug("scope-definition"),
                        MemoryJournal::new(0, 0, 0, 0),
                    ),
                ]),
                EmptyMemoryStages::new(vec![slug("scope-definition")]),
            ))),
        ),
        (
            "CommandFailed",
            IntentExecutionEvent::CommandFailed(CommandFailed::new(
                ev(),
                agg(),
                CommandFailure::new(
                    "aidlc-utility".to_string(),
                    "aidlc-utility set-status --stage unknown-stage".to_string(),
                    "Unknown stage: unknown-stage".to_string(),
                ),
            )),
        ),
        (
            "DecisionRecorded",
            IntentExecutionEvent::DecisionRecorded(
                DecisionRecorded::new(
                    ev(),
                    agg(),
                    DecisionPrompt::new("code-generation", "Plan Approval")
                        .with_options("A. Approve Plan,B. Request Changes")
                        .with_rationale("承認前に計画を確認する")
                        .with_plan_approval(PlanDecisionEvidence::new(
                            evidence(),
                            PlanSession::new("session-1".to_string()).unwrap(),
                        )),
                )
                .with_human_before(Some(ev())),
            ),
        ),
        (
            "DecisionRecorded(summary)",
            IntentExecutionEvent::DecisionRecorded(DecisionRecorded::new(
                ev(),
                agg(),
                DecisionPrompt::new("requirements-analysis", "Consolidated summary")
                    .with_summary_file("requirements-questions.md"),
            )),
        ),
        (
            "PromptObserved",
            IntentExecutionEvent::PromptObserved(
                PromptObserved::new(ev(), agg(), "session-1", "A", true).with_approval_observation(
                    Some(
                        PlanApprovalOperationId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0888")
                            .unwrap(),
                    ),
                ),
            ),
        ),
        (
            "PlanAnswerLogged",
            IntentExecutionEvent::PlanAnswerLogged(Box::new(PlanAnswerLogged::new(
                ev(),
                agg(),
                PlanApprovalOperationId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0999").unwrap(),
                PlanAnswerInput::new(
                    PlanApprovalOrigin::new(SpaceName::default(), agg()),
                    "code-generation".to_string(),
                    PlanDecisionEvidence::new(
                        evidence(),
                        PlanSession::new("session-1".to_string()).unwrap(),
                    ),
                    PlanChoice::RequestChanges,
                    None,
                ),
            ))),
        ),
        (
            "AnswerRecorded(summary)",
            IntentExecutionEvent::AnswerRecorded(AnswerRecorded::new(
                ev(),
                agg(),
                AnswerId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0aaa").unwrap(),
                "requirements-analysis",
                "A. Accept assumptions",
                AnswerDisposition::SummaryConfirmed(
                    SummaryEvidence::new("requirements-questions.md", sha('f')).unwrap(),
                ),
            )),
        ),
        (
            "AnswerRecorded(report-owned)",
            IntentExecutionEvent::AnswerRecorded(AnswerRecorded::new(
                ev(),
                agg(),
                AnswerId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0bbb").unwrap(),
                "intent-capture",
                "Approve",
                AnswerDisposition::ApprovalGateReportOwned,
            )),
        ),
        (
            "AnswerRecorded(recorded)",
            IntentExecutionEvent::AnswerRecorded(AnswerRecorded::new(
                ev(),
                agg(),
                AnswerId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0ccc").unwrap(),
                "requirements-analysis",
                "B",
                AnswerDisposition::Recorded,
            )),
        ),
        (
            "DirectiveIssued(error)",
            IntentExecutionEvent::DirectiveIssued(DirectiveIssued::new(
                ev(),
                agg(),
                directive(PublishedDirective::Error {
                    stage: slug("intent-capture"),
                }),
            )),
        ),
        (
            "DirectiveContextInvalidated",
            IntentExecutionEvent::DirectiveContextInvalidated(DirectiveContextInvalidated::new(
                ev(),
                agg(),
                directive(PublishedDirective::LoadSteering {
                    stage: slug("intent-capture"),
                    part: 1,
                    parts: 3,
                    token: "steer-01".to_string(),
                }),
            )),
        ),
        (
            "DirectiveIssued",
            IntentExecutionEvent::DirectiveIssued(DirectiveIssued::new(
                ev(),
                agg(),
                directive(PublishedDirective::RunStage {
                    stage: slug("code-generation"),
                    unit: Some("u2-workflow-authority".to_string()),
                }),
            )),
        ),
        (
            "Reported(GateOpened)",
            reported(
                committed(
                    ReportTransition::GateOpened {
                        artifacts: ArtifactPaths::new(vec!["intent.md".to_string()]),
                    },
                    vec![TransitionStep::GateStart],
                ),
                Some(StageValidation::Basis("{\"schema\":3}".to_string())),
                None,
            ),
        ),
        (
            "Reported(GateApproved recovered)",
            reported(
                committed(
                    ReportTransition::GateApproved {
                        user_input: Some("Approve".to_string()),
                    },
                    vec![TransitionStep::GateStartRecovered, TransitionStep::Approve],
                ),
                Some(StageValidation::Warning("receipt unavailable".to_string())),
                Some(SourceBaseline::new(None).unwrap()),
            ),
        ),
        (
            "Reported(GateRejected)",
            reported(
                committed(
                    ReportTransition::GateRejected {
                        feedback: Some("more detail".to_string()),
                    },
                    vec![TransitionStep::Reject],
                ),
                None,
                None,
            ),
        ),
        (
            "Reported(StageRevised)",
            reported(
                committed(ReportTransition::StageRevised, vec![TransitionStep::Revise]),
                None,
                None,
            ),
        ),
        (
            "Reported(StageSkipped)",
            reported(
                committed(
                    ReportTransition::StageSkipped {
                        reason: "out of scope".to_string(),
                    },
                    vec![TransitionStep::Skip],
                ),
                None,
                Some(SourceBaseline::new(Some(listing())).unwrap()),
            ),
        ),
        (
            "Reported(AlreadyCompletedMovedOn)",
            reported(
                ReportResult::NoOp {
                    scope: "classic".to_string(),
                    no_op: ReportNoOp::AlreadyCompletedMovedOn {
                        stage: slug("intent-capture"),
                        current: slug("scope-definition"),
                    },
                },
                None,
                None,
            ),
        ),
        (
            "Reported(WorkflowAlreadyCompleted)",
            reported(
                ReportResult::NoOp {
                    scope: "classic".to_string(),
                    no_op: ReportNoOp::WorkflowAlreadyCompleted {
                        stage: slug("scope-definition"),
                    },
                },
                None,
                None,
            ),
        ),
        (
            "Jumped(backward with scope)",
            IntentExecutionEvent::Jumped(
                Jumped::new(
                    ev(),
                    agg(),
                    slug("intent-capture"),
                    JumpDirection::Backward,
                    Some(observation()),
                )
                .with_scope(Some(JumpScope::new(
                    "express".to_string(),
                    StageSlugSet::new([slug("intent-capture"), slug("scope-definition")]),
                ))),
            ),
        ),
        (
            "Jumped(redo)",
            IntentExecutionEvent::Jumped(Jumped::new(
                ev(),
                agg(),
                slug("scope-definition"),
                JumpDirection::Redo,
                None,
            )),
        ),
        (
            "Jumped(forward)",
            IntentExecutionEvent::Jumped(Jumped::new(
                ev(),
                agg(),
                slug("scope-definition"),
                JumpDirection::Forward,
                None,
            )),
        ),
        (
            "Started",
            IntentExecutionEvent::Started(Started::new(
                ev(),
                agg(),
                support::intent_id(),
                support::stages(),
            )),
        ),
        (
            "GateOpened",
            IntentExecutionEvent::GateOpened(GateOpened::new(
                ev(),
                agg(),
                slug("intent-capture"),
                ArtifactPaths::new(vec!["intent.md".to_string()]),
            )),
        ),
        (
            "GateApproved",
            IntentExecutionEvent::GateApproved(GateApproved::new(
                ev(),
                agg(),
                slug("intent-capture"),
                Some("Approve".to_string()),
            )),
        ),
        (
            "GateRejected",
            IntentExecutionEvent::GateRejected(GateRejected::new(
                ev(),
                agg(),
                slug("intent-capture"),
                Some("more detail".to_string()),
            )),
        ),
        (
            "StageRevised",
            IntentExecutionEvent::StageRevised(StageRevised::new(
                ev(),
                agg(),
                slug("intent-capture"),
            )),
        ),
        (
            "StageSkipped",
            IntentExecutionEvent::StageSkipped(StageSkipped::new(
                ev(),
                agg(),
                slug("scope-definition"),
                "out of scope".to_string(),
            )),
        ),
        (
            "Parked",
            IntentExecutionEvent::Parked(Parked::new(ev(), agg(), slug("intent-capture"))),
        ),
        (
            "Unparked",
            IntentExecutionEvent::Unparked(Unparked::new(ev(), agg())),
        ),
        (
            "Recomposed",
            IntentExecutionEvent::Recomposed(Recomposed::new(
                ev(),
                agg(),
                StageSlugSet::new([slug("scope-definition")]),
                StageSlugSet::new([slug("intent-capture")]),
            )),
        ),
        (
            "SingleStageRunCommitted",
            IntentExecutionEvent::SingleStageRunCommitted(SingleStageRunCommitted::new(
                ev(),
                agg(),
                slug("intent-capture"),
            )),
        ),
        (
            "ReviewRequested",
            IntentExecutionEvent::ReviewRequested(ReviewRequested::new(
                ev(),
                agg(),
                slug("intent-capture"),
                "aidlc-product-lead-agent",
                2,
                true,
                review_fixture::binding(),
            )),
        ),
        (
            "ReviewCompleted",
            IntentExecutionEvent::ReviewCompleted(ReviewCompleted::new(
                ev(),
                agg(),
                slug("intent-capture"),
                "aidlc-product-lead-agent",
                2,
                ReviewVerdict::NotReady,
                review_fixture::completion(),
            )),
        ),
        (
            "PracticesAffirmed",
            IntentExecutionEvent::PracticesAffirmed(PracticesAffirmed::new(
                ev(),
                agg(),
                slug("intent-capture"),
                "owner",
                PromotedSections::new(vec![PromotedSection::new(
                    "Way of Working",
                    "trunk-based.\n",
                )])
                .unwrap(),
                RuleLines::new(vec!["ALWAYS review. (affirmed 2026-09-05)".to_string()]),
                RuleLines::new(vec!["NEVER force-push. (affirmed 2026-09-05)".to_string()]),
            )),
        ),
        (
            "SkeletonStanceRecorded",
            IntentExecutionEvent::SkeletonStanceRecorded(SkeletonStanceRecorded::new(
                ev(),
                agg(),
                SkeletonStance::On,
            )),
        ),
        (
            "AutonomyModeSet",
            IntentExecutionEvent::AutonomyModeSet(AutonomyModeSet::new(
                ev(),
                agg(),
                AutonomyMode::Autonomous,
            )),
        ),
    ]
}

/// 全変種で、行の識別子 2 欄（`id` / `aggregate_id`）の文法外は欄名付きで拒否される —
/// どの変種も自分の復号器で識別子を検査し、別の変種の検査に頼らない。
#[test]
fn every_variant_refuses_a_malformed_identifier_by_its_field_name() {
    for (label, event) in events() {
        let json = serde_json::to_value(IntentExecutionEventDto::of(&event)).unwrap();
        let variant = json
            .as_object()
            .and_then(|object| object.keys().next().cloned())
            .unwrap_or_else(|| panic!("{label}: 外側は変種名 1 つのオブジェクト"));
        // `PlanAnswerLogged` だけは欄名を `event_id` と綴る（書く側と同じ綴り）。
        let id_field = if variant == "PlanAnswerLogged" {
            "event_id"
        } else {
            "id"
        };
        for (field, expected) in [("id", id_field), ("aggregate_id", "aggregate_id")] {
            let actual = mutate(&event, |j| {
                j[&variant][field] = "not-a-uuid".into();
            });
            // `SingleStageRunStarted` だけは欄名を添えず不変条件違反として拒否する
            // （現行の綴り。欄名付きへ揃えるかは人間の裁定事項であり、ここでは固定するだけ）。
            let expected = if variant == "SingleStageRunStarted" {
                DtoDecodeError::InvariantViolation
            } else {
                DtoDecodeError::malformed(expected, "not-a-uuid")
            };
            assert_eq!(actual.err(), Some(expected), "{label}: {field} の文法外");
        }
    }
}

/// ステージ参照を運ぶ変種は、文法外の綴りを `stage` の欄名で拒否する。
#[test]
fn stage_bearing_variants_refuse_a_malformed_stage() {
    let all = events();
    let by = |label: &str| all.iter().find(|(l, _)| *l == label).unwrap().1.clone();
    for (label, variant) in [
        ("GateOpened", "GateOpened"),
        ("GateApproved", "GateApproved"),
        ("GateRejected", "GateRejected"),
        ("StageRevised", "StageRevised"),
        ("StageSkipped", "StageSkipped"),
        ("Parked", "Parked"),
        ("SingleStageRunCommitted", "SingleStageRunCommitted"),
        ("TaskSynchronized", "TaskSynchronized"),
        ("ReviewRequested", "ReviewRequested"),
        ("ReviewCompleted", "ReviewCompleted"),
        ("PracticesAffirmed", "PracticesAffirmed"),
    ] {
        let actual = mutate(&by(label), |j| {
            j[variant]["stage"] = "Bad Slug".into();
        });
        assert_eq!(
            actual.err(),
            Some(DtoDecodeError::malformed("stage", "Bad Slug")),
            "{label}"
        );
    }
    assert_eq!(
        mutate(&by("SingleStageRunStarted"), |j| {
            j["SingleStageRunStarted"]["stage"] = "Bad Slug".into();
        })
        .err(),
        Some(DtoDecodeError::InvariantViolation),
        "隔離実行の開始だけは欄名を添えない現行の綴り"
    );
    assert_eq!(
        mutate(&by("Jumped(redo)"), |j| {
            j["Jumped"]["target"] = "Bad Slug".into();
        })
        .err(),
        Some(DtoDecodeError::malformed("target", "Bad Slug")),
        "跳躍先も文法検査を通す"
    );
    assert_eq!(
        mutate(&by("Reported(StageRevised)"), |j| {
            j["Reported"]["result"]["Committed"]["stage"] = "Bad Slug".into();
        })
        .err(),
        Some(DtoDecodeError::malformed("stage", "Bad Slug")),
        "報告の確定位置も文法検査を通す"
    );
}

/// 証跡・基準・任意の識別子欄が壊れた行は、その欄名で拒否される。
#[test]
fn corrupt_evidence_and_optional_identifiers_are_refused_by_name() {
    let all = events();
    let by = |label: &str| all.iter().find(|(l, _)| *l == label).unwrap().1.clone();
    let cases: Vec<(
        &str,
        Result<IntentExecutionEvent, DtoDecodeError>,
        DtoDecodeError,
    )> = vec![
        (
            "計画提示の直前応答 id が文法外",
            mutate(&by("DecisionRecorded"), |j| {
                j["DecisionRecorded"]["human_before"] = "x".into();
            }),
            DtoDecodeError::malformed("human_before", "x"),
        ),
        (
            "在席応答の承認観測 id が文法外",
            mutate(&by("PromptObserved"), |j| {
                j["PromptObserved"]["approval_observation_id"] = "x".into();
            }),
            DtoDecodeError::malformed("approval_observation_id", "x"),
        ),
        (
            "要約確認の SHA-256 が文法外",
            mutate(&by("AnswerRecorded(summary)"), |j| {
                j["AnswerRecorded"]["summary"]["questions_sha256"] = "nope".into();
            }),
            DtoDecodeError::malformed("summary", "invalid summary evidence"),
        ),
        (
            "レビュー判定の指紋が文法外",
            mutate(&by("ReviewCompleted"), |j| {
                j["ReviewCompleted"]["evidence"]["fingerprint"] = "nope".into();
            }),
            DtoDecodeError::malformed("review_completion", "invalid review binding"),
        ),
        (
            "レビュー要求の指紋が文法外",
            mutate(&by("ReviewRequested"), |j| {
                j["ReviewRequested"]["evidence"]["fingerprint"] = "nope".into();
            }),
            DtoDecodeError::malformed("review_binding", "invalid review binding"),
        ),
        (
            "報告のソース基準が一覧の文法外",
            mutate(&by("Reported(StageSkipped)"), |j| {
                j["Reported"]["source_baseline"]["listing"] = "not a listing".into();
            }),
            DtoDecodeError::malformed("baseline", "malformed listing line"),
        ),
        (
            "誕生記録の intent id が文法外",
            mutate(&by("Started"), |j| {
                j["Started"]["intent_id"] = "x".into();
            }),
            DtoDecodeError::malformed("intent_id", "x"),
        ),
    ];
    for (label, actual, expected) in cases {
        let actual = actual
            .err()
            .unwrap_or_else(|| panic!("{label}: 拒否される"));
        match (&actual, &expected) {
            (
                DtoDecodeError::Malformed {
                    field: actual_field,
                    ..
                },
                DtoDecodeError::Malformed {
                    field: expected_field,
                    ..
                },
            ) => assert_eq!(
                actual_field, expected_field,
                "{label}: 欄名 (実際: {actual:?})"
            ),
            _ => panic!("{label}: Malformed を期待した (実際: {actual:?})"),
        }
    }
}

#[test]
fn every_variant_the_read_side_spells_is_read_back_unchanged() {
    for (label, event) in events() {
        let dto = IntentExecutionEventDto::of(&event);
        let bytes = serde_json::to_string(&dto).unwrap();
        let read: IntentExecutionEventDto = serde_json::from_str(&bytes).unwrap();
        assert_eq!(read, dto, "{label}: 直列化の往復で DTO が変わらない");
        let decoded = read
            .to_domain()
            .unwrap_or_else(|error| panic!("{label}: {error:?}"));
        assert_eq!(decoded, event, "{label}: ドメインへ戻して等しい: {bytes}");
        assert_eq!(
            IntentExecutionEventDto::of(&decoded),
            dto,
            "{label}: 戻したものを再び綴っても同じ行"
        );
        assert!(
            bytes.contains(event.id().as_str()) && bytes.contains(event.aggregate_id().as_str()),
            "{label}: id / aggregate_id が行に載る: {bytes}"
        );
    }
}

#[test]
fn the_wire_spelling_of_closed_vocabularies_is_fixed() {
    let all = events();
    let json = |label: &str| -> Value {
        let event = &all.iter().find(|(l, _)| *l == label).unwrap().1;
        serde_json::to_value(IntentExecutionEventDto::of(event)).unwrap()
    };
    let rows = json("LearningsCaptured");
    let rows = &rows["LearningsCaptured"]["learnings"];
    assert_eq!(rows[0]["disposition"], "fresh");
    assert_eq!(rows[1]["disposition"], "practice_line_only");
    assert_eq!(rows[2]["disposition"], "audit_row_only");
    let jumped = json("Jumped(backward with scope)");
    assert_eq!(jumped["Jumped"]["direction"], "backward");
    assert_eq!(jumped["Jumped"]["scope"]["name"], "express");
    assert_eq!(json("Jumped(redo)")["Jumped"]["direction"], "redo");
    assert_eq!(
        json("AnswerRecorded(report-owned)")["AnswerRecorded"]["disposition"],
        "approval-gate-report-owned"
    );
    assert_eq!(
        json("PlanAnswerLogged")["PlanAnswerLogged"]["input"]["choice"],
        "Request Changes"
    );
    assert_eq!(
        json("Reported(GateApproved recovered)")["Reported"]["result"]["Committed"]["steps"],
        serde_json::json!(["GateStartRecovered", "Approve"])
    );
}

/// 保存済みの行の 1 箇所を書き換えて、読む側が成功に丸めないことを見る。
fn mutate(
    event: &IntentExecutionEvent,
    edit: impl FnOnce(&mut Value),
) -> Result<IntentExecutionEvent, DtoDecodeError> {
    let mut json = serde_json::to_value(IntentExecutionEventDto::of(event)).unwrap();
    edit(&mut json);
    let dto: IntentExecutionEventDto = serde_json::from_value(json).unwrap();
    dto.to_domain()
}

#[test]
fn a_corrupt_row_is_refused_with_the_field_that_broke() {
    let all = events();
    let by = |label: &str| all.iter().find(|(l, _)| *l == label).unwrap().1.clone();
    let malformed = |field: &'static str, found: &str| DtoDecodeError::malformed(field, found);
    let cases: Vec<(
        &str,
        Result<IntentExecutionEvent, DtoDecodeError>,
        DtoDecodeError,
    )> = vec![
        (
            "学びの内訳が未知の綴り",
            mutate(&by("LearningsCaptured"), |j| {
                j["LearningsCaptured"]["learnings"][0]["disposition"] = "kept".into();
            }),
            malformed("learnings.disposition", "kept"),
        ),
        (
            "学びの候補番号が文法外",
            mutate(&by("LearningsCaptured"), |j| {
                j["LearningsCaptured"]["learnings"][1]["candidate_id"] = "".into();
            }),
            malformed("learnings.candidate_id", ""),
        ),
        (
            "学びの stage が文法外",
            mutate(&by("LearningsCaptured"), |j| {
                j["LearningsCaptured"]["stage"] = "Not Slug".into();
            }),
            malformed("stage", "Not Slug"),
        ),
        (
            "学びの space が文法外",
            mutate(&by("LearningsCaptured"), |j| {
                j["LearningsCaptured"]["space"] = "../x".into();
            }),
            malformed("space", "../x"),
        ),
        (
            "学びの intent 記録名が文法外",
            mutate(&by("LearningsCaptured"), |j| {
                j["LearningsCaptured"]["intent"] = "".into();
            }),
            malformed("intent", ""),
        ),
        (
            "学びの id が UUIDv7 でない",
            mutate(&by("LearningsCaptured"), |j| {
                j["LearningsCaptured"]["id"] = "x".into();
            }),
            malformed("id", "x"),
        ),
        (
            "学びの aggregate_id が UUIDv7 でない",
            mutate(&by("LearningsCaptured"), |j| {
                j["LearningsCaptured"]["aggregate_id"] = "x".into();
            }),
            malformed("aggregate_id", "x"),
        ),
        (
            "日誌観測の stage が文法外",
            mutate(&by("MemoryJournalsObserved"), |j| {
                j["MemoryJournalsObserved"]["survey"][0]["stage"] = "1bad".into();
            }),
            malformed("survey.stage", "1bad"),
        ),
        (
            "日誌観測の空位置が文法外",
            mutate(&by("MemoryJournalsObserved"), |j| {
                j["MemoryJournalsObserved"]["empty_stages"][0] = "Bad Slug".into();
            }),
            malformed("empty_stages", "Bad Slug"),
        ),
        (
            "日誌観測の id が文法外",
            mutate(&by("MemoryJournalsObserved"), |j| {
                j["MemoryJournalsObserved"]["id"] = "x".into();
            }),
            malformed("id", "x"),
        ),
        (
            "日誌観測の aggregate_id が文法外",
            mutate(&by("MemoryJournalsObserved"), |j| {
                j["MemoryJournalsObserved"]["aggregate_id"] = "x".into();
            }),
            malformed("aggregate_id", "x"),
        ),
        (
            "指示の stage が文法外",
            mutate(&by("DirectiveIssued"), |j| {
                j["DirectiveIssued"]["directive"]["directive"]["RunStage"]["stage"] = "Bad".into();
            }),
            malformed("stage", "Bad"),
        ),
        (
            "steering 指示の stage が文法外",
            mutate(&by("DirectiveContextInvalidated"), |j| {
                j["DirectiveContextInvalidated"]["directive"]["directive"]["LoadSteering"]["stage"] =
                    "Bad".into();
            }),
            malformed("stage", "Bad"),
        ),
        (
            "指示の intent_id が文法外",
            mutate(&by("DirectiveIssued"), |j| {
                j["DirectiveIssued"]["directive"]["intent_id"] = "x".into();
            }),
            malformed("intent_id", "x"),
        ),
        (
            "指示の承認操作 id が文法外",
            mutate(&by("DirectiveIssued"), |j| {
                j["DirectiveIssued"]["directive"]["approval_operation_id"] = "x".into();
            }),
            malformed("approval_operation_id", "x"),
        ),
        (
            "回答の内訳が未知の綴り",
            mutate(&by("AnswerRecorded(report-owned)"), |j| {
                j["AnswerRecorded"]["disposition"] = "kept".into();
            }),
            malformed("disposition", "kept"),
        ),
        (
            "要約確認の証跡を欠く",
            mutate(&by("AnswerRecorded(summary)"), |j| {
                j["AnswerRecorded"]["summary"] = Value::Null;
            }),
            malformed("summary", "missing"),
        ),
        (
            "回答の answer_id が文法外",
            mutate(&by("AnswerRecorded(summary)"), |j| {
                j["AnswerRecorded"]["answer_id"] = "x".into();
            }),
            malformed("answer_id", "x"),
        ),
        (
            "跳躍の向きが未知の綴り",
            mutate(&by("Jumped(redo)"), |j| {
                j["Jumped"]["direction"] = "sideways".into();
            }),
            malformed("direction", "sideways"),
        ),
        (
            "跳躍 scope の位置が文法外",
            mutate(&by("Jumped(backward with scope)"), |j| {
                j["Jumped"]["scope"]["executes"][0] = "Bad".into();
            }),
            malformed("scope.executes", "Bad"),
        ),
        (
            "跳躍観測の位置が文法外",
            mutate(&by("Jumped(backward with scope)"), |j| {
                j["Jumped"]["observation"]["artifacts"][0]["stage"] = "Bad".into();
            }),
            malformed("stage", "Bad"),
        ),
        (
            "報告の操作列が未知の綴り",
            mutate(&by("Reported(GateOpened)"), |j| {
                j["Reported"]["result"]["Committed"]["steps"][0] = "Explode".into();
            }),
            malformed("steps", "Explode"),
        ),
        (
            "報告の根拠と警告が両方ある",
            mutate(&by("Reported(GateOpened)"), |j| {
                j["Reported"]["validation_warning"] = "also".into();
            }),
            DtoDecodeError::InvariantViolation,
        ),
        (
            "報告の操作列が遷移と合わない",
            mutate(&by("Reported(GateOpened)"), |j| {
                j["Reported"]["result"]["Committed"]["steps"][0] = "Skip".into();
            }),
            DtoDecodeError::InvariantViolation,
        ),
        (
            "報告の report_id が文法外",
            mutate(&by("Reported(StageRevised)"), |j| {
                j["Reported"]["report_id"] = "x".into();
            }),
            malformed("report_id", "x"),
        ),
        (
            "計画回答の選択が未知の綴り",
            mutate(&by("PlanAnswerLogged"), |j| {
                j["PlanAnswerLogged"]["input"]["choice"] = "Maybe".into();
            }),
            malformed("choice", "Maybe"),
        ),
        (
            "計画回答の space が文法外",
            mutate(&by("PlanAnswerLogged"), |j| {
                j["PlanAnswerLogged"]["input"]["space"] = "../x".into();
            }),
            malformed("space", "../x"),
        ),
        (
            "計画提示に証跡と要約ファイルが両方ある",
            mutate(&by("DecisionRecorded"), |j| {
                j["DecisionRecorded"]["summary_file"] = "q.md".into();
            }),
            DtoDecodeError::InvariantViolation,
        ),
        (
            "計画提示のセッションが空",
            mutate(&by("DecisionRecorded"), |j| {
                j["DecisionRecorded"]["plan_approval"]["session"] = "".into();
            }),
            malformed("plan_session", ""),
        ),
        (
            "計画証跡の unit が文法外",
            mutate(&by("PlanAnswerLogged"), |j| {
                j["PlanAnswerLogged"]["input"]["evidence"]["authority"]["unit"] = "Bad Unit".into();
            }),
            DtoDecodeError::InvariantViolation,
        ),
        (
            "計画証跡の intent_id が文法外",
            mutate(&by("PlanAnswerLogged"), |j| {
                j["PlanAnswerLogged"]["input"]["evidence"]["authority"]["intent_id"] = "x".into();
            }),
            malformed("intent_id", "x"),
        ),
        (
            "計画証跡の指紋が文法外",
            mutate(&by("PlanAnswerLogged"), |j| {
                j["PlanAnswerLogged"]["input"]["evidence"]["fingerprint"] = "nope".into();
            }),
            DtoDecodeError::InvariantViolation,
        ),
    ];
    for (label, actual, expected) in cases {
        assert_eq!(actual.err(), Some(expected), "{label}");
    }
}

/// intent の誕生記録は、記録名とソース基準を持つときもそのまま読み戻せる。
#[test]
fn an_intent_birth_with_a_record_name_and_a_source_baseline_is_read_back_unchanged() {
    use core_command_domain::orchestration::{
        Created, IntentEvent, IntentEventId, IntentRecordName, StartRequest,
    };
    use core_command_domain::workflow_definition::{DefinitionRevision, WorkflowDefinitionId};
    use core_read_model_updater::orchestration::IntentEventDto;
    let created = Created::new(
        IntentEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0001").unwrap(),
        support::intent_id(),
        WorkflowDefinitionId::parse("claude").unwrap(),
        DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
        StartRequest::new("classic", "contract")
            .with_record_name(IntentRecordName::new(
                IntentDirName::parse("260907-selfhost-stage1").unwrap(),
                "selfhost-stage1",
            ))
            .with_source_baseline(SourceBaseline::new(Some(listing())).unwrap()),
        support::stages(),
        support::scan(),
    );
    let event = IntentEvent::Created(created);
    let dto = IntentEventDto::of(&event, support::at());
    let bytes = serde_json::to_string(&dto).unwrap();
    let read: IntentEventDto = serde_json::from_str(&bytes).unwrap();
    let intent = read.to_domain().unwrap();
    assert_eq!(
        intent
            .record_name()
            .map(|name| name.directory().as_str().to_string()),
        Some("260907-selfhost-stage1".to_string())
    );
    assert_eq!(
        intent
            .source_baseline()
            .and_then(|baseline| baseline.listing().map(str::to_string)),
        Some(listing())
    );
    let round_trip: IntentEventDto =
        serde_json::from_str(&serde_json::to_string(&read).unwrap()).unwrap();
    assert_eq!(round_trip, dto, "読み戻した DTO を再び綴っても同じ行");
    // EXECUTE のまま greenfield 調整済みと名乗る行は組み上げない。
    let mut json: Value = serde_json::from_str(&bytes).unwrap();
    json["Created"]["stages"][1]["greenfield_adjusted"] = true.into();
    let read: IntentEventDto = serde_json::from_value(json).unwrap();
    assert_eq!(
        read.to_domain().err(),
        Some(DtoDecodeError::InvariantViolation)
    );
}

/// intent の誕生記録の壊れた行は、壊れた欄の名前で拒否される（読む側 DTO の復号器）。
#[test]
fn a_corrupt_intent_birth_is_refused_with_the_field_that_broke() {
    use core_command_domain::orchestration::IntentEvent;
    use core_read_model_updater::orchestration::IntentEventDto;
    let event = IntentEvent::Created(support::intent_created());
    let json = serde_json::to_value(IntentEventDto::of(&event, support::at())).unwrap();
    let mutate = |edit: &dyn Fn(&mut Value)| -> DtoDecodeError {
        let mut json = json.clone();
        edit(&mut json);
        let dto: IntentEventDto = serde_json::from_value(json).unwrap();
        dto.to_domain().expect_err("壊れた行は拒否される")
    };
    let field_of = |error: &DtoDecodeError| match error {
        DtoDecodeError::Malformed { field, .. } => (*field).to_string(),
        DtoDecodeError::InvariantViolation => "<invariant>".to_string(),
    };
    let cases: Vec<(&str, Box<dyn Fn(&mut Value)>, &str)> = vec![
        ("id", Box::new(|j| j["Created"]["id"] = "x".into()), "id"),
        (
            "aggregate_id",
            Box::new(|j| j["Created"]["aggregate_id"] = "x".into()),
            "aggregate_id",
        ),
        (
            "definition_id",
            Box::new(|j| j["Created"]["definition_id"] = "".into()),
            "definition_id",
        ),
        (
            "definition_revision",
            Box::new(|j| j["Created"]["definition_revision"] = "nope".into()),
            "definition_revision",
        ),
        (
            "record_name.directory",
            Box::new(|j| {
                j["Created"]["start_request"]["record_name"] =
                    serde_json::json!({"directory": "", "slug": "x"});
            }),
            "record_name.directory",
        ),
        (
            // intent 面のソース基準は欄名を添えず不変条件違反として拒む（現行の綴り）。
            "baseline",
            Box::new(|j| {
                j["Created"]["start_request"]["source_baseline"] =
                    serde_json::json!({"listing": "not a listing"});
            }),
            "<invariant>",
        ),
        (
            "stage number",
            Box::new(|j| j["Created"]["stages"][0]["display"]["number"] = "x".into()),
            "number",
        ),
        (
            "stage display",
            Box::new(|j| j["Created"]["stages"][0]["display"]["name"] = "two\nlines".into()),
            "display",
        ),
        (
            "stage slug",
            Box::new(|j| j["Created"]["stages"][0]["slug"] = "Bad Slug".into()),
            "slug",
        ),
        (
            "scan",
            Box::new(|j| j["Created"]["scan"]["languages"] = "two\nlines".into()),
            "scan",
        ),
    ];
    for (label, edit, expected) in cases {
        let error = mutate(&*edit);
        assert_eq!(field_of(&error), expected, "{label} (実際: {error:?})");
    }
}
