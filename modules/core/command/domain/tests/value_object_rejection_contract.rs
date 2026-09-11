//! 値オブジェクト・ローカルエンティティの parse 拒否と退化 — 不正入力を成功に丸めない。
#![allow(clippy::unwrap_used)]
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    AutonomyMode, CodeGenerationRunFloor, ContinuationAttemptId, ContinuationError,
    ContinuationRequest, ContinuationSignature, DecisionPrompt, InspectedTool, MemoryEntry,
    MemoryEntryHeading, ReviewedStage, ReviewedUnit, ReviewerScopeBlock, ReviewerScopeVerdict,
    RunBoundaryKind, RunFloorError, ScopeToken, SummaryQuestions, SummaryQuestionsError,
};
use core_command_domain::workspace::{
    AuditFieldKey, AuditFields, EventType, HookDropSummary, HookHealth, HookHealthError,
    HookHealthId, HookHealthTarget, HookName, SessionAudit, SessionAuditError, SessionAuditId,
    SessionAuditObservationId, SessionAuditRecord, SpaceName,
};

fn at() -> DateTime<Utc> {
    "2026-09-11T00:00:00Z".parse().unwrap()
}

// --- MemoryEntry ---------------------------------------------------------------

#[test]
fn a_journal_line_without_a_bullet_or_dash_degrades_instead_of_failing() {
    // 箇条書き記号が無い / 記号の後に空白が無い行は、行全体を要約として退化させる。
    for raw in ["解釈だけ", "-記号直後に文字", "*x"] {
        let entry = MemoryEntry::parse(MemoryEntryHeading::Interpretations, raw);
        assert_eq!(entry.timestamp(), "");
        assert_eq!(entry.summary(), raw);
        assert_eq!(entry.context(), "");
        assert_eq!(entry.line(), raw);
    }
}

#[test]
fn a_timestamp_needs_whitespace_on_both_sides_of_the_dash() {
    // `^(\S+)\s+—\s+(.*)$` — 空白が欠けるとダッシュ区切りとして読まない。
    for raw in [
        "- 2026-09-11T00:00:00Z—要約",
        "- 2026-09-11T00:00:00Z —要約",
        "- 2026-09-11T00:00:00Z 要約",
    ] {
        let entry = MemoryEntry::parse(MemoryEntryHeading::Deviations, raw);
        assert_eq!(entry.timestamp(), "", "{raw}");
        assert_eq!(entry.summary(), raw.trim_start_matches("- "), "{raw}");
    }
    let entry = MemoryEntry::parse(
        MemoryEntryHeading::Tradeoffs,
        "* 2026-09-11T00:00:00Z \u{2014} 要約; 文脈",
    );
    assert_eq!(entry.timestamp(), "2026-09-11T00:00:00Z");
    assert_eq!(entry.summary(), "要約");
    assert_eq!(entry.context(), "文脈");
}

// --- ContinuationRequest ---------------------------------------------------------

#[test]
fn a_continuation_request_rejects_a_zero_limit_and_an_impossible_reset_history() {
    let id = ContinuationAttemptId::generate();
    assert_eq!(
        ContinuationRequest::new(id.clone(), None, false, 0).unwrap_err(),
        ContinuationError::InvalidLimit
    );
    // リセット (署名なし) は再入でも上限 1 以外でもない。
    assert_eq!(
        ContinuationRequest::new(id.clone(), None, true, 1).unwrap_err(),
        ContinuationError::InvalidHistory
    );
    assert_eq!(
        ContinuationRequest::new(id.clone(), None, false, 2).unwrap_err(),
        ContinuationError::InvalidHistory
    );
    let reset = ContinuationRequest::new(id, None, false, 1).unwrap();
    assert!(reset.is_reset());
    // リセットは環境指定でも上限を動かさない。
    assert_eq!(
        reset.for_mode(AutonomyMode::Autonomous, Some("5")).limit(),
        1
    );
}

