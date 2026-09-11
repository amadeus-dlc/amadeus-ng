//! セッション監査の適用・対象・再構成を公開集約APIで検査する。
#![allow(clippy::unwrap_used)]
use core_command_domain::workspace::{
    AuditFieldKey, AuditFields, EventType, HookHealthTarget, SessionAudit, SessionAuditObservation,
    SessionAuditRecord, SpaceName,
};
fn observation(space: &str, status: &str) -> SessionAuditObservation {
    SessionAuditObservation::new(
        core_command_domain::workspace::SessionAuditObservationId::generate(),
        HookHealthTarget::new(SpaceName::parse(space).unwrap(), None),
        SessionAuditRecord::new(
            EventType::SubagentCompleted,
            AuditFields::new().with(
                AuditFieldKey::parse("Agent Type").unwrap(),
                "aidlc-developer-agent",
            ),
        )
        .unwrap(),
        status.into(),
    )
}
fn at() -> chrono::DateTime<chrono::Utc> {
    "2026-09-09T00:00:00Z".parse().unwrap()
}
#[test]
fn running_completion_creates_one_fact_and_replays_only_saved_delta() {
    let (mut aggregate, first) = SessionAudit::start(&observation("default", "Running"), at())
        .unwrap()
        .unwrap();
    assert_eq!(aggregate.seq_nr(), 1);
    let snapshot = aggregate.clone();
    let second = aggregate
        .record(&observation("default", "Running"), at())
        .unwrap()
        .unwrap();
    assert_ne!(first.id(), second.id());
    assert_eq!(
        SessionAudit::replay(snapshot, [(second, 2, at())]),
        aggregate
    );
}
#[test]
fn completed_and_unknown_workflows_do_not_create_completion_facts() {
    for status in ["Completed", "", "Paused"] {
        assert!(
            SessionAudit::start(&observation("default", status), at())
                .unwrap()
                .is_none()
        );
    }
}
#[test]
fn ignored_completion_keeps_the_existing_aggregate_unchanged() {
    let (mut aggregate, _) = SessionAudit::start(&observation("default", "Running"), at())
        .unwrap()
        .unwrap();
    let before = aggregate.clone();
    assert!(
        aggregate
            .record(&observation("default", "Completed"), at())
            .unwrap()
            .is_none()
    );
    assert_eq!(aggregate, before);
}
#[test]
fn another_record_cannot_be_mixed_into_a_session_audit() {
    let (mut aggregate, _) = SessionAudit::start(&observation("default", "Running"), at())
        .unwrap()
        .unwrap();
    let before = aggregate.clone();
    assert!(
        aggregate
            .record(&observation("other", "Running"), at())
            .is_err()
    );
    assert_eq!(aggregate, before);
}
#[test]
fn missing_required_and_foreign_audit_fields_are_unconstructible() {
    assert!(SessionAuditRecord::new(EventType::SubagentCompleted, AuditFields::new()).is_err());
    assert!(
        SessionAuditRecord::new(
            EventType::SessionEnded,
            AuditFields::new()
                .with(AuditFieldKey::parse("Reason").unwrap(), "exit")
                .with(AuditFieldKey::parse("Agent Type").unwrap(), "foreign")
        )
        .is_err()
    );
}
#[test]
#[should_panic(expected = "invalid SessionAudit history")]
fn missing_saved_sequence_is_fatal() {
    let (mut aggregate, _) = SessionAudit::start(&observation("default", "Running"), at())
        .unwrap()
        .unwrap();
    let snapshot = aggregate.clone();
    let event = aggregate
        .record(&observation("default", "Running"), at())
        .unwrap()
        .unwrap();
    let _ = SessionAudit::replay(snapshot, [(event, 3, at())]);
}

/// 凍結の拒否行は工具・対象・ステージを必須にし、Unit だけを任意にする。
fn freeze_fields(unit: Option<&str>) -> AuditFields {
    let mut fields = AuditFields::new()
        .with(AuditFieldKey::parse("Tool").unwrap(), "Write")
        .with(
            AuditFieldKey::parse("Target").unwrap(),
            "inception/requirements-analysis/requirements.md",
        )
        .with(
            AuditFieldKey::parse("Stage").unwrap(),
            "requirements-analysis",
        );
    if let Some(unit) = unit {
        fields = fields.with(AuditFieldKey::parse("Unit").unwrap(), unit);
    }
    fields
}

