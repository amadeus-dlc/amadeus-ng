//! ワイヤ形式のバイトを**逐語で固定**する。
//!
//! ここに書かれた JSON は改訂 9 の直前（ドメインが serde を持っていた時点）に実測した
//! 出力そのものである。DTO へ移してもバイトが 1 文字も変わっていないことが、この逐語一致で
//! 証明される。行に書かれて残る値なので、期待値を書き換えるときは移行の要否を考えること。

#![allow(
    clippy::panic,
    reason = "想定外ケースの即時失敗はテストの検証手段である (house style)"
)]

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    AutonomyMode, AutonomyModeSet, Created, GateApproved, GateOpened, GateRejected, Intent,
    IntentExecution, IntentExecutionEvent, IntentExecutionId, IntentId, Jumped, Parked, Recomposed,
    StageCompleted, StageDisplay, StageEntry, StageRevised, StageSkipped, StartRequest, Started,
    WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
    WorkflowDefinitionId,
};

use core_command_domain::orchestration::{
    IntentEvent, IntentEventId, IntentExecutionEventId, Unparked,
};

/// テストの固定イベント識別子 (実行面)。
fn execution_event_id() -> IntentExecutionEventId {
    IntentExecutionEventId::parse(EXECUTION_EVENT).expect("UUIDv7")
}

/// テストの固定イベント識別子 (intent 面)。
fn intent_event_id() -> IntentEventId {
    IntentEventId::parse(INTENT_EVENT).expect("UUIDv7")
}

use super::{
    DtoDecodeError, IntentDto, IntentEventDto, IntentExecutionDto, IntentExecutionEventDto,
};

const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";
/// intent イベント自身の識別子 (テストの固定値 — 本番は集約が採番する)。
const INTENT_EVENT: &str = "0191aaaa-bbbb-7ccc-9ddd-eeeeffff0001";
/// 実行イベント自身の識別子 (同上)。
const EXECUTION_EVENT: &str = "0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002";
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
    Intent::from((
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
        ),
        at(),
    ))
}

/// `Started` の逐語形。
///
/// genesis の材料 3 点 (実行 id・intent id・解決済み計画) を運ぶ。計画の写しを載せるのは
/// 実行の歴史が**自ストリームだけ**で再生できるための条件であり、1 要素の綴りは
/// intent 面 (`INTENT_BODY` の `stages`) と同一である。
const STARTED_BODY: &str = r#"{"Started":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","intent_id":"01a02785-1bd8-76eb-aeea-5aa303ebd5b6","stages":[{"slug":"state-init","phase":"Initialization","plan_action":"Execute","conditional":false,"display":{"number":"0.1","name":"State Init","lead_agent":"orchestrator"}},{"slug":"intent-capture","phase":"Ideation","plan_action":"Execute","conditional":false,"display":{"number":"1.1","name":"Intent Capture","lead_agent":"orchestrator"}},{"slug":"scope-definition","phase":"Ideation","plan_action":"Execute","conditional":false,"display":{"number":"1.4","name":"Scope Definition","lead_agent":"orchestrator"}}]}}"#;