#[test]
fn an_environment_limit_override_accepts_only_positive_finite_numbers() {
    let request = || {
        ContinuationRequest::new(
            ContinuationAttemptId::generate(),
            Some(
                ContinuationSignature::parse(&format!(
                    "code-generation::{}::{}",
                    "a".repeat(64),
                    "b".repeat(64)
                ))
                .unwrap(),
            ),
            false,
            4,
        )
        .unwrap()
    };
    assert_eq!(request().for_mode(AutonomyMode::Gated, None).limit(), 2);
    assert_eq!(
        request().for_mode(AutonomyMode::Autonomous, None).limit(),
        8
    );
    assert_eq!(
        request()
            .for_mode(AutonomyMode::Gated, Some(" +12abc "))
            .limit(),
        12
    );
    for raw in ["-3", "0", "abc", ""] {
        assert_eq!(
            request().for_mode(AutonomyMode::Gated, Some(raw)).limit(),
            2,
            "{raw:?}"
        );
    }
    assert_eq!(
        request()
            .for_mode(AutonomyMode::Gated, Some(&"9".repeat(40)))
            .limit(),
        u64::MAX
    );
}

// --- DecisionPrompt ---------------------------------------------------------------

#[test]
fn a_decision_prompt_carries_its_rationale_and_options() {
    let prompt = DecisionPrompt::new("code-generation", "Plan Approval")
        .with_options("A, B")
        .with_rationale("because");
    assert_eq!(prompt.stage(), "code-generation");
    assert_eq!(prompt.decision(), "Plan Approval");
    assert_eq!(prompt.options(), Some("A, B"));
    assert_eq!(prompt.rationale(), Some("because"));
    assert_eq!(prompt.summary_file(), None);
}

// --- CodeGenerationRunFloor ---------------------------------------------------------

#[test]
fn a_run_floor_whose_latest_boundary_was_never_counted_is_rejected() {
    assert_eq!(
        CodeGenerationRunFloor::new(0, 0, 0, 0, Some((RunBoundaryKind::StageStarted, at()))),
        Err(RunFloorError)
    );
    assert_eq!(
        CodeGenerationRunFloor::new(1, 0, 0, 0, None),
        Err(RunFloorError)
    );
    assert_eq!(
        CodeGenerationRunFloor::new(0, 0, 0, 0, None).unwrap(),
        CodeGenerationRunFloor::default()
    );
}

// --- ReviewerScopeBlock -------------------------------------------------------------

#[test]
fn a_reviewer_scope_block_exposes_the_material_the_refusal_names() {
    let block = ReviewerScopeBlock::new(
        InspectedTool::Read,
        ScopeToken::parse("construction/u1/plan.md").unwrap(),
        ReviewedStage::new("code-generation".to_string()),
        ReviewedUnit::parse("u2").unwrap(),
    );
    assert_eq!(block.tool(), InspectedTool::Read);
    assert_eq!(block.target().as_str(), "construction/u1/plan.md");
    assert_eq!(block.stage().as_str(), "code-generation");
    assert_eq!(block.unit().as_str(), "u2");
    assert!(ReviewerScopeVerdict::Blocked(block).is_blocked());
}

// --- SessionAudit / HookHealth の再構成 ----------------------------------------------

