//! 報告事実の書込み形式と検査付き復号。
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use core_command_domain::orchestration::{IntentExecutionEvent, ReportId, ReportRequest, Verdict};
use core_command_interface_adapter::orchestration::IntentExecutionEventDto;

#[expect(
    clippy::disallowed_methods,
    reason = "ドメイン永続化DTOの破損注入であり、公開契約JSONの描画ではない"
)]
#[test]
fn persisted_report_rejects_steps_that_disagree_with_its_transition() {
    let (mut execution, _) = core_command_domain::orchestration::IntentExecution::start(
        support::execution_id(),
        &support::intent(),
        support::at(),
    );
    let event = execution
        .apply_report(
            ReportId::generate(),
            &support::intent(),
            &ReportRequest::new(Verdict::AwaitingApproval, None, None, None, false),
            None,
            support::at(),
        )
        .unwrap();
    let bytes = serde_json::to_vec(&IntentExecutionEventDto::of(&event)).unwrap();
    let mut stored: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    *stored
        .get_mut("Reported")
        .unwrap()
        .get_mut("result")
        .unwrap()
        .get_mut("Committed")
        .unwrap()
        .get_mut("steps")
        .unwrap() = serde_json::Value::Array(vec![serde_json::Value::String("Skip".into())]);
    let corrupted: IntentExecutionEventDto = serde_json::from_value(stored).unwrap();
    assert!(
        corrupted.to_domain().is_err(),
        "別の操作を名乗る報告結果を復号しない"
    );
    assert!(matches!(event, IntentExecutionEvent::Reported(_)));
}
