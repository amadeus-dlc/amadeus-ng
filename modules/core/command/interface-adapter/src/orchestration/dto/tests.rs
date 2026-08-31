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

use core_command_domain::orchestration::IntentEvent;

use super::{IntentDto, IntentEventDto, IntentExecutionDto, IntentExecutionEventDto};

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

/// 全 12 変種を、逐語で固定した綴りと組で並べる。
fn every_variant() -> Vec<(IntentExecutionEvent, &'static str)> {
    vec![
        (
            IntentExecutionEvent::Started(Started::new(IntentId::parse(INTENT).expect("UUIDv7"))),
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

/// スナップショット行の逐語形 (genesis 直後)。
const GENESIS_SNAPSHOT: &str = r#"{"id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","intent_id":"01a02785-1bd8-76eb-aeea-5aa303ebd5b6","stages":[{"slug":"state-init","phase":"Initialization"},{"slug":"intent-capture","phase":"Ideation"},{"slug":"scope-definition","phase":"Ideation"}],"overlay":["Execute","Execute","Execute"],"checkbox":["InProgress","Pending","Pending"],"cursor":0,"status":"Running","parked_at":null,"autonomy":"Gated","approved":[false,false,false],"revision_count":[0,0,0],"seq_nr":1,"last_updated_at":"2026-08-23T00:00:00Z"}"#;

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
    // B7: 封筒の 4 点 (aggregate_id / seq_nr / occurred_at / manifest) は本家の列が持つ。
    // payload 列に混ざっていないことを綴りで固定する (旧 `schema_version` も同様に消えた)。
    // 改訂 9 でドメインから serde が消えたため、この検査の置き場もここへ移った。
    let event = IntentExecutionEvent::Parked(Parked::new(slug("intent-capture")));
    let json =
        serde_json::to_string(&IntentExecutionEventDto::of(&event)).expect("DTO は直列化できる");
    for absent in [
        "seq_nr",
        "occurred_at",
        "schema_version",
        "aggregate_id",
        "manifest",
    ] {
        assert!(
            !json.contains(absent),
            "{absent} が payload に残っている: {json}"
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
    let row = format!(r#"{{"Created":{INTENT_BODY}}}"#);
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
        let tampered = row.replacen(from, to, 1);
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
    let row = format!(r#"{{"Created":{INTENT_BODY}}}"#);
    let tampered = row.replacen(r#""name":"State Init""#, r#""name":"State\nInit""#, 1);
    let decoded: IntentEventDto = serde_json::from_str(&tampered).expect("JSON としては読める");
    assert!(decoded.to_domain().is_err(), "改行入りの表示属性は拒む");
}

#[test]
fn a_row_whose_scan_field_is_not_single_line_is_refused() {
    let row = format!(r#"{{"Created":{INTENT_BODY}}}"#);
    let tampered = row.replacen(r#""languages":"Unknown""#, r#""languages":"a\nb""#, 1);
    let decoded: IntentEventDto = serde_json::from_str(&tampered).expect("JSON としては読める");
    assert!(decoded.to_domain().is_err(), "改行入りの走査結果は拒む");
}

#[test]
fn an_optional_request_field_round_trips_when_present() {
    // `depth` / `test_strategy` は省略可能で、誕生の材料としてだけ運ばれる (集約状態には
    // ならない)。両方が載った行も読めることを固定する。
    let row = format!(r#"{{"Created":{INTENT_BODY}}}"#);
    let filled = row.replacen(
        r#""test_strategy":null"#,
        r#""test_strategy":"balanced""#,
        1,
    );
    let decoded: IntentEventDto = serde_json::from_str(&filled).expect("記録済みの行は読める");
    let IntentEvent::Created(created) = decoded.to_domain().expect("ドメインへ戻せる");
    let intent = Intent::from(created);
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
        (
            r#""checkbox":["InProgress""#,
            r#""checkbox":["in-progress""#,
        ),
        (r#""status":"Running""#, r#""status":"running""#),
        (r#""autonomy":"Gated""#, r#""autonomy":"gated""#),
        // 集約不変条件 (範囲外カーソル) — 完全コンストラクタが拒む。
        (r#""cursor":0"#, r#""cursor":99"#),
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
    let tampered = r#"{"Recomposed":{"skipped":["NOT A SLUG"],"added":[]}}"#;
    let decoded: IntentExecutionEventDto =
        serde_json::from_str(tampered).expect("JSON としては読める");
    assert!(decoded.to_domain().is_err());
}

// ---------------------------------------------------------------------------
// intent 自身のジャーナル面 (issue #50)
// ---------------------------------------------------------------------------

/// intent の材料のバイト形 (2 面共通 — `Created` の中身・intent 集約のスナップショット行)。
const INTENT_BODY: &str = r#"{"id":"01a02785-1bd8-76eb-aeea-5aa303ebd5b6","definition_id":"claude","definition_revision":"sha256:0000000000000000000000000000000000000000000000000000000000000000","start_request":{"scope":"classic","request":"contract","depth":"standard","test_strategy":null,"review":null},"stages":[{"slug":"state-init","phase":"Initialization","plan_action":"Execute","conditional":false,"display":{"number":"0.1","name":"State Init","lead_agent":"orchestrator"}},{"slug":"intent-capture","phase":"Ideation","plan_action":"Execute","conditional":false,"display":{"number":"1.1","name":"Intent Capture","lead_agent":"orchestrator"}},{"slug":"scope-definition","phase":"Ideation","plan_action":"Execute","conditional":false,"display":{"number":"1.4","name":"Scope Definition","lead_agent":"orchestrator"}}],"scan":{"project_type":"greenfield","languages":"Unknown","frameworks":"Unknown","build_system":"Unknown"}}"#;

/// intent の誕生イベント (ジャーナル面の材料 — `intent()` と同じ材料から組む)。
fn created_event() -> IntentEvent {
    IntentEvent::Created(Created::new(
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

#[expect(
    clippy::disallowed_methods,
    reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
)]
#[test]
fn the_intent_journal_row_serialises_to_the_recorded_bytes_and_round_trips() {
    let event = created_event();
    let expected = format!(r#"{{"Created":{INTENT_BODY}}}"#);
    let json = serde_json::to_string(&IntentEventDto::of(&event)).expect("DTO は直列化できる");
    assert_eq!(json, expected, "intent ジャーナルのワイヤ形式が変わった");

    let decoded: IntentEventDto = serde_json::from_str(&expected).expect("記録済みの行は読める");
    assert_eq!(decoded.to_domain().expect("ドメインへ戻せる"), event);
}

#[expect(
    clippy::disallowed_methods,
    reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
)]
#[test]
fn the_intent_faces_share_the_same_bytes() {
    // (a) `Created` の中身と (b) intent 集約のスナップショット行は同じ材料 = 同じバイトで
    // ある (issue #50 — `IntentDto` 1 本に束ねた根拠)。かつての第 3 面 (`Started` の埋め
    // 込み) は issue #56 で消えた — `Started` は intent の識別子だけを運ぶ。
    let snapshot = serde_json::to_string(&IntentDto::of(&intent())).expect("DTO は直列化できる");
    assert_eq!(
        snapshot, INTENT_BODY,
        "スナップショット面のバイトが変わった"
    );
}

#[test]
fn a_malformed_identifier_in_the_intent_journal_is_refused_with_its_field() {
    let broken = format!(r#"{{"Created":{INTENT_BODY}}}"#).replace(INTENT, "not-a-uuid");
    let decoded: IntentEventDto = serde_json::from_str(&broken).expect("形は DTO として読める");
    let err = decoded.to_domain().expect_err("識別子の文法違反は拒否");
    assert_eq!(err.to_string(), "malformed field id: not-a-uuid");
}