#[test]
fn a_session_audit_snapshot_must_match_its_target_and_carry_a_sequence() {
    let target = HookHealthTarget::new(SpaceName::parse("default").unwrap(), None);
    let other = HookHealthTarget::new(SpaceName::parse("other").unwrap(), None);
    let record = SessionAuditRecord::new(
        EventType::SubagentCompleted,
        AuditFields::new().with(
            AuditFieldKey::parse("Agent Type").unwrap(),
            "aidlc-developer-agent",
        ),
    )
    .unwrap();
    assert_eq!(
        SessionAudit::new(
            SessionAuditId::for_target(&other),
            target.clone(),
            record.clone(),
            SessionAuditObservationId::generate(),
            1,
            1,
            at(),
        )
        .unwrap_err(),
        SessionAuditError::InvalidHistory
    );
    assert_eq!(
        SessionAudit::new(
            SessionAuditId::for_target(&target),
            target.clone(),
            record.clone(),
            SessionAuditObservationId::generate(),
            0,
            0,
            at(),
        )
        .unwrap_err(),
        SessionAuditError::InvalidHistory
    );
    assert!(
        SessionAudit::new(
            SessionAuditId::for_target(&target),
            target,
            record,
            SessionAuditObservationId::generate(),
            1,
            1,
            at(),
        )
        .is_ok()
    );
}

#[test]
fn a_hook_health_snapshot_must_match_its_hook_and_hold_some_observation() {
    let target = HookHealthTarget::new(SpaceName::parse("default").unwrap(), None);
    let hook = HookName::parse("aidlc-session-start").unwrap();
    let other = HookName::parse("aidlc-session-end").unwrap();
    assert_eq!(
        HookHealth::new(
            HookHealthId::for_hook(&target, &other),
            target.clone(),
            hook.clone(),
            Some(at()),
            at(),
            1,
            1,
            HookDropSummary::new(0, None).unwrap(),
        )
        .unwrap_err(),
        HookHealthError::TargetMismatch
    );
    // 通番 0、または心拍も欠落も無い観測は履歴として成立しない。
    assert_eq!(
        HookHealth::new(
            HookHealthId::for_hook(&target, &hook),
            target.clone(),
            hook.clone(),
            Some(at()),
            at(),
            0,
            0,
            HookDropSummary::new(0, None).unwrap(),
        )
        .unwrap_err(),
        HookHealthError::InvalidHistory
    );
    assert_eq!(
        HookHealth::new(
            HookHealthId::for_hook(&target, &hook),
            target,
            hook,
            None,
            at(),
            1,
            1,
            HookDropSummary::new(0, None).unwrap(),
        )
        .unwrap_err(),
        HookHealthError::InvalidHistory
    );
}

// --- SummaryQuestions の構造拒否 ------------------------------------------------------

const SUMMARY: &str = "## Consolidated Summary Confirmation\n\n[Answer]: A\n";

#[test]
fn a_summary_heading_indented_as_code_is_not_a_section() {
    // 4 空白のインデントはコードブロックであり、確認節にならない。
    let content = "    ## Consolidated Summary Confirmation\n\n[Answer]: A\n";
    assert_eq!(
        SummaryQuestions::parse(content, "A"),
        Err(SummaryQuestionsError::InvalidAnswer)
    );
}

#[test]
fn duplicate_question_sections_are_rejected() {
    let content = format!("## Q1. one\n\n## Q1. again\n\n{SUMMARY}");
    assert_eq!(
        SummaryQuestions::parse(&content, "A"),
        Err(SummaryQuestionsError::InvalidStructure(
            "duplicate H2 section \"Q1\"".to_string()
        ))
    );
}

#[test]
fn a_duplicate_assumption_confirmation_after_the_summary_is_rejected() {
    let content = format!(
        "{SUMMARY}\n## Assumption Confirmation\n\n[Answer]: A. Accept assumptions\n\n## Assumption Confirmation\n"
    );
    assert_eq!(
        SummaryQuestions::parse(&content, "A"),
        Err(SummaryQuestionsError::InvalidStructure(
            "duplicate H2 section \"Assumption Confirmation\"".to_string()
        ))
    );
}

#[test]
fn a_single_assumption_confirmation_after_the_summary_is_excluded_from_the_digest() {
    let with =
        format!("{SUMMARY}\n## Assumption Confirmation\n\n[Answer]: A. Accept assumptions\n");
    let without = SUMMARY.to_string();
    assert_eq!(
        SummaryQuestions::parse(&with, "A").unwrap().sha256(),
        SummaryQuestions::parse(&without, "A").unwrap().sha256()
    );
}

