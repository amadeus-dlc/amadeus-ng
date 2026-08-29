//! ワイヤ形式のバイトを**逐語で固定**する。
//!
//! ここに書かれた JSON は改訂 9 の直前（ドメインが serde を持っていた時点）に実測した
//! 出力そのものである。DTO へ移してもバイトが 1 文字も変わっていないことが、この逐語一致で
//! 証明される。行に書かれて残る値なので、期待値を書き換えるときは移行の要否を考えること。

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    AutonomyMode, AutonomyModeSet, GateApproved, GateOpened, GateRejected, Intent, IntentExecution,
    IntentExecutionEvent, IntentExecutionId, IntentId, JumpDirection, Jumped, Parked,
    PhaseBoundary, Recomposed, StageCompleted, StageDisplay, StageEntry, StageRevised,
    StageSkipped, StartRequest, Started, WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
    WorkflowDefinitionId,
};

use super::{WireEvent, WireSnapshot};

const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";
const EXECUTION: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";
const AT_TEXT: &str = "2026-08-23T00:00:00Z";

fn at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(AT_TEXT)
        .expect("固定の ISO 8601 UTC")
        .with_timezone(&Utc)
}

fn slug(value: &str) -> StageSlug {
    StageSlug::parse(value).expect("文法内の slug")
}

fn display(number: &str, name: &str) -> StageDisplay {
    StageDisplay::new(
        StageNumber::parse(number).expect("文法内のステージ番号"),
        name,
        "orchestrator",
    )
    .expect("単一行")
}

fn stages() -> Vec<StageEntry> {
    vec![
        StageEntry::new(
            slug("state-init"),
            PhaseId::Initialization,
            PlanAction::Execute,
            false,
            display("0.1", "State Init"),
        ),
        StageEntry::new(
            slug("intent-capture"),
            PhaseId::Ideation,
            PlanAction::Execute,
            false,
            display("1.1", "Intent Capture"),
        ),
        StageEntry::new(
            slug("scope-definition"),
            PhaseId::Ideation,
            PlanAction::Execute,
            false,
            display("1.4", "Scope Definition"),
        ),
    ]
}

fn intent() -> Intent {
    Intent::from_material(
        IntentId::parse(INTENT).expect("UUIDv7"),
        WorkflowDefinitionId::parse("claude").expect("定義 id"),
        DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision"),
        StartRequest::new("classic", "contract").with_depth("standard"),
        stages(),
        WorkspaceScan::new(
            BrownfieldGreenfield::Greenfield,
            "Unknown",
            "Unknown",
            "Unknown",
        )
        .expect("単一行"),
    )
    .expect("合成計画は Intent の不変条件を満たす")
}