/// 全 12 変種を、逐語で固定した綴りと組で並べる。
fn every_variant() -> Vec<(IntentExecutionEvent, &'static str)> {
    vec![
        (
            IntentExecutionEvent::Started(Started::new(
                execution_event_id(),
                IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
                IntentId::parse(INTENT).expect("UUIDv7"),
                stages(),
            )),
            STARTED_BODY,
        ),
        (
            IntentExecutionEvent::StageCompleted(StageCompleted::new(
                execution_event_id(),
                IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
                slug("state-init"),
            )),
            r#"{"StageCompleted":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","stage":"state-init"}}"#,
        ),
        (
            IntentExecutionEvent::GateOpened(GateOpened::new(
                execution_event_id(),
                IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
                slug("intent-capture"),
                vec!["a.md".to_string()],
            )),
            r#"{"GateOpened":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","stage":"intent-capture","artifacts":["a.md"]}}"#,
        ),
        (
            IntentExecutionEvent::GateApproved(GateApproved::new(
                execution_event_id(),
                IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
                slug("intent-capture"),
                Some("ok".to_string()),
            )),
            r#"{"GateApproved":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","stage":"intent-capture","user_input":"ok"}}"#,
        ),
        (
            IntentExecutionEvent::GateRejected(GateRejected::new(
                execution_event_id(),
                IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
                slug("intent-capture"),
                Some("why".to_string()),
            )),
            r#"{"GateRejected":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","stage":"intent-capture","feedback":"why"}}"#,
        ),
        (
            IntentExecutionEvent::StageRevised(StageRevised::new(
                execution_event_id(),
                IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
                slug("intent-capture"),
            )),
            r#"{"StageRevised":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","stage":"intent-capture"}}"#,
        ),
        (
            IntentExecutionEvent::StageSkipped(StageSkipped::new(
                execution_event_id(),
                IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
                slug("intent-capture"),
                "not needed".to_string(),
            )),
            r#"{"StageSkipped":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","stage":"intent-capture","reason":"not needed"}}"#,
        ),
        (
            IntentExecutionEvent::Jumped(Jumped::new(
                execution_event_id(),
                IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
                slug("intent-capture"),
            )),
            r#"{"Jumped":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","target":"intent-capture"}}"#,
        ),
        (
            IntentExecutionEvent::Parked(Parked::new(
                execution_event_id(),
                IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
                slug("intent-capture"),
            )),
            r#"{"Parked":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","stage":"intent-capture"}}"#,
        ),
        (
            IntentExecutionEvent::Unparked(Unparked::new(
                execution_event_id(),
                IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
            )),
            r#"{"Unparked":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000"}}"#,
        ),
        (
            IntentExecutionEvent::Recomposed(Recomposed::new(
                execution_event_id(),
                IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
                vec![slug("scope-definition")],
                vec![slug("intent-capture")],
            )),
            r#"{"Recomposed":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","skipped":["scope-definition"],"added":["intent-capture"]}}"#,
        ),
        (
            IntentExecutionEvent::AutonomyModeSet(AutonomyModeSet::new(
                execution_event_id(),
                IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
                AutonomyMode::Autonomous,
            )),
            r#"{"AutonomyModeSet":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","mode":"Autonomous"}}"#,
        ),
    ]
}

/// スナップショット行の逐語形 (genesis 直後)。
///
/// 誕生 = 初期化完了済み (issue #76) により、`checkbox` の先頭は `Completed`、`cursor` は
/// 最初のゲート付きステージ (索引 1) である。**ワイヤの形** (項目名・並び) は変わって
/// いない — 変わったのは誕生時の状態そのものである。
const GENESIS_SNAPSHOT: &str = r#"{"id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","intent_id":"01a02785-1bd8-76eb-aeea-5aa303ebd5b6","stages":[{"slug":"state-init","phase":"Initialization"},{"slug":"intent-capture","phase":"Ideation"},{"slug":"scope-definition","phase":"Ideation"}],"overlay":["Execute","Execute","Execute"],"checkbox":["Completed","InProgress","Pending"],"cursor":1,"status":"Running","parked_at":null,"autonomy":"Gated","approved":[false,false,false],"revision_count":[0,0,0],"seq_nr":1,"last_updated_at":"2026-08-23T00:00:00Z"}"#;

#[expect(
    clippy::disallowed_methods,
    reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
)]
#[test]
fn every_event_variant_serialises_to_the_recorded_bytes() {
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

#[expect(
    clippy::disallowed_methods,
    reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
)]
#[test]
fn the_snapshot_serialises_to_the_recorded_bytes_and_round_trips() {
    let (aggregate, _) = IntentExecution::start(
        IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
        &intent(),
        at(),
    );
    let json =
        serde_json::to_string(&IntentExecutionDto::of(&aggregate)).expect("DTO は直列化できる");
    assert_eq!(
        json, GENESIS_SNAPSHOT,
        "スナップショットのワイヤ形式が変わった"
    );

    let decoded: IntentExecutionDto =
        serde_json::from_str(GENESIS_SNAPSHOT).expect("記録済みの行は読める");
    assert_eq!(
        decoded,
        IntentExecutionDto::of(&aggregate),
        "行の形は DTO として往復する (差分再生の基底 — 本家 example 同型。オーナー裁定 2026-08-30)"
    );
}