#[test]
fn question_ids_need_a_digit_run_without_a_leading_zero_and_a_separator() {
    // `Q01` と `Q1x` は質問 id ではないので、重複ではなく普通の節として通る。
    let content = format!("## Q01. one\n\n## Q01. two\n\n## Q1x\n\n## Q1x\n\n{SUMMARY}");
    assert!(SummaryQuestions::parse(&content, "A").is_ok());
}

#[test]
fn an_answer_tag_nested_in_a_list_or_quote_container_does_not_count() {
    // 回答行は節の直下に置く。リスト項目・引用の中に畳まれた `[Answer]:` は回答ではない。
    for nested in [
        "## Consolidated Summary Confirmation\n\n1) item\n\n   [Answer]: A\n",
        "## Consolidated Summary Confirmation\n\n- item\n  [Answer]: A\n",
        "## Consolidated Summary Confirmation\n\n> quote\n> [Answer]: A\n",
    ] {
        assert_eq!(
            SummaryQuestions::parse(nested, "A"),
            Err(SummaryQuestionsError::InvalidAnswer),
            "{nested:?}"
        );
    }
    // 同じ節でリストとタブ字下げのコード行が並んでも、直下の回答行は読める。
    let content = "## Consolidated Summary Confirmation\n\n1) item\n\n[Answer]: A\n\n\tcode\n";
    assert!(SummaryQuestions::parse(content, "A").is_ok());
}

#[test]
fn raw_html_blocks_hide_their_body_from_the_visible_lines() {
    // `<pre>` の中身は描画されないので、その中の見出しや回答行は数えない。
    let content =
        format!("{SUMMARY}\n<pre>\n## Later\n[Answer]: B\n</pre>\n<script>\n\n</script>\n");
    assert!(SummaryQuestions::parse(&content, "A").is_ok());
}

// --- 変換口・識別子・表示 ---------------------------------------------------------------

#[test]
fn owned_string_conversions_route_through_the_same_parse_as_borrowed_input() {
    use core_command_domain::workflow_definition::{
        DefinitionRevision, StageSlug, WorkflowDefinitionId,
    };
    assert_eq!(
        StageSlug::try_from("code-generation".to_string()).unwrap(),
        StageSlug::parse("code-generation").unwrap()
    );
    assert!(StageSlug::try_from("Not A Slug".to_string()).is_err());
    assert_eq!(
        WorkflowDefinitionId::try_from("claude".to_string()).unwrap(),
        WorkflowDefinitionId::parse("claude").unwrap()
    );
    assert!(WorkflowDefinitionId::try_from(String::new()).is_err());
    let revision = format!("sha256:{}", "0".repeat(64));
    assert_eq!(
        DefinitionRevision::try_from(revision.clone()).unwrap(),
        DefinitionRevision::parse(&revision).unwrap()
    );
    assert!(DefinitionRevision::try_from("sha256:short".to_string()).is_err());
}

