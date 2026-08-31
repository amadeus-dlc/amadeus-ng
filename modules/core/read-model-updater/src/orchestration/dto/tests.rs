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
    AutonomyMode, AutonomyModeSet, Created, GateApproved, GateOpened, GateRejected, Intent,
    IntentExecutionEvent, IntentId, Jumped, Parked, Recomposed, StageCompleted, StageDisplay,
    StageEntry, StageRevised, StageSkipped, StartRequest, Started, WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
    WorkflowDefinitionId,
};

use super::{IntentEventDto, IntentExecutionEventDto};

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
    Intent::from(Created::new(
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
    ))
}

/// intent ジャーナル行の逐語 (issue #56 — 計画・表示属性・走査結果はこの面が正本)。
///
/// 書く側 (command interface-adapter の `IntentEventDto`) と同じバイトであることは
/// 横断適合テスト (`journal_protocol_conformance`) が固定する。
const INTENT_ROW: &str = r#"{"Created":{"id":"01a02785-1bd8-76eb-aeea-5aa303ebd5b6","definition_id":"claude","definition_revision":"sha256:0000000000000000000000000000000000000000000000000000000000000000","start_request":{"scope":"classic","request":"contract","depth":"standard","test_strategy":null},"stages":[{"slug":"state-init","phase":"Initialization","plan_action":"Execute","conditional":false,"display":{"number":"0.1","name":"State Init","lead_agent":"orchestrator"}},{"slug":"intent-capture","phase":"Ideation","plan_action":"Execute","conditional":false,"display":{"number":"1.1","name":"Intent Capture","lead_agent":"orchestrator"}},{"slug":"scope-definition","phase":"Ideation","plan_action":"Execute","conditional":false,"display":{"number":"1.4","name":"Scope Definition","lead_agent":"orchestrator"}}],"scan":{"project_type":"greenfield","languages":"Unknown","frameworks":"Unknown","build_system":"Unknown"}}}"#;

/// 全 12 変種を、逐語で固定した綴りと組で並べる。
fn every_variant() -> Vec<(IntentExecutionEvent, &'static str)> {
    vec![
        (
            IntentExecutionEvent::Started(Started::new(
                IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("UUIDv7"),
            )),
            r#"{"Started":{"intent_id":"01a02785-1bd8-76eb-aeea-5aa303ebd5b6"}}"#,
        ),
        (
            IntentExecutionEvent::StageCompleted(StageCompleted::new(slug("state-init"))),
            r#"{"StageCompleted":{"stage":"state-init"}}"#,
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
            )),
            r#"{"GateApproved":{"stage":"intent-capture","user_input":"ok"}}"#,
        ),
        (
            IntentExecutionEvent::GateRejected(GateRejected::new(
                slug("intent-capture"),
                Some("why".to_string()),
            )),
            r#"{"GateRejected":{"stage":"intent-capture","feedback":"why"}}"#,
        ),
        (
            IntentExecutionEvent::StageRevised(StageRevised::new(slug("intent-capture"))),
            r#"{"StageRevised":{"stage":"intent-capture"}}"#,
        ),
        (
            IntentExecutionEvent::StageSkipped(StageSkipped::new(
                slug("intent-capture"),
                "not needed".to_string(),
            )),
            r#"{"StageSkipped":{"stage":"intent-capture","reason":"not needed"}}"#,
        ),
        (
            IntentExecutionEvent::Jumped(Jumped::new(slug("intent-capture"))),
            r#"{"Jumped":{"target":"intent-capture"}}"#,
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
            )),
            r#"{"Recomposed":{"skipped":["scope-definition"],"added":["intent-capture"]}}"#,
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
        let json = serde_json::to_string(&IntentExecutionEventDto::of(&event))
            .expect("DTO は直列化できる");
        assert_eq!(json, expected, "変種のワイヤ形式が変わった");
    }
}

#[test]
fn every_event_variant_round_trips_through_the_wire() {
    for (event, expected) in every_variant() {
        let decoded: IntentExecutionEventDto =
            serde_json::from_str(expected).expect("記録済みの行は読める");
        assert_eq!(decoded.to_domain().expect("ドメインへ戻せる"), event);
    }
}

