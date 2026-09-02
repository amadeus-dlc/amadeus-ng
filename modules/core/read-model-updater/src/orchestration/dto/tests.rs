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
    IntentEvent, IntentEventId, IntentExecutionEvent, IntentExecutionEventId, IntentExecutionId,
    IntentId, Jumped, Parked, Recomposed, StageCompleted, StageDisplay, StageEntry, StageRevised,
    StageSkipped, StartRequest, Started, Unparked, WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
    WorkflowDefinitionId,
};

use super::{DtoDecodeError, IntentEventDto, IntentExecutionEventDto};

/// b40 のテスト用固定イベント識別子 (同じ材料から組んだイベントを同値に保つため)。
fn event_id() -> IntentExecutionEventId {
    IntentExecutionEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002").expect("UUIDv7")
}

/// b40 のテスト用集約識別子 (行の `aid` と payload の `aggregate_id` を揃える)。
fn execution_id() -> IntentExecutionId {
    IntentExecutionId::parse(EXECUTION).expect("UUIDv7")
}

/// b40 のテスト用固定イベント識別子 (intent 面)。
fn intent_event_id() -> IntentEventId {
    IntentEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0001").expect("UUIDv7")
}

const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";
const EXECUTION: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";

fn at() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2026-08-23T00:00:00Z")
        .expect("固定時刻")
        .with_timezone(&chrono::Utc)
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

/// intent の誕生イベント (行を組む材料)。
fn created_event() -> IntentEvent {
    IntentEvent::Created(created())
}

fn intent() -> Intent {
    Intent::from((created(), at()))
}

fn created() -> Created {
    Created::new(
        intent_event_id(),
        IntentId::parse(INTENT).expect("UUIDv7"),
        WorkflowDefinitionId::parse("claude").expect("定義 id"),
        DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision"),
        StartRequest::new("classic", "contract")
            .with_depth("standard")
            .with_review("adversarial"),
        stages(),
        WorkspaceScan::new(
            BrownfieldGreenfield::Greenfield,
            "Unknown",
            "Unknown",
            "Unknown",
        )
        .expect("単一行"),
    )
}

/// intent ジャーナル行の逐語 (issue #56 — 計画・表示属性・走査結果はこの面が正本)。
///
/// 書く側 (command interface-adapter の `IntentEventDto`) と同じバイトであることは
/// 横断適合テスト (`journal_protocol_conformance`) が固定する。
const INTENT_ROW: &str = r#"{"Created":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0001","aggregate_id":"01a02785-1bd8-76eb-aeea-5aa303ebd5b6","definition_id":"claude","definition_revision":"sha256:0000000000000000000000000000000000000000000000000000000000000000","start_request":{"scope":"classic","request":"contract","depth":"standard","test_strategy":null,"review":"adversarial"},"stages":[{"slug":"state-init","phase":"Initialization","plan_action":"Execute","conditional":false,"display":{"number":"0.1","name":"State Init","lead_agent":"orchestrator"}},{"slug":"intent-capture","phase":"Ideation","plan_action":"Execute","conditional":false,"display":{"number":"1.1","name":"Intent Capture","lead_agent":"orchestrator"}},{"slug":"scope-definition","phase":"Ideation","plan_action":"Execute","conditional":false,"display":{"number":"1.4","name":"Scope Definition","lead_agent":"orchestrator"}}],"scan":{"project_type":"greenfield","languages":"Unknown","frameworks":"Unknown","build_system":"Unknown"},"created_at":"2026-08-23T00:00:00Z"}}"#;

/// `Started` 行の逐語 — genesis の材料 3 点 (実行 id・intent id・解決済み計画)。
///
/// 書く側 (command interface-adapter の `StartedDto`) と**同一の文字列**であることは
/// 横断適合テスト (`journal_protocol_conformance`) が固定する。
const STARTED_ROW: &str = r#"{"Started":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","intent_id":"01a02785-1bd8-76eb-aeea-5aa303ebd5b6","stages":[{"slug":"state-init","phase":"Initialization","plan_action":"Execute","conditional":false,"display":{"number":"0.1","name":"State Init","lead_agent":"orchestrator"}},{"slug":"intent-capture","phase":"Ideation","plan_action":"Execute","conditional":false,"display":{"number":"1.1","name":"Intent Capture","lead_agent":"orchestrator"}},{"slug":"scope-definition","phase":"Ideation","plan_action":"Execute","conditional":false,"display":{"number":"1.4","name":"Scope Definition","lead_agent":"orchestrator"}}]}}"#;