#[expect(
    clippy::disallowed_methods,
    reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
)]
#[test]
fn the_payload_carries_no_transport_metadata() {
    // B7: 輸送のメタデータ (seq_nr / occurred_at / manifest) は本家の列が持つ。payload 列に
    // 混ざっていないことを綴りで固定する (旧 `schema_version` も同様に消えた)。
    //
    // **`aggregate_id` はこの一覧から外れた** (b40) — ドメインイベントはエンティティの一種
    // なので「どの集約の事実か」を自分で述べる。封筒の `aid` 列と重複して見えるが、重複こそが
    // 狙いである: 復号境界が両者を照合して、行と payload が別々の歴史を語る破損を検出する。
    let event = IntentExecutionEvent::Parked(Parked::new(
        execution_event_id(),
        IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
        slug("intent-capture"),
    ));
    let json =
        serde_json::to_string(&IntentExecutionEventDto::of(&event)).expect("DTO は直列化できる");
    for absent in ["seq_nr", "occurred_at", "schema_version", "manifest"] {
        assert!(
            !json.contains(absent),
            "{absent} が payload に残っている: {json}"
        );
    }
    for present in ["\"id\"", "\"aggregate_id\""] {
        assert!(
            json.contains(present),
            "{present} は payload に載る (イベントはエンティティ): {json}"
        );
    }
}

#[expect(
    clippy::disallowed_methods,
    reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
)]
#[test]
fn the_snapshot_payload_carries_no_optimistic_version() {
    // 版数の正本は本家 `SnapshotEnvelope::version()` (行の列) であり、payload 列は純粋な
    // ドメイン内容だけを持つ (ADR-010 / B7)。
    let (aggregate, _) = IntentExecution::start(
        IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
        &intent(),
        at(),
    );
    let json =
        serde_json::to_string(&IntentExecutionDto::of(&aggregate)).expect("DTO は直列化できる");
    assert!(
        !json.contains("version"),
        "楽観 version は payload に載らない: {json}"
    );
}