#[test]
fn a_row_whose_spelling_is_outside_the_closed_set_is_refused() {
    // 閉集合外の綴りは推測せずに拒む。ドメインへ写す前に止まるので、壊れた値が投影核に入らない
    // (intent ジャーナル面 — 計画の綴りは誕生の材料が正本である。issue #56)。
    let tampered = INTENT_ROW.replace(r#""phase":"Ideation""#, r#""phase":"ideation""#);
    let decoded: IntentEventDto = serde_json::from_str(&tampered).expect("JSON としては読める");
    assert!(decoded.to_domain().is_err(), "小文字の phase は閉集合の外");
}

#[test]
#[should_panic(expected = "recorded history violates the plan invariants")]
fn a_row_that_breaks_an_aggregate_invariant_crashes_reconstruction() {
    // 形は読めるが Always Valid を破る行 — 再構成は失敗を返さず、壊れた歴史はクラッシュが正
    // (オーナー裁定 2026-08-30)。
    // 先頭ステージ (initialization) を SKIP に畳むと Always Valid を破る。
    let tampered = INTENT_ROW.replacen(r#""plan_action":"Execute""#, r#""plan_action":"Skip""#, 1);
    let decoded: IntentEventDto = serde_json::from_str(&tampered).expect("JSON としては読める");
    let _ = decoded.to_domain();
}

#[test]
fn a_malformed_identifier_is_refused_with_its_field() {
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
        let tampered = INTENT_ROW.replacen(from, to, 1);
        let decoded: IntentEventDto = serde_json::from_str(&tampered).expect("JSON としては読める");
        assert!(decoded.to_domain().is_err(), "拒むべき値: {to}");
    }
}

#[test]
fn a_started_row_with_a_malformed_intent_id_is_refused() {
    let decoded: IntentExecutionEventDto =
        serde_json::from_str(r#"{"Started":{"intent_id":"not-a-uuid"}}"#)
            .expect("JSON としては読める");
    assert!(decoded.to_domain().is_err(), "文法外の intent 識別子は拒む");
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
        let decoded: IntentExecutionEventDto =
            serde_json::from_str(&tampered).expect("JSON としては読める");
        assert!(decoded.to_domain().is_err(), "拒むべき行: {tampered}");
    }
}

#[test]
fn a_malformed_closed_set_value_in_a_control_variant_is_refused() {
    // direction / phase_boundary はイベントから消えた (導出 — 2026-08-30) ため、実行の
    // ジャーナル面に残る閉集合は mode だけである (計画の閉集合は intent ジャーナル面 —
    // `a_malformed_identifier_is_refused_with_its_field`)。
    let (from, to) = (r#""mode":"Autonomous""#, r#""mode":"autonomous""#);
    {
        let row = every_variant()
            .into_iter()
            .map(|(_, json)| json)
            .find(|json| json.contains(from))
            .expect("その綴りを含む変種がある");
        let tampered = row.replace(from, to);
        let decoded: IntentExecutionEventDto =
            serde_json::from_str(&tampered).expect("JSON としては読める");
        assert!(decoded.to_domain().is_err(), "拒むべき値: {to}");
    }
}

#[test]
fn an_optional_request_field_round_trips_when_present() {
    // `depth` / `test_strategy` は省略可能で、intent の誕生の材料としてだけ運ばれる
    // (集約状態にはならない)。両方が載った行も読めることを固定する (intent ジャーナル面 —
    // issue #56 で `Started` からこの面へ移った)。
    let row = INTENT_ROW.replacen(
        r#""test_strategy":null"#,
        r#""test_strategy":"balanced""#,
        1,
    );
    let decoded: IntentEventDto = serde_json::from_str(&row).expect("記録済みの行は読める");
    let intent = decoded.to_domain().expect("ドメインへ戻せる");
    assert_eq!(intent.test_strategy(), Some("balanced"));
    assert_eq!(intent.depth(), Some("standard"));
}

#[test]
fn a_malformed_stage_reference_in_a_list_variant_is_refused() {
    // 列の中の 1 本でも文法外の slug は復号を止める (slugs_of の失敗面)。
    let tampered = r#"{"Recomposed":{"skipped":["NOT A SLUG"],"added":[]}}"#;
    let decoded: IntentExecutionEventDto =
        serde_json::from_str(tampered).expect("JSON としては読める");
    assert!(decoded.to_domain().is_err());
}

#[expect(
    clippy::disallowed_methods,
    reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
)]
#[test]
fn the_intent_journal_row_serialises_to_the_recorded_bytes_and_round_trips() {
    let json = serde_json::to_string(&IntentEventDto::of(&intent())).expect("DTO は直列化できる");
    assert_eq!(json, INTENT_ROW, "intent ジャーナルのワイヤ形式が変わった");

    let decoded: IntentEventDto = serde_json::from_str(INTENT_ROW).expect("記録済みの行は読める");
    assert_eq!(
        decoded.to_domain().expect("ドメインへ戻せる"),
        intent(),
        "誕生の材料は検査付き再構成で集約値へ戻る"
    );
}

#[test]
fn a_malformed_intent_row_is_refused() {
    let broken = INTENT_ROW.replacen(
        r#""id":"01a02785-1bd8-76eb-aeea-5aa303ebd5b6""#,
        r#""id":"not-a-uuid""#,
        1,
    );
    let decoded: IntentEventDto = serde_json::from_str(&broken).expect("形は DTO として読める");
    assert!(decoded.to_domain().is_err(), "識別子の文法違反は拒否");
}