#[test]
fn a_review_freeze_block_carries_its_tool_target_stage_and_optional_unit() {
    let record = SessionAuditRecord::new(EventType::ReviewFreezeBlocked, freeze_fields(None))
        .expect("必須 3 項目だけで構築できる");
    assert_eq!(record.kind(), EventType::ReviewFreezeBlocked);
    assert_eq!(record.fields(), &freeze_fields(None));
    assert!(
        SessionAuditRecord::new(
            EventType::ReviewFreezeBlocked,
            freeze_fields(Some("u2-workflow-authority"))
        )
        .is_ok(),
        "per-unit の書込みは Unit を載せる"
    );
}

#[test]
fn a_review_freeze_block_missing_a_required_field_or_carrying_a_foreign_one_is_unconstructible() {
    let missing_target = AuditFields::new()
        .with(AuditFieldKey::parse("Tool").unwrap(), "Write")
        .with(
            AuditFieldKey::parse("Stage").unwrap(),
            "requirements-analysis",
        );
    assert!(SessionAuditRecord::new(EventType::ReviewFreezeBlocked, missing_target).is_err());
    let foreign = freeze_fields(None).with(AuditFieldKey::parse("Reason").unwrap(), "exit");
    assert!(SessionAuditRecord::new(EventType::ReviewFreezeBlocked, foreign).is_err());
}

#[test]
fn a_review_freeze_block_is_recorded_whatever_the_workflow_status_says() {
    for status in ["Running", "Completed", ""] {
        let observation = SessionAuditObservation::new(
            core_command_domain::workspace::SessionAuditObservationId::generate(),
            HookHealthTarget::new(SpaceName::parse("default").unwrap(), None),
            SessionAuditRecord::new(EventType::ReviewFreezeBlocked, freeze_fields(None)).unwrap(),
            status.into(),
        );
        assert!(
            SessionAudit::start(&observation, at()).unwrap().is_some(),
            "拒否の事実は workflow の status で消えない ({status})"
        );
    }
}

/// 読み取り範囲の拒否行は工具・対象・ステージ・Unit の 4 つをすべて必須にする。
fn scope_fields(with_unit: bool) -> AuditFields {
    let mut fields = AuditFields::new()
        .with(AuditFieldKey::parse("Tool").unwrap(), "Read")
        .with(
            AuditFieldKey::parse("Target").unwrap(),
            "/r/construction/u2-beta/design.md",
        )
        .with(AuditFieldKey::parse("Stage").unwrap(), "functional-design");
    if with_unit {
        fields = fields.with(AuditFieldKey::parse("Unit").unwrap(), "u1-alpha");
    }
    fields
}

#[test]
fn a_reviewer_scope_block_carries_its_tool_target_stage_and_unit() {
    let record = SessionAuditRecord::new(EventType::ReviewerScopeBlocked, scope_fields(true))
        .expect("必須 4 項目で構築できる");
    assert_eq!(record.kind(), EventType::ReviewerScopeBlocked);
    assert_eq!(record.fields(), &scope_fields(true));
}

#[test]
fn a_reviewer_scope_block_without_a_unit_is_unconstructible() {
    // 差し向け記録は必ず Unit を名乗るので、Unit の無い拒否行は存在しない
    // (upstream `emitReviewerScopeBlocked` も 4 項目を常に渡す)。
    assert!(SessionAuditRecord::new(EventType::ReviewerScopeBlocked, scope_fields(false)).is_err());
    let foreign = scope_fields(true).with(AuditFieldKey::parse("Reason").unwrap(), "exit");
    assert!(SessionAuditRecord::new(EventType::ReviewerScopeBlocked, foreign).is_err());
}

#[test]
fn a_reviewer_scope_block_is_recorded_whatever_the_workflow_status_says() {
    for status in ["Running", "Completed", ""] {
        let observation = SessionAuditObservation::new(
            core_command_domain::workspace::SessionAuditObservationId::generate(),
            HookHealthTarget::new(SpaceName::parse("default").unwrap(), None),
            SessionAuditRecord::new(EventType::ReviewerScopeBlocked, scope_fields(true)).unwrap(),
            status.into(),
        );
        assert!(
            SessionAudit::start(&observation, at()).unwrap().is_some(),
            "拒否の事実は workflow の status で消えない ({status})"
        );
    }
}