#[test]
fn identifiers_render_their_canonical_spelling_and_refuse_the_rest() {
    use core_command_domain::orchestration::{AnswerId, AnswerIdError};
    use core_command_domain::workspace::{
        ArtifactAuditEventId, SessionAuditEventId, SessionAuditId,
    };
    let canonical = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";
    let answer = AnswerId::parse(canonical).unwrap();
    assert_eq!(answer.to_string(), canonical);
    assert_eq!(
        AnswerId::parse("0190AAAA-BBBB-7CCC-9DDD-EEEEFFFF0000"),
        Err(AnswerIdError::NotCanonicalUuidV7)
    );
    assert_eq!(
        AnswerId::parse("0190aaaa-bbbb-4ccc-9ddd-eeeeffff0000"),
        Err(AnswerIdError::NotCanonicalUuidV7)
    );

    let event = SessionAuditEventId::parse(canonical).unwrap();
    assert_eq!(event.as_str(), canonical);
    assert_eq!(
        SessionAuditEventId::parse("0190aaaa-bbbb-4ccc-9ddd-eeeeffff0000"),
        Err(SessionAuditError::InvalidEventIdentity)
    );
    let artifact = ArtifactAuditEventId::parse(canonical).unwrap();
    assert_eq!(artifact.as_str(), canonical);
    assert_eq!(
        ArtifactAuditEventId::parse("0190aaaa-bbbb-4ccc-9ddd-eeeeffff0000"),
        Err(HookHealthError::InvalidEventIdentity)
    );

    let target = HookHealthTarget::new(SpaceName::parse("default").unwrap(), None);
    let derived = SessionAuditId::for_target(&target);
    assert_eq!(SessionAuditId::parse(derived.as_str()).unwrap(), derived);
    assert!(derived.as_str().starts_with("session-audit:"));
    assert_eq!(
        SessionAuditId::parse("session-audit:UPPER"),
        Err(SessionAuditError::InvalidIdentity)
    );
    assert_eq!(SpaceName::parse("default").unwrap().to_string(), "default");
}

#[test]
fn summary_evidence_requires_a_lowercase_sha256_digest() {
    use core_command_domain::orchestration::SummaryEvidence;
    assert_eq!(
        SummaryEvidence::new("q.md", "F".repeat(64)).unwrap_err(),
        SummaryQuestionsError::InvalidStructure("invalid SHA-256 content digest".to_string())
    );
    assert_eq!(
        SummaryEvidence::new("q.md", "f".repeat(63)).unwrap_err(),
        SummaryQuestionsError::InvalidStructure("invalid SHA-256 content digest".to_string())
    );
    let evidence = SummaryEvidence::new("q.md", "f".repeat(64)).unwrap();
    assert_eq!(evidence.questions_file(), "q.md");
    assert_eq!(evidence.questions_sha256(), "f".repeat(64));
}

#[test]
fn empty_defaults_read_as_nothing() {
    use core_command_domain::orchestration::{PlanQuestions, TransitionSteps};
    let questions = PlanQuestions::default();
    assert_eq!(questions.answer(), None);
    assert!(!questions.approved());
    assert!(!questions.pending());
    assert_eq!(questions.fingerprint(), None);
    assert_eq!(
        TransitionSteps::default(),
        TransitionSteps::new(Vec::new()).unwrap()
    );
}

#[test]
fn a_stage_node_lists_the_files_it_produces_in_declaration_order() {
    use core_command_domain::workflow_definition::{
        ExecutionKind, PhaseId, StageMode, StageNodeBuilder, StageNumber,
    };
    let node = StageNodeBuilder::new(
        core_command_domain::workflow_definition::StageSlug::parse("code-generation").unwrap(),
        StageNumber::parse("3.5").unwrap(),
        "Code Generation".to_string(),
        PhaseId::Construction,
        ExecutionKind::Always,
        StageMode::Inline,
    )
    .produces(vec![
        "code-generation-plan.md".to_string(),
        "traceability.json".to_string(),
        "code-summary".to_string(),
    ])
    .build();
    let files = node.produced_artifact_files();
    assert_eq!(
        files,
        vec![
            "code-generation-plan.md".to_string(),
            "traceability.json".to_string(),
            "code-summary.md".to_string(),
        ]
    );
}

const H1_AFTER_SUMMARY: &str = "unsupported HTML H1 heading \"<h1>\" after the consolidated summary; only Q<n>, \"Requested Changes Feedback\", or one \"Assumption Confirmation\" section may follow";