/// 全 12 変種を、逐語で固定した綴りと組で並べる。
fn every_variant() -> Vec<(IntentExecutionEvent, &'static str)> {
    vec![
        (
            IntentExecutionEvent::Started(Started::new(intent())),
            r#"{"Started":{"intent":{"id":"01a02785-1bd8-76eb-aeea-5aa303ebd5b6","definition_id":"claude","definition_revision":"sha256:0000000000000000000000000000000000000000000000000000000000000000","start_request":{"scope":"classic","request":"contract","depth":"standard","test_strategy":null},"stages":[{"slug":"state-init","phase":"Initialization","plan_action":"Execute","conditional":false,"display":{"number":"0.1","name":"State Init","lead_agent":"orchestrator"}},{"slug":"intent-capture","phase":"Ideation","plan_action":"Execute","conditional":false,"display":{"number":"1.1","name":"Intent Capture","lead_agent":"orchestrator"}},{"slug":"scope-definition","phase":"Ideation","plan_action":"Execute","conditional":false,"display":{"number":"1.4","name":"Scope Definition","lead_agent":"orchestrator"}}],"scan":{"project_type":"greenfield","languages":"Unknown","frameworks":"Unknown","build_system":"Unknown"}}}}"#,
        ),
        (
            IntentExecutionEvent::StageCompleted(StageCompleted::new(
                slug("state-init"),
                Some(slug("intent-capture")),
            )),
            r#"{"StageCompleted":{"stage":"state-init","next_stage":"intent-capture"}}"#,
        ),
        (
            IntentExecutionEvent::GateOpened(GateOpened::new(
                slug("intent-capture"),
                vec!["a.md".to_string()],
            )),
            r#"{"GateOpened":{"stage":"intent-capture","artifacts":["a.md"]}}"#,
        ),
        (
            IntentExecutionEvent::GateApproved(GateApproved::new(
                slug("intent-capture"),
                Some("ok".to_string()),
                Some(slug("scope-definition")),
                Some(PhaseBoundary::new(PhaseId::Ideation, PhaseId::Inception)),
            )),
            r#"{"GateApproved":{"stage":"intent-capture","user_input":"ok","next_stage":"scope-definition","phase_boundary":{"from_phase":"Ideation","to_phase":"Inception"}}}"#,
        ),
        (
            IntentExecutionEvent::GateRejected(GateRejected::new(
                slug("intent-capture"),
                Some("why".to_string()),
                2,
            )),
            r#"{"GateRejected":{"stage":"intent-capture","feedback":"why","revision_count":2}}"#,
        ),
        (
            IntentExecutionEvent::StageRevised(StageRevised::new(slug("intent-capture"))),
            r#"{"StageRevised":{"stage":"intent-capture"}}"#,
        ),
        (
            IntentExecutionEvent::StageSkipped(StageSkipped::new(
                slug("intent-capture"),
                "not needed".to_string(),
                Some(slug("scope-definition")),
            )),
            r#"{"StageSkipped":{"stage":"intent-capture","reason":"not needed","next_stage":"scope-definition"}}"#,
        ),
        (
            IntentExecutionEvent::Jumped(Jumped::new(
                JumpDirection::Forward,
                slug("state-init"),
                slug("intent-capture"),
                vec![slug("scope-definition")],
                vec![slug("intent-capture")],
            )),
            r#"{"Jumped":{"direction":"Forward","source":"state-init","target":"intent-capture","stages_reset":["scope-definition"],"stages_skipped":["intent-capture"]}}"#,
        ),
        (
            IntentExecutionEvent::Parked(Parked::new(slug("intent-capture"))),
            r#"{"Parked":{"stage":"intent-capture"}}"#,
        ),
        (IntentExecutionEvent::Unparked, r#""Unparked""#),
        (
            IntentExecutionEvent::Recomposed(Recomposed::new(
                vec![slug("scope-definition")],
                vec![slug("intent-capture")],
                vec![slug("state-init")],
            )),
            r#"{"Recomposed":{"skipped":["scope-definition"],"added":["intent-capture"],"stages_in_scope":["state-init"]}}"#,
        ),
        (
            IntentExecutionEvent::AutonomyModeSet(AutonomyModeSet::new(AutonomyMode::Autonomous)),
            r#"{"AutonomyModeSet":{"mode":"Autonomous"}}"#,
        ),
    ]
}

/// スナップショット行の逐語形 (genesis 直後)。
const GENESIS_SNAPSHOT: &str = r#"{"id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","intent_id":"01a02785-1bd8-76eb-aeea-5aa303ebd5b6","overlay":["Execute","Execute","Execute"],"checkbox":["InProgress","Pending","Pending"],"cursor":0,"status":"Running","parked_at":null,"autonomy":"Gated","approved":[false,false,false],"revision_count":[0,0,0],"seq_nr":1,"last_updated_at":"2026-08-23T00:00:00Z"}"#;

#[expect(
    clippy::disallowed_methods,
    reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
)]
#[test]
fn every_event_variant_serialises_to_the_recorded_bytes() {
    for (event, expected) in every_variant() {
        let json = serde_json::to_string(&WireEvent::of(&event)).expect("DTO は直列化できる");
        assert_eq!(json, expected, "変種のワイヤ形式が変わった");
    }
}

#[test]
fn every_event_variant_round_trips_through_the_wire() {
    for (event, expected) in every_variant() {
        let decoded: WireEvent = serde_json::from_str(expected).expect("記録済みの行は読める");
        assert_eq!(decoded.to_domain().expect("ドメインへ戻せる"), event);
    }
}

#[expect(
    clippy::disallowed_methods,
    reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
)]
#[test]
fn the_snapshot_serialises_to_the_recorded_bytes_and_round_trips() {
    let (aggregate, _) = IntentExecution::start(
        IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
        intent(),
        at(),
    );
    let json = serde_json::to_string(&WireSnapshot::of(&aggregate)).expect("DTO は直列化できる");
    assert_eq!(
        json, GENESIS_SNAPSHOT,
        "スナップショットのワイヤ形式が変わった"
    );

    let decoded: WireSnapshot =
        serde_json::from_str(GENESIS_SNAPSHOT).expect("記録済みの行は読める");
    assert_eq!(decoded.to_domain().expect("ドメインへ戻せる"), aggregate);
}

#[test]
fn a_row_whose_spelling_is_outside_the_closed_set_is_refused() {
    // 閉集合外の綴りは推測せずに拒む。ドメインへ写す前に止まるので、壊れた値が集約に入らない。
    let tampered = GENESIS_SNAPSHOT.replace(r#""status":"Running""#, r#""status":"running""#);
    let decoded: WireSnapshot = serde_json::from_str(&tampered).expect("JSON としては読める");
    assert!(decoded.to_domain().is_err(), "小文字の status は閉集合の外");
}

#[test]
fn a_row_that_breaks_an_aggregate_invariant_is_refused_at_the_check_point() {
    // 形は読めるが不変条件を破る行は `from_snapshot` の検査点で止まる — 担保の場所が
    // ドメインの serde 属性からこの層の変換関数へ移っただけで、担保自体は落ちていない。
    let tampered = GENESIS_SNAPSHOT.replace(r#""cursor":0"#, r#""cursor":99"#);
    let decoded: WireSnapshot = serde_json::from_str(&tampered).expect("JSON としては読める");
    assert!(decoded.to_domain().is_err(), "範囲外カーソルは不変条件違反");
}
