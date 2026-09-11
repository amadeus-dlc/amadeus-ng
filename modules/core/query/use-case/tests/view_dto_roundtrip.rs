//! ドメイン非依存の View（読取り専用 DTO）が、行の値を**変換せずそのまま**返す契約。
//!
//! View は DAO が引いた 1 行の写しである。ここで固定するのは「与えた列がその列の
//! 取得口から逐語で戻ること」「`Option` の不在が不在のまま伝わること」であり、
//! 値の解釈や既定値の補完を View がしないことを主張する。

// テストコードでは unwrap / expect を許可 (オーナー規約)。
#![allow(clippy::unwrap_used, clippy::expect_used)]

use core_query_use_case::orchestration::{
    AnswerResultView, ArtifactAuditView, ExecutionView, HookHealthView, IntentRecordView,
    PlanGenerationView, ReportResultView, SessionAuditView, TestingContractView,
};

fn s(text: &str) -> String {
    text.to_string()
}

#[test]
fn an_artifact_audit_view_returns_every_column_verbatim() {
    let view = ArtifactAuditView::new(
        s("aa-1"),
        s("/r/intent"),
        s("plan.md"),
        s("Write"),
        s("code-generation"),
        true,
        s("2026-09-11T00:00:00Z"),
    );
    assert_eq!(view.id(), "aa-1");
    assert_eq!(view.target(), "/r/intent");
    assert_eq!(view.file(), "plan.md");
    assert_eq!(view.tool(), "Write");
    assert_eq!(view.context(), "code-generation");
    assert!(view.created());
    assert_eq!(view.occurred_at(), "2026-09-11T00:00:00Z");
    assert_eq!(view.clone(), view);
}

#[test]
fn a_session_audit_view_returns_every_column_verbatim() {
    let view = SessionAuditView::new(s("sa-1"), s("/r/intent"), s("SESSION_START"));
    assert_eq!(view.id(), "sa-1");
    assert_eq!(view.target(), "/r/intent");
    assert_eq!(view.kind(), "SESSION_START");
}

#[test]
fn a_hook_health_view_keeps_absent_heartbeat_and_drop_absent() {
    let absent = HookHealthView::new(s("hh-1"), s("/r"), s("review-freeze"), None, 0, 0, None);
    assert_eq!(absent.id(), "hh-1");
    assert_eq!(absent.target(), "/r");
    assert_eq!(absent.hook(), "review-freeze");
    assert_eq!(absent.heartbeat(), None);
    assert_eq!(absent.seq_nr(), 0);
    assert_eq!(absent.drops(), 0);
    assert_eq!(absent.latest_drop(), None);
    let present = HookHealthView::new(
        s("hh-2"),
        s("/r"),
        s("review-freeze"),
        Some(s("2026-09-11T00:00:00Z")),
        7,
        2,
        Some(s("uninspected")),
    );
    assert_eq!(present.heartbeat(), Some("2026-09-11T00:00:00Z"));
    assert_eq!(present.seq_nr(), 7);
    assert_eq!(present.drops(), 2);
    assert_eq!(present.latest_drop(), Some("uninspected"));
}

#[test]
fn an_execution_view_defaults_its_projected_flags_and_accepts_them_by_builder() {
    let view = ExecutionView::new(
        s("exec-1"),
        s("intent-1"),
        s("feature"),
        s("running"),
        Some(s("code-generation")),
        None,
        false,
        s("binding"),
    );
    assert_eq!(view.execution_id(), "exec-1");
    assert_eq!(view.intent_id(), "intent-1");
    assert_eq!(view.scope(), "feature");
    assert_eq!(view.status(), "running");
    assert_eq!(view.cursor_slug(), Some("code-generation"));
    assert_eq!(view.parked_at_slug(), None);
    assert!(!view.parked_active());
    assert_eq!(view.state_binding(), "binding");
    assert!(!view.first_substantive_run(), "投影前の既定は偽");
    assert_eq!(view.continuation_wait(), None, "投影前の既定は不在");
    let projected = view
        .clone()
        .with_first_substantive_run(true)
        .with_continuation_wait(Some(s("awaiting-approval")));
    assert!(projected.first_substantive_run());
    assert_eq!(projected.continuation_wait(), Some("awaiting-approval"));
    let cleared = projected.with_continuation_wait(None);
    assert_eq!(cleared.continuation_wait(), None);
    assert_ne!(cleared, view, "投影済みの旗は等価性に含まれる");
}

#[test]
fn a_report_result_view_returns_every_column_verbatim() {
    let view = ReportResultView::new(
        s("exec-1"),
        s("code-generation"),
        s("feature"),
        s("completed"),
        s("[\"advance\"]"),
        None,
        Some(s("build-and-test")),
    );
    assert_eq!(view.execution_id(), "exec-1");
    assert_eq!(view.stage(), "code-generation");
    assert_eq!(view.scope(), "feature");
    assert_eq!(view.result_kind(), "completed");
    assert_eq!(view.steps(), "[\"advance\"]");
    assert_eq!(view.no_op_reason(), None);
    assert_eq!(view.current_stage(), Some("build-and-test"));
    let no_op = ReportResultView::new(
        s("exec-1"),
        s("code-generation"),
        s("feature"),
        s("no-op"),
        s("[]"),
        Some(s("already completed")),
        None,
    );
    assert_eq!(no_op.no_op_reason(), Some("already completed"));
    assert_eq!(no_op.current_stage(), None);
}

#[test]
fn a_plan_generation_view_returns_every_column_verbatim() {
    let view = PlanGenerationView::new(s("pg-1"), s("generated"), Some(s("u2")), None, 42);
    assert_eq!(view.id(), "pg-1");
    assert_eq!(view.status(), "generated");
    assert_eq!(view.unit(), Some("u2"));
    assert_eq!(view.error(), None);
    assert_eq!(view.as_of(), 42);
    let failed = PlanGenerationView::new(s("pg-2"), s("failed"), None, Some(s("boom")), 0);
    assert_eq!(failed.unit(), None);
    assert_eq!(failed.error(), Some("boom"));
    assert_eq!(failed.as_of(), 0);
}

#[test]
fn a_testing_contract_view_keeps_each_optional_column_independent() {
    let resolved = TestingContractView::new(Some(s("tdd")), Some(s("## Testing Contract")), None);
    assert_eq!(resolved.contract(), Some("tdd"));
    assert_eq!(resolved.rendered(), Some("## Testing Contract"));
    assert_eq!(resolved.error(), None);
    let failed = TestingContractView::new(None, None, Some(s("posture missing")));
    assert_eq!(failed.contract(), None);
    assert_eq!(failed.rendered(), None);
    assert_eq!(failed.error(), Some("posture missing"));
}

#[test]
fn an_answer_result_view_returns_every_column_verbatim() {
    let view = AnswerResultView::new(s("exec-1"), s("code-generation"), s("approved"));
    assert_eq!(view.execution_id(), "exec-1");
    assert_eq!(view.stage(), "code-generation");
    assert_eq!(view.disposition(), "approved");
}

#[test]
fn an_intent_record_view_returns_every_column_verbatim() {
    let view = IntentRecordView::new(
        s("260907-selfhost-stage1"),
        s("/r/aidlc/spaces/default/intents/260907-selfhost-stage1"),
    );
    assert_eq!(view.intent_id(), "260907-selfhost-stage1");
    assert_eq!(
        view.directory(),
        "/r/aidlc/spaces/default/intents/260907-selfhost-stage1"
    );
}