#[test]
fn only_an_atx_h2_opens_the_summary_section() {
    // setext 下線・字下げ・HTML 見出しは確認節を開かない。
    for content in [
        "Consolidated Summary Confirmation\n---\n[Answer]: A\n",
        "Consolidated Summary Confirmation\n    ---\n[Answer]: A\n",
        "    <h2>Consolidated Summary Confirmation</h2>\n[Answer]: A\n",
        "<h2>Consolidated Summary Confirmation</h2>\n\n[Answer]: A\n",
        "<h2x>Consolidated Summary Confirmation</h2x>\n\n[Answer]: A\n",
    ] {
        assert_eq!(
            SummaryQuestions::parse(content, "A"),
            Err(SummaryQuestionsError::InvalidAnswer),
            "{content:?}"
        );
    }
}

#[test]
fn an_html_heading_tag_after_the_summary_is_rendered_unless_it_is_a_link_or_code() {
    // リンク先の角括弧やインラインコードの中の `<h1>` は描画されない。
    for content in [
        "## Consolidated Summary Confirmation\n\n[link](<a<h1>b>) text\n[Answer]: A\n",
        "## Consolidated Summary Confirmation\n\n[x](  <h1>) text\n[Answer]: A\n",
        "## Consolidated Summary Confirmation\n\n`code <h1>` text\n[Answer]: A\n",
    ] {
        assert!(SummaryQuestions::parse(content, "A").is_ok(), "{content:?}");
    }
    // 角括弧をエスケープすればリンクではなく、閉じない `<h1` も描画される見出しになる。
    // 二重バッククォートの中に単独のバッククォートがあっても、その外の `<h1>` は隠れない。
    for content in [
        "## Consolidated Summary Confirmation\n\n\\[x](<h1>)\n[Answer]: A\n",
        "## Consolidated Summary Confirmation\n\n[x](<h1\n[Answer]: A\n",
        "## Consolidated Summary Confirmation\n\n``co`de`` <h1>x</h1>\n[Answer]: A\n",
    ] {
        assert_eq!(
            SummaryQuestions::parse(content, "A"),
            Err(SummaryQuestionsError::InvalidStructure(
                H1_AFTER_SUMMARY.to_string()
            )),
            "{content:?}"
        );
    }
}

#[test]
fn comments_raw_html_and_fences_hide_their_body_from_the_answer_scan() {
    for content in [
        "## Consolidated Summary Confirmation\n\n<!-- a\n[Answer]: B\n-->\n[Answer]: A\n",
        "## Consolidated Summary Confirmation\n\n<!-- a -->\n[Answer]: A\n",
        "## Consolidated Summary Confirmation\n\n\\<!-- a\n[Answer]: A\n",
        "## Consolidated Summary Confirmation\n\n<textarea>\n[Answer]: B\n</textarea>\n[Answer]: A\n",
        "## Consolidated Summary Confirmation\n\n<pre>\n## Later\n</pre>\n[Answer]: A\n",
        "## Consolidated Summary Confirmation\n\n[Answer]: A\n\n``` x\n[Answer]: B\n```\n",
        "## Consolidated Summary Confirmation\n\n[Answer]: A\n\n    ```\n    ## Later\n",
        "## Consolidated Summary Confirmation\n\n[Answer]: A\n\n~~~~\n## Later\n~~~\n~~~~\n",
        "## Consolidated Summary Confirmation\n\n- a\n\n  - b\n\n[Answer]: A\n",
        "## Consolidated Summary Confirmation\n\n1. a\n\n\t[Answer]: B\n\n[Answer]: A\n",
    ] {
        assert!(SummaryQuestions::parse(content, "A").is_ok(), "{content:?}");
    }
}

#[test]
fn question_ids_tolerate_a_tab_or_a_suffix_that_is_not_a_separator() {
    // `Q1.x` や `Q1<tab>x` は質問 id ではないので重複しても普通の節として通る。
    let content =
        "## Q1.x\n\n## Q1.x\n\n## Q1\tx\n\n## Consolidated Summary Confirmation\n\n[Answer]: A\n";
    assert!(SummaryQuestions::parse(content, "A").is_ok());
}