#[expect(
    clippy::disallowed_methods,
    reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
)]
#[test]
fn the_snapshot_carries_only_the_stage_keys_as_static_material() {
    // 改訂 3 の受入基準の後継 — 集約が自己完結 replay のために持つ添字帳 (slug + phase の
    // 2 項目だけ) は写しに載る (issue #44 — オーナー裁定 2026-08-30「replay や apply_event が
    // 集約側に必要」)。それ以外の intent 由来の静的材料 (定義・表示属性・計画・条件フラグ) は
    // 従来どおり載らない。綴りは行に書かれて残る値なので、属性名を逐語で固定する。
    let (aggregate, _) = IntentExecution::start(
        IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
        &intent(),
        at(),
    );
    let json =
        serde_json::to_string(&IntentExecutionDto::of(&aggregate)).expect("DTO は直列化できる");
    assert!(
        json.contains(r#""stages":[{"slug":"state-init","phase":"Initialization"}"#),
        "添字帳 (slug + phase) は写しに載る: {json}"
    );
    for absent in [
        "definition_id",
        "definition_revision",
        "plan_action",
        "conditional",
        "display",
    ] {
        assert!(!json.contains(absent), "{absent} は写しに載らない: {json}");
    }
    for present in [
        "id",
        "intent_id",
        "overlay",
        "checkbox",
        "cursor",
        "status",
        "parked_at",
        "autonomy",
        "approved",
        "revision_count",
        "seq_nr",
        "last_updated_at",
    ] {
        assert!(json.contains(present), "{present} は写しに載る: {json}");
    }
}

#[test]
fn a_malformed_identifier_is_refused_with_its_field() {
    // 誕生の材料 (intent ジャーナル面) の識別子・閉集合の検査。`Started` 面の検査は
    // `a_started_row_with_a_malformed_intent_id_is_refused` が持つ (issue #56 で面が分かれた)。
    let row = format!(r#"{{"Created":{CREATED_BODY}}}"#);
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
        let tampered = row.replacen(from, to, 1);
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
        (r#""plan_action":"Execute""#, r#""plan_action":"MAYBE""#),
        (r#""number":"0.1""#, r#""number":"not-a-number""#),
    ] {
        let tampered = STARTED_BODY.replacen(from, to, 1);
        assert_ne!(
            tampered, STARTED_BODY,
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
    let row = every_variant()
        .into_iter()
        .map(|(_, json)| json)
        .find(|json| json.contains(r#""mode":"Autonomous""#))
        .expect("その綴りを含む変種がある");
    let tampered = row.replace(r#""mode":"Autonomous""#, r#""mode":"autonomous""#);
    let decoded: IntentExecutionEventDto =
        serde_json::from_str(&tampered).expect("JSON としては読める");
    assert!(decoded.to_domain().is_err(), "拒むべき値: autonomous");
}

#[test]
fn a_row_whose_stage_display_is_not_single_line_is_refused() {
    // 表示属性は状態ファイルの bullet 行に書かれる値なので、改行が混ざる行は復号で止める
    // (intent ジャーナル面 — 表示属性の正本は誕生の材料である。issue #56)。
    let row = format!(r#"{{"Created":{CREATED_BODY}}}"#);
    let tampered = row.replacen(r#""name":"State Init""#, r#""name":"State\nInit""#, 1);
    let decoded: IntentEventDto = serde_json::from_str(&tampered).expect("JSON としては読める");
    assert!(decoded.to_domain().is_err(), "改行入りの表示属性は拒む");
}

#[test]
fn a_row_whose_scan_field_is_not_single_line_is_refused() {
    let row = format!(r#"{{"Created":{CREATED_BODY}}}"#);
    let tampered = row.replacen(r#""languages":"Unknown""#, r#""languages":"a\nb""#, 1);
    let decoded: IntentEventDto = serde_json::from_str(&tampered).expect("JSON としては読める");
    assert!(decoded.to_domain().is_err(), "改行入りの走査結果は拒む");
}

#[test]
fn an_optional_request_field_round_trips_when_present() {
    // `depth` / `test_strategy` は省略可能で、誕生の材料としてだけ運ばれる (集約状態には
    // ならない)。両方が載った行も読めることを固定する。
    let row = format!(r#"{{"Created":{CREATED_BODY}}}"#);
    let filled = row.replacen(
        r#""test_strategy":null"#,
        r#""test_strategy":"balanced""#,
        1,
    );
    let decoded: IntentEventDto = serde_json::from_str(&filled).expect("記録済みの行は読める");
    let IntentEvent::Created(created) = decoded.to_domain().expect("ドメインへ戻せる");
    let intent = Intent::from((created, at()));
    assert_eq!(intent.test_strategy(), Some("balanced"));
    assert_eq!(intent.depth(), Some("standard"));
}

#[test]
fn a_snapshot_row_with_a_broken_spelling_is_refused_field_by_field() {
    // to_domain の失敗面 — 識別子の文法違反と閉集合外の綴りは、どの列でも復号を止める
    // (BR1.5。基底の復元は完全コンストラクタを必ず通り、検査を迂回する読取口は無い)。
    for (from, to) in [
        // 識別子の文法違反。
        (
            r#""id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000""#,
            r#""id":"broken""#,
        ),
        (
            r#""intent_id":"01a02785-1bd8-76eb-aeea-5aa303ebd5b6""#,
            r#""intent_id":"broken""#,
        ),
        // 添字帳の slug / phase。
        (r#""slug":"state-init""#, r#""slug":"NOT A SLUG""#),
        (r#""phase":"Initialization""#, r#""phase":"initialization""#),
        // 実行時ベクトルと列挙の綴り。
        (r#""overlay":["Execute""#, r#""overlay":["execute""#),
        (r#""checkbox":["Completed""#, r#""checkbox":["completed""#),
        (r#""status":"Running""#, r#""status":"running""#),
        (r#""autonomy":"Gated""#, r#""autonomy":"gated""#),
        // 集約不変条件 (範囲外カーソル) — 完全コンストラクタが拒む。
        (r#""cursor":1"#, r#""cursor":99"#),
    ] {
        let mutated = GENESIS_SNAPSHOT.replace(from, to);
        assert_ne!(mutated, GENESIS_SNAPSHOT, "置換対象が行に無い: {from}");
        let decoded: IntentExecutionDto =
            serde_json::from_str(&mutated).expect("DTO としては読める");
        assert!(
            decoded.to_domain().is_err(),
            "{from} -> {to} は復号を止める"
        );
    }
    // 対照: 無改竄の行はドメインへ戻る。
    let intact: IntentExecutionDto =
        serde_json::from_str(GENESIS_SNAPSHOT).expect("記録済みの行は読める");
    assert!(intact.to_domain().is_ok());
}

#[test]
fn a_malformed_stage_reference_in_a_list_variant_is_refused() {
    // 列の中の 1 本でも文法外の slug は復号を止める (slugs_of の失敗面)。
    let tampered = r#"{"Recomposed":{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","skipped":["NOT A SLUG"],"added":[]}}"#;
    let decoded: IntentExecutionEventDto =
        serde_json::from_str(tampered).expect("JSON としては読める");
    assert!(decoded.to_domain().is_err());
}

// ---------------------------------------------------------------------------
// intent 自身のジャーナル面 (issue #50)
// ---------------------------------------------------------------------------

/// intent スナップショット行のバイト形 (集約の全状態 — `id` は**集約の**識別子)。
const INTENT_SNAPSHOT: &str = r#"{"id":"01a02785-1bd8-76eb-aeea-5aa303ebd5b6","definition_id":"claude","definition_revision":"sha256:0000000000000000000000000000000000000000000000000000000000000000","start_request":{"scope":"classic","request":"contract","depth":"standard","test_strategy":null,"review":"adversarial"},"stages":[{"slug":"state-init","phase":"Initialization","plan_action":"Execute","conditional":false,"display":{"number":"0.1","name":"State Init","lead_agent":"orchestrator"}},{"slug":"intent-capture","phase":"Ideation","plan_action":"Execute","conditional":false,"display":{"number":"1.1","name":"Intent Capture","lead_agent":"orchestrator"}},{"slug":"scope-definition","phase":"Ideation","plan_action":"Execute","conditional":false,"display":{"number":"1.4","name":"Scope Definition","lead_agent":"orchestrator"}}],"scan":{"project_type":"greenfield","languages":"Unknown","frameworks":"Unknown","build_system":"Unknown"},"created_at":"2026-08-23T00:00:00Z"}"#;

/// `Created` ペイロードのバイト形 (ジャーナル面 — `id` は**イベント自身の**識別子で、
/// 集約の識別子は `aggregate_id` が運ぶ。内容部分はスナップショット面と同じ綴りである)。
const CREATED_BODY: &str = r#"{"id":"0191aaaa-bbbb-7ccc-9ddd-eeeeffff0001","aggregate_id":"01a02785-1bd8-76eb-aeea-5aa303ebd5b6","definition_id":"claude","definition_revision":"sha256:0000000000000000000000000000000000000000000000000000000000000000","start_request":{"scope":"classic","request":"contract","depth":"standard","test_strategy":null,"review":"adversarial"},"stages":[{"slug":"state-init","phase":"Initialization","plan_action":"Execute","conditional":false,"display":{"number":"0.1","name":"State Init","lead_agent":"orchestrator"}},{"slug":"intent-capture","phase":"Ideation","plan_action":"Execute","conditional":false,"display":{"number":"1.1","name":"Intent Capture","lead_agent":"orchestrator"}},{"slug":"scope-definition","phase":"Ideation","plan_action":"Execute","conditional":false,"display":{"number":"1.4","name":"Scope Definition","lead_agent":"orchestrator"}}],"scan":{"project_type":"greenfield","languages":"Unknown","frameworks":"Unknown","build_system":"Unknown"},"created_at":"2026-08-23T00:00:00Z"}"#;

/// intent の誕生イベント (ジャーナル面の材料 — `intent()` と同じ材料から組む)。
fn created_event() -> IntentEvent {
    IntentEvent::Created(Created::new(
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
    ))
}

#[expect(
    clippy::disallowed_methods,
    reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
)]
#[test]
fn the_intent_journal_row_serialises_to_the_recorded_bytes_and_round_trips() {
    let event = created_event();
    let expected = format!(r#"{{"Created":{CREATED_BODY}}}"#);
    let json =
        serde_json::to_string(&IntentEventDto::of(&event, at())).expect("DTO は直列化できる");
    assert_eq!(json, expected, "intent ジャーナルのワイヤ形式が変わった");

    let decoded: IntentEventDto = serde_json::from_str(&expected).expect("記録済みの行は読める");
    assert_eq!(decoded.to_domain().expect("ドメインへ戻せる"), event);
}

#[expect(
    clippy::disallowed_methods,
    reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
)]
#[test]
fn the_intent_faces_share_the_same_content_bytes() {
    // (a) `Created` の中身と (b) intent 集約のスナップショット行は同じ材料を同じ綴りで運ぶ
    // (issue #50 — 部品 DTO を共有する根拠)。b40 で先頭の識別子だけが分かれた: ジャーナル面は
    // 「イベントの id + どの集約の事実か」、スナップショット面は「集約の id」である。
    let snapshot = serde_json::to_string(&IntentDto::of(&intent())).expect("DTO は直列化できる");
    assert_eq!(
        snapshot, INTENT_SNAPSHOT,
        "スナップショット面のバイトが変わった"
    );
    // 内容部分 (`definition_id` 以降) は 2 面で 1 文字も違わない — 先頭の識別子だけが
    // 「集約の id」と「イベントの id + aggregate_id」に分かれている (b40)。
    let tail = |row: &str| row[row.find(r#""definition_id""#).expect("内容の始まり")..].to_string();
    assert_eq!(tail(INTENT_SNAPSHOT), tail(CREATED_BODY));
}

#[test]
fn a_malformed_identifier_in_the_intent_journal_is_refused_with_its_field() {
    // イベント自身の識別子と集約の識別子は別のフィールドなので、材料も別々に名乗る (b40)。
    let row = format!(r#"{{"Created":{CREATED_BODY}}}"#);
    for (from, field) in [(INTENT_EVENT, "id"), (INTENT, "aggregate_id")] {
        let broken = row.replace(from, "not-a-uuid");
        let decoded: IntentEventDto = serde_json::from_str(&broken).expect("形は DTO として読める");
        let err = decoded.to_domain().expect_err("識別子の文法違反は拒否");
        assert_eq!(
            err.to_string(),
            format!("malformed field {field}: not-a-uuid")
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
            execution_event_id(),
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
fn an_intent_snapshot_row_with_a_broken_spelling_is_refused_field_by_field() {
    // スナップショット面の復号も検査付き再構成を通る (b40 でジャーナル面と別の型に
    // 分かれたので、こちらの検査も面ごとに固定する)。`id` は**集約の**識別子である。
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
        (r#""slug":"state-init""#, r#""slug":"Not A Slug""#),
        (
            r#""project_type":"greenfield""#,
            r#""project_type":"Greenfield""#,
        ),
    ] {
        let tampered = INTENT_SNAPSHOT.replacen(from, to, 1);
        assert_ne!(tampered, INTENT_SNAPSHOT, "置換対象が行に無い: {from}");
        let decoded: IntentDto = serde_json::from_str(&tampered).expect("DTO としては読める");
        assert!(
            decoded.to_domain().is_err(),
            "{from} -> {to} は復号を止める"
        );
    }
    // 健全な行はドメインへ戻る (失敗面だけでなく成功面も通す)。
    let decoded: IntentDto = serde_json::from_str(INTENT_SNAPSHOT).expect("記録済みの行は読める");
    assert_eq!(decoded.to_domain().expect("ドメインへ戻せる"), intent());
}

#[test]
fn an_intent_journal_row_whose_plan_breaks_its_invariants_is_refused() {
    // 計画そのものの不変条件はドメイン (`StageEntry::check_plan`) が持つ。ジャーナル面の
    // 復号がそれを呼ぶことで、破れた計画は集約の再構成まで届かない (b40 — `Started` 面と
    // 同じ規律を intent 面にも揃えた)。
    let row = format!(r#"{{"Created":{CREATED_BODY}}}"#);
    // 先頭ステージ (initialization) を SKIP に畳むと Always Valid を破る。
    let tampered = row.replacen(r#""plan_action":"Execute""#, r#""plan_action":"Skip""#, 1);
    let decoded: IntentEventDto = serde_json::from_str(&tampered).expect("JSON としては読める");
    assert_eq!(decoded.to_domain(), Err(DtoDecodeError::InvariantViolation));
}