/// 全 12 変種を、逐語で固定した綴りと組で並べる。
fn every_variant() -> Vec<(IntentExecutionEvent, &'static str)> {
    vec![
        (
            IntentExecutionEvent::Started(Started::new(
                event_id(),
                IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
                IntentId::parse(INTENT).expect("UUIDv7"),
                stages(),
            )),
            STARTED_ROW,
        ),
        (
            IntentExecutionEvent::StageCompleted(StageCompleted::new(
                event_id(),
                execution_id(),
                slug("state-init"),
            )),
            r#"{"StageCompleted":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","stage":"state-init"}}"#,
        ),
        (
            IntentExecutionEvent::GateOpened(GateOpened::new(
                event_id(),
                execution_id(),
                slug("intent-capture"),
                vec!["a.md".to_string()],
            )),
            r#"{"GateOpened":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","stage":"intent-capture","artifacts":["a.md"]}}"#,
        ),
        (
            IntentExecutionEvent::GateApproved(GateApproved::new(
                event_id(),
                execution_id(),
                slug("intent-capture"),
                Some("ok".to_string()),
            )),
            r#"{"GateApproved":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","stage":"intent-capture","user_input":"ok"}}"#,
        ),
        (
            IntentExecutionEvent::GateRejected(GateRejected::new(
                event_id(),
                execution_id(),
                slug("intent-capture"),
                Some("why".to_string()),
            )),
            r#"{"GateRejected":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","stage":"intent-capture","feedback":"why"}}"#,
        ),
        (
            IntentExecutionEvent::StageRevised(StageRevised::new(
                event_id(),
                execution_id(),
                slug("intent-capture"),
            )),
            r#"{"StageRevised":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","stage":"intent-capture"}}"#,
        ),
        (
            IntentExecutionEvent::StageSkipped(StageSkipped::new(
                event_id(),
                execution_id(),
                slug("intent-capture"),
                "not needed".to_string(),
            )),
            r#"{"StageSkipped":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","stage":"intent-capture","reason":"not needed"}}"#,
        ),
        (
            IntentExecutionEvent::Jumped(Jumped::new(
                event_id(),
                execution_id(),
                slug("intent-capture"),
            )),
            r#"{"Jumped":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","target":"intent-capture"}}"#,
        ),
        (
            IntentExecutionEvent::Parked(Parked::new(
                event_id(),
                execution_id(),
                slug("intent-capture"),
            )),
            r#"{"Parked":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","stage":"intent-capture"}}"#,
        ),
        (
            IntentExecutionEvent::Unparked(Unparked::new(event_id(), execution_id())),
            r#"{"Unparked":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000"}}"#,
        ),
        (
            IntentExecutionEvent::Recomposed(Recomposed::new(
                event_id(),
                execution_id(),
                vec![slug("scope-definition")],
                vec![slug("intent-capture")],
            )),
            r#"{"Recomposed":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","skipped":["scope-definition"],"added":["intent-capture"]}}"#,
        ),
        (
            IntentExecutionEvent::AutonomyModeSet(AutonomyModeSet::new(
                event_id(),
                execution_id(),
                AutonomyMode::Autonomous,
            )),
            r#"{"AutonomyModeSet":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","mode":"Autonomous"}}"#,
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
fn a_row_whose_plan_breaks_its_invariants_is_refused_at_the_decode_boundary() {
    // 形は読めるが計画の不変条件を破る行は、**復号の境界で** `InvariantViolation` として
    // 止める (b40 — `Started` 面と同じ規律を intent 面にも揃えた)。通すと集約の再構成まで
    // 届いてクラッシュするが、行のバイトから分類できる破損をクラッシュに任せる理由は無い。
    // 先頭ステージ (initialization) を SKIP に畳むと Always Valid を破る。
    let tampered = INTENT_ROW.replacen(r#""plan_action":"Execute""#, r#""plan_action":"Skip""#, 1);
    let decoded: IntentEventDto = serde_json::from_str(&tampered).expect("JSON としては読める");
    assert_eq!(decoded.to_domain(), Err(DtoDecodeError::InvariantViolation));
}

#[test]
#[should_panic(expected = "recorded history violates the plan invariants")]
fn an_invariant_violation_that_slips_past_the_decode_boundary_crashes_reconstruction() {
    // 復号の境界を通り抜けた不変条件違反は回復せずクラッシュが正である (オーナー裁定
    // 2026-08-30 — 再構成は失敗を返さない)。b40 で境界の検査が増えたが、**クラッシュ規律
    // そのものは変わらない**ことをここで固定する: 検査を経ない `Intent::from` は落ちる。
    let broken = vec![StageEntry::new(
        slug("state-init"),
        PhaseId::Initialization,
        PlanAction::Skip,
        false,
        display("0.1", "State Init"),
    )];
    let _ = Intent::from((
        Created::new(
            intent_event_id(),
            IntentId::parse(INTENT).expect("UUIDv7"),
            WorkflowDefinitionId::parse("claude").expect("定義 id"),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision"),
            StartRequest::new("classic", "contract"),
            broken,
            WorkspaceScan::new(
                BrownfieldGreenfield::Greenfield,
                "Unknown",
                "Unknown",
                "Unknown",
            )
            .expect("単一行"),
        ),
        at(),
    ));
}

#[test]
fn a_malformed_identifier_is_refused_with_its_field() {
    for (from, to) in [
        (
            r#""id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0001""#,
            r#""id":"not-a-uuid""#,
        ),
        (
            r#""aggregate_id":"01a02785-1bd8-76eb-aeea-5aa303ebd5b6""#,
            r#""aggregate_id":"not-a-uuid""#,
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
fn a_started_row_with_malformed_material_is_refused() {
    // 識別子 2 種と計画の綴りを 1 つずつ壊す — genesis の材料はどれも検査付き再構成を通る。
    for (from, to) in [
        (
            r#""aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000""#,
            r#""aggregate_id":"not-a-uuid""#,
        ),
        (
            r#""intent_id":"01a02785-1bd8-76eb-aeea-5aa303ebd5b6""#,
            r#""intent_id":"not-a-uuid""#,
        ),
        (r#""slug":"state-init""#, r#""slug":"Not A Slug""#),
        (r#""phase":"Initialization""#, r#""phase":"Nowhere""#),
        (r#""plan_action":"Execute""#, r#""plan_action":"EXECUTE""#),
        (r#""number":"0.1""#, r#""number":"zero""#),
    ] {
        let tampered = STARTED_ROW.replacen(from, to, 1);
        assert_ne!(
            tampered, STARTED_ROW,
            "差し替え元が逐語形に存在する: {from}"
        );
        let decoded: IntentExecutionEventDto =
            serde_json::from_str(&tampered).expect("JSON としては読める");
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
    let tampered = r#"{"Recomposed":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","skipped":["NOT A SLUG"],"added":[]}}"#;
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
    let json = serde_json::to_string(&IntentEventDto::of(&created_event(), at()))
        .expect("DTO は直列化できる");
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
    for from in [
        r#""id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0001""#,
        r#""aggregate_id":"01a02785-1bd8-76eb-aeea-5aa303ebd5b6""#,
    ] {
        let broken = INTENT_ROW.replacen(
            from,
            &from
                .replace("-1bd8-76eb-aeea-5aa303ebd5b6", "")
                .replace("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0001", "not-a-uuid")
                .replace("01a02785", "not-a-uuid"),
            1,
        );
        let decoded: IntentEventDto = serde_json::from_str(&broken).expect("形は DTO として読める");
        assert!(
            decoded.to_domain().is_err(),
            "識別子の文法違反は拒否: {from}"
        );
    }
}

#[test]
fn a_started_row_whose_plan_breaks_its_invariants_is_refused() {
    // 計画そのものの不変条件はドメイン (`StageEntry::check_plan`) が持つ。DTO はそれを呼ぶ
    // だけで判断を複製しない。ここで止めないと、破れた計画が集約の再構成まで届いて
    // クラッシュする (再構成は失敗を返さない — オーナー裁定 2026-08-30)。
    let init = StageEntry::new(
        slug("state-init"),
        PhaseId::Initialization,
        PlanAction::Execute,
        false,
        display("0.1", "State Init"),
    );
    let skipped_head = StageEntry::new(
        slug("intent-capture"),
        PhaseId::Ideation,
        PlanAction::Skip,
        false,
        display("1.1", "Intent Capture"),
    );
    for (label, plan) in [
        ("空の計画", Vec::new()),
        ("同じ slug が 2 回", vec![init.clone(), init]),
        ("索引 0 が非 EXECUTE", vec![skipped_head]),
    ] {
        let dto = IntentExecutionEventDto::of(&IntentExecutionEvent::Started(Started::new(
            event_id(),
            IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
            IntentId::parse(INTENT).expect("UUIDv7"),
            plan,
        )));
        assert_eq!(
            dto.to_domain().expect_err("破れた計画は復号の境界で止める"),
            DtoDecodeError::InvariantViolation,
            "{label}"
        );
    }
}

#[test]
fn an_unparked_row_with_a_broken_identifier_is_refused_with_its_field() {
    // `Unparked` はドメインの材料を持たないが識別子は持つ (b40)。材料が無い変種でも
    // 復号の検査を素通ししないことを固定する。
    let row = every_variant()
        .into_iter()
        .find_map(|(event, json)| {
            matches!(event, IntentExecutionEvent::Unparked(_)).then_some(json)
        })
        .expect("Unparked の行がある");
    for (field, outlaw) in [
        ("id", "0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002"),
        ("aggregate_id", "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000"),
    ] {
        let tampered = row.replacen(
            &format!(r#""{field}":"{outlaw}""#),
            &format!(r#""{field}":"not-a-uuid""#),
            1,
        );
        assert_ne!(tampered, row, "置換対象が行に無い: {field}");
        let decoded: IntentExecutionEventDto =
            serde_json::from_str(&tampered).expect("JSON としては読める");
        let error = decoded.to_domain().expect_err("文法外の識別子は拒否");
        assert!(
            matches!(&error, DtoDecodeError::Malformed { field: got, .. } if *got == field),
            "{field}: 綴りの拒否ではない — {error:?}"
        );
    }
}
