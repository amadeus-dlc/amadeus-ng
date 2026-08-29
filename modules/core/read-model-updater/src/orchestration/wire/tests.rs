//! ワイヤ形式のバイトを**逐語で固定**する (読む側)。
//!
//! ここに書かれた JSON は改訂 9 の直前（ドメインが serde を持っていた時点）に実測した
//! 出力そのものであり、**書く側の同名テストと同じリテラル**である。両側が同じ綴りを
//! 独立に固定しているので、どちらかの綴りが動けばどちらかのテストが落ちる — これが
//! 「型を共有せずにワイヤ形式の一致を保つ」ための単体側の歯止めである
//! (横断の歯止めは `journal_protocol_conformance` とゴールデンパリティ)。

#![allow(
    clippy::panic,
    reason = "想定外ケースの即時失敗はテストの検証手段である (house style)"
)]

use core_command_domain::orchestration::{
    AutonomyMode, AutonomyModeSet, GateApproved, GateOpened, GateRejected, Intent,
    IntentExecutionEvent, IntentId, JumpDirection, Jumped, Parked, PhaseBoundary, Recomposed,
    StageCompleted, StageDisplay, StageEntry, StageRevised, StageSkipped, StartRequest, Started,
    WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
    WorkflowDefinitionId,
};

use super::WireEvent;

const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";
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

#[expect(
    clippy::disallowed_methods,
    reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
)]
#[test]
fn every_event_variant_serialises_to_the_recorded_bytes() {
    // 書く側と**同じリテラル**である。両側が独立に同じ綴りを固定しているので、
    // 片側だけが動けばここか向こうが落ちる。
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

#[test]
fn a_row_whose_spelling_is_outside_the_closed_set_is_refused() {
    // 閉集合外の綴りは推測せずに拒む。ドメインへ写す前に止まるので、壊れた値が投影核に入らない。
    let (_, started) = every_variant().swap_remove(0);
    let tampered = started.replace(r#""phase":"Ideation""#, r#""phase":"ideation""#);
    let decoded: WireEvent = serde_json::from_str(&tampered).expect("JSON としては読める");
    assert!(decoded.to_domain().is_err(), "小文字の phase は閉集合の外");
}

#[test]
fn a_row_that_breaks_an_aggregate_invariant_is_refused_at_the_check_point() {
    // 形は読めるが Always Valid を破る行は `Intent::from_material` の検査点で止まる。
    let (_, started) = every_variant().swap_remove(0);
    // 先頭ステージ (initialization) を SKIP に畳むと Always Valid を破る。
    let tampered = started.replacen(r#""plan_action":"Execute""#, r#""plan_action":"Skip""#, 1);
    let decoded: WireEvent = serde_json::from_str(&tampered).expect("JSON としては読める");
    assert!(
        decoded.to_domain().is_err(),
        "initialization ステージの SKIP は不変条件違反"
    );
}

#[test]
fn a_malformed_identifier_is_refused_with_its_field() {
    let (_, started) = every_variant().swap_remove(0);
    for (from, to) in [
        (
            r#""id":"01a02785-1bd8-76eb-aeea-5aa303ebd5b6""#,
            r#""id":"not-a-uuid""#,
        ),
        (r#""definition_id":"claude""#, r#""definition_id":"""#),
        (
            r#""definition_revision":"sha256:0000000000000000000000000000000000000000000000000000000000000000""#,
            r#""definition_revision":"md5:0""#,
        ),
        (r#""number":"0.1""#, r#""number":"zero""#),
        (r#""slug":"state-init""#, r#""slug":"Not A Slug""#),
        (
            r#""project_type":"greenfield""#,
            r#""project_type":"Greenfield""#,
        ),
        (r#""plan_action":"Execute""#, r#""plan_action":"EXECUTE""#),
    ] {
        let tampered = started.replacen(from, to, 1);
        let decoded: WireEvent = serde_json::from_str(&tampered).expect("JSON としては読める");
        assert!(decoded.to_domain().is_err(), "拒むべき値: {to}");
    }
}

#[test]
fn a_malformed_stage_reference_in_any_variant_is_refused() {
    // ステージ参照は多くの変種に現れる。どの変種でも同じ検査を通ることを固定する。
    for (_, expected) in every_variant() {
        if !expected.contains(r#""stage":"#)
            && !expected.contains(r#""source":"#)
            && !expected.contains(r#""skipped":"#)
        {
            continue;
        }
        let tampered = expected
            .replace(r#""intent-capture""#, r#""Not A Slug""#)
            .replace(r#""state-init""#, r#""Not A Slug""#)
            .replace(r#""scope-definition""#, r#""Not A Slug""#);
        let decoded: WireEvent = serde_json::from_str(&tampered).expect("JSON としては読める");
        assert!(decoded.to_domain().is_err(), "拒むべき行: {tampered}");
    }
}

#[test]
fn a_malformed_closed_set_value_in_a_control_variant_is_refused() {
    for (from, to) in [
        (r#""direction":"Forward""#, r#""direction":"forward""#),
        (r#""mode":"Autonomous""#, r#""mode":"autonomous""#),
        (r#""from_phase":"Ideation""#, r#""from_phase":"ideation""#),
        (r#""to_phase":"Inception""#, r#""to_phase":"inception""#),
    ] {
        let row = every_variant()
            .into_iter()
            .map(|(_, json)| json)
            .find(|json| json.contains(from))
            .expect("その綴りを含む変種がある");
        let tampered = row.replace(from, to);
        let decoded: WireEvent = serde_json::from_str(&tampered).expect("JSON としては読める");
        assert!(decoded.to_domain().is_err(), "拒むべき値: {to}");
    }
}

#[test]
fn an_optional_request_field_round_trips_when_present() {
    // `depth` / `test_strategy` は省略可能で、`Started` の材料としてだけ運ばれる
    // (集約状態にはならない)。両方が載った行も読めることを固定する。
    let (_, started) = every_variant().swap_remove(0);
    let filled = started.replacen(
        r#""test_strategy":null"#,
        r#""test_strategy":"balanced""#,
        1,
    );
    let decoded: WireEvent = serde_json::from_str(&filled).expect("記録済みの行は読める");
    let IntentExecutionEvent::Started(payload) = decoded.to_domain().expect("ドメインへ戻せる")
    else {
        panic!("Started を期待した");
    };
    assert_eq!(payload.intent().test_strategy(), Some("balanced"));
    assert_eq!(payload.intent().depth(), Some("standard"));
}
