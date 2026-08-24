//! `EventPayloadWire` — `WorkflowExecutionEventPayload` の JSON 表現 (functional-spec §4.1)。

use core_domain::orchestration::{
    AutonomyModeSet, GateApproved, GateOpened, GateRejected, Jumped, Parked, PhaseBoundary,
    Recomposed, StageCompleted, StageRevised, StageSkipped, StartRequest, Started,
    WorkflowExecutionEventPayload,
};
use core_domain::workflow_definition::{PhaseId, StageSlug};
use core_use_case::orchestration::{CorruptCause, EventStoreError};
use serde::Serialize;

use super::{
    SCHEMA_VERSION, StageEntryWire, WireObject, corrupt_error, direction_token,
    parse_definition_id, parse_definition_revision, parse_direction, parse_entry, parse_json,
    parse_phase, parse_slug, to_canonical_json,
};

/// フェーズ境界のワイヤ表現。
///
/// functional-spec §4.1 の表は `phase_boundary: string | null` と書くが、ドメインの
/// `PhaseBoundary` は**2 つの `PhaseId` の組**である。1 本の文字列に畳むには区切り記号を
/// 発明することになり、往復の忠実さを区切り規約に賭けることになるため、入れ子の
/// オブジェクトで両半分をそれぞれ `PhaseId::parse` に通す形にした (設計質問として報告済み)。
#[derive(Debug, Serialize)]
pub(crate) struct PhaseBoundaryWire {
    from_phase: &'static str,
    to_phase: &'static str,
}

/// `PhaseBoundaryWire` が持つキーの閉集合。
const PHASE_BOUNDARY_KEYS: [&str; 2] = ["from_phase", "to_phase"];

/// `WorkflowExecutionEventPayload` のワイヤ表現 (functional-spec §4.1)。
///
/// JSON は `{"type": "<変種名>", …材料}` — `type` タグは serde の internally tagged
/// 表現で先頭に出る。封筒 (`intent_id` / `seq_nr` / `schema_version` / `occurred_at`) は
/// 列に出すのでここには含めない。
///
/// 固定トークンは upstream 綴り (`PlanAction` の `EXECUTE` / `SKIP`、`PhaseId` の 5 語、
/// `AutonomyMode` の `autonomous` / `gated`)、それ以外の列挙は snake_case。
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub(crate) enum EventPayloadWire {
    /// 実行の開始 (解決済み計画を自己完結で持つ)。
    Started {
        definition_id: String,
        definition_revision: String,
        scope: String,
        request: String,
        depth: Option<String>,
        test_strategy: Option<String>,
        stages: Vec<StageEntryWire>,
    },
    /// 非ゲートステージの完了。
    StageCompleted {
        stage: String,
        next_stage: Option<String>,
    },
    /// 承認ゲートの開放。
    GateOpened {
        stage: String,
        artifacts: Vec<String>,
    },
    /// 承認ゲートの通過。
    GateApproved {
        stage: String,
        user_input: Option<String>,
        next_stage: Option<String>,
        phase_boundary: Option<PhaseBoundaryWire>,
    },
    /// 承認ゲートでの差し戻し。
    GateRejected {
        stage: String,
        feedback: Option<String>,
        revision_count: u32,
    },
    /// 差し戻し後のゲート再入。
    StageRevised { stage: String },
    /// ステージの読み飛ばし。
    StageSkipped {
        stage: String,
        reason: String,
        next_stage: Option<String>,
    },
    /// カーソルの移動。
    Jumped {
        direction: &'static str,
        source: String,
        target: String,
        stages_reset: Vec<String>,
        stages_skipped: Vec<String>,
    },
    /// park マーカーの設置。
    Parked { stage: String },
    /// park マーカーの除去 (材料なし)。
    Unparked,
    /// 実効プランの再形成。
    Recomposed {
        skipped: Vec<String>,
        added: Vec<String>,
        stages_in_scope: Vec<String>,
    },
    /// 自律モードの設定。
    AutonomyModeSet { mode: &'static str },
}

/// 12 変種の `type` タグ (閉集合)。復号はこの表の外を `UnknownEventType` で拒否する。
const EVENT_TYPES: [&str; 12] = [
    "Started",
    "StageCompleted",
    "GateOpened",
    "GateApproved",
    "GateRejected",
    "StageRevised",
    "StageSkipped",
    "Jumped",
    "Parked",
    "Unparked",
    "Recomposed",
    "AutonomyModeSet",
];

/// slug 列を文字列列へ。
fn slug_texts(slugs: &[StageSlug]) -> Vec<String> {
    slugs.iter().map(|s| s.as_str().to_string()).collect()
}

/// 省略可能な slug を文字列へ。
fn optional_slug_text(slug: Option<&StageSlug>) -> Option<String> {
    slug.map(|s| s.as_str().to_string())
}

impl EventPayloadWire {
    /// ワイヤの版 (C5 `schema_version: 1`)。
    pub(crate) const SCHEMA_VERSION: u32 = SCHEMA_VERSION;

    /// ジャーナル行の `event_type` 列に書く変種名。
    pub(crate) const fn event_type(payload: &WorkflowExecutionEventPayload) -> &'static str {
        match payload {
            WorkflowExecutionEventPayload::Started(_) => "Started",
            WorkflowExecutionEventPayload::StageCompleted(_) => "StageCompleted",
            WorkflowExecutionEventPayload::GateOpened(_) => "GateOpened",
            WorkflowExecutionEventPayload::GateApproved(_) => "GateApproved",
            WorkflowExecutionEventPayload::GateRejected(_) => "GateRejected",
            WorkflowExecutionEventPayload::StageRevised(_) => "StageRevised",
            WorkflowExecutionEventPayload::StageSkipped(_) => "StageSkipped",
            WorkflowExecutionEventPayload::Jumped(_) => "Jumped",
            WorkflowExecutionEventPayload::Parked(_) => "Parked",
            WorkflowExecutionEventPayload::Unparked => "Unparked",
            WorkflowExecutionEventPayload::Recomposed(_) => "Recomposed",
            WorkflowExecutionEventPayload::AutonomyModeSet(_) => "AutonomyModeSet",
        }
    }

    /// ペイロードを正準 JSON へ符号化する (`journal.payload` 列の値)。
    ///
    /// # Errors
    ///
    /// 正準 JSON へ写せない場合に `Corrupt(UndecodablePayload)` を返す。
    pub(crate) fn encode(
        aggregate_id: &str,
        seq_nr: u64,
        payload: &WorkflowExecutionEventPayload,
    ) -> Result<String, EventStoreError> {
        to_canonical_json(&EventPayloadWire::from_payload(payload))
            .map_err(|cause| corrupt_error(aggregate_id, Some(seq_nr), cause))
    }

    /// ジャーナル行の材料からペイロードを復元する (検査点 1 / 2 — security-design §2)。
    ///
    /// `event_type` は列の値、`payload` は正準 JSON。列の `type` と JSON の `type` が
    /// 食い違う行は受け付けない。
    ///
    /// # Errors
    ///
    /// 版不一致 (`Corrupt(SchemaVersion)`)、`type` が 12 語の閉集合の外
    /// (`Corrupt(UnknownEventType)`)、構文・型・未知フィールド・値の形式違反
    /// (`Corrupt(UndecodablePayload)`) を返す。
    pub(crate) fn decode(
        aggregate_id: &str,
        seq_nr: u64,
        schema_version: u32,
        event_type: &str,
        payload: &str,
    ) -> Result<WorkflowExecutionEventPayload, EventStoreError> {
        EventPayloadWire::decode_inner(schema_version, event_type, payload)
            .map_err(|cause| corrupt_error(aggregate_id, Some(seq_nr), cause))
    }

    /// ドメインのペイロードをワイヤの形へ写す。
    fn from_payload(payload: &WorkflowExecutionEventPayload) -> EventPayloadWire {
        match payload {
            WorkflowExecutionEventPayload::Started(started) => EventPayloadWire::Started {
                definition_id: started.definition_id().as_str().to_string(),
                definition_revision: started.definition_revision().as_str().to_string(),
                scope: started.scope().to_string(),
                request: started.request().to_string(),
                depth: started.depth().map(str::to_string),
                test_strategy: started.test_strategy().map(str::to_string),
                stages: started
                    .stages()
                    .iter()
                    .map(StageEntryWire::from_entry)
                    .collect(),
            },
            WorkflowExecutionEventPayload::StageCompleted(completed) => {
                EventPayloadWire::StageCompleted {
                    stage: completed.stage().as_str().to_string(),
                    next_stage: optional_slug_text(completed.next_stage()),
                }
            }
            WorkflowExecutionEventPayload::GateOpened(opened) => EventPayloadWire::GateOpened {
                stage: opened.stage().as_str().to_string(),
                artifacts: opened.artifacts().to_vec(),
            },
            WorkflowExecutionEventPayload::GateApproved(approved) => {
                EventPayloadWire::GateApproved {
                    stage: approved.stage().as_str().to_string(),
                    user_input: approved.user_input().map(str::to_string),
                    next_stage: optional_slug_text(approved.next_stage()),
                    phase_boundary: approved.phase_boundary().map(|boundary| PhaseBoundaryWire {
                        from_phase: boundary.from_phase().as_str(),
                        to_phase: boundary.to_phase().as_str(),
                    }),
                }
            }
            WorkflowExecutionEventPayload::GateRejected(rejected) => {
                EventPayloadWire::GateRejected {
                    stage: rejected.stage().as_str().to_string(),
                    feedback: rejected.feedback().map(str::to_string),
                    revision_count: rejected.revision_count(),
                }
            }
            WorkflowExecutionEventPayload::StageRevised(revised) => {
                EventPayloadWire::StageRevised {
                    stage: revised.stage().as_str().to_string(),
                }
            }
            WorkflowExecutionEventPayload::StageSkipped(skipped) => {
                EventPayloadWire::StageSkipped {
                    stage: skipped.stage().as_str().to_string(),
                    reason: skipped.reason().to_string(),
                    next_stage: optional_slug_text(skipped.next_stage()),
                }
            }
            WorkflowExecutionEventPayload::Jumped(jumped) => EventPayloadWire::Jumped {
                direction: direction_token(jumped.direction()),
                source: jumped.source().as_str().to_string(),
                target: jumped.target().as_str().to_string(),
                stages_reset: slug_texts(jumped.stages_reset()),
                stages_skipped: slug_texts(jumped.stages_skipped()),
            },
            WorkflowExecutionEventPayload::Parked(parked) => EventPayloadWire::Parked {
                stage: parked.stage().as_str().to_string(),
            },
            WorkflowExecutionEventPayload::Unparked => EventPayloadWire::Unparked,
            WorkflowExecutionEventPayload::Recomposed(recomposed) => EventPayloadWire::Recomposed {
                skipped: slug_texts(recomposed.skipped()),
                added: slug_texts(recomposed.added()),
                stages_in_scope: slug_texts(recomposed.stages_in_scope()),
            },
            WorkflowExecutionEventPayload::AutonomyModeSet(set) => {
                EventPayloadWire::AutonomyModeSet {
                    mode: super::autonomy_token(set.mode()),
                }
            }
        }
    }

    /// 復号の本体 (材料の付与は呼出側)。
    fn decode_inner(
        schema_version: u32,
        event_type: &str,
        payload: &str,
    ) -> Result<WorkflowExecutionEventPayload, CorruptCause> {
        if schema_version != SCHEMA_VERSION {
            return Err(CorruptCause::SchemaVersion);
        }
        if !EVENT_TYPES.contains(&event_type) {
            return Err(CorruptCause::UnknownEventType);
        }
        let json = parse_json(payload)?;
        let object = WireObject::new(&json)?;
        let tag = object.string("type")?;
        if !EVENT_TYPES.contains(&tag) {
            return Err(CorruptCause::UnknownEventType);
        }
        if tag != event_type {
            // 列とペイロードで変種名が食い違う行は、どちらが正かを決められない。
            return Err(CorruptCause::UndecodablePayload);
        }
        match tag {
            "Started" => {
                object.only(&[
                    "type",
                    "definition_id",
                    "definition_revision",
                    "scope",
                    "request",
                    "depth",
                    "test_strategy",
                    "stages",
                ])?;
                let mut request =
                    StartRequest::new(object.string("scope")?, object.string("request")?);
                if let Some(depth) = object.optional_string("depth")? {
                    request = request.with_depth(depth);
                }
                if let Some(strategy) = object.optional_string("test_strategy")? {
                    request = request.with_test_strategy(strategy);
                }
                let stages = object
                    .array("stages")?
                    .iter()
                    .map(parse_entry)
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(WorkflowExecutionEventPayload::Started(Started::new(
                    parse_definition_id(object.string("definition_id")?)?,
                    parse_definition_revision(object.string("definition_revision")?)?,
                    &request,
                    stages,
                )))
            }
            "StageCompleted" => {
                object.only(&["type", "stage", "next_stage"])?;
                Ok(WorkflowExecutionEventPayload::StageCompleted(
                    StageCompleted::new(
                        parse_slug(object.string("stage")?)?,
                        optional_slug(&object, "next_stage")?,
                    ),
                ))
            }
            "GateOpened" => {
                object.only(&["type", "stage", "artifacts"])?;
                Ok(WorkflowExecutionEventPayload::GateOpened(GateOpened::new(
                    parse_slug(object.string("stage")?)?,
                    object.texts("artifacts")?,
                )))
            }
            "GateApproved" => {
                object.only(&[
                    "type",
                    "stage",
                    "user_input",
                    "next_stage",
                    "phase_boundary",
                ])?;
                Ok(WorkflowExecutionEventPayload::GateApproved(
                    GateApproved::new(
                        parse_slug(object.string("stage")?)?,
                        object.optional_string("user_input")?.map(str::to_string),
                        optional_slug(&object, "next_stage")?,
                        decode_phase_boundary(&object)?,
                    ),
                ))
            }
            "GateRejected" => {
                object.only(&["type", "stage", "feedback", "revision_count"])?;
                Ok(WorkflowExecutionEventPayload::GateRejected(
                    GateRejected::new(
                        parse_slug(object.string("stage")?)?,
                        object.optional_string("feedback")?.map(str::to_string),
                        object.u32("revision_count")?,
                    ),
                ))
            }
            "StageRevised" => {
                object.only(&["type", "stage"])?;
                Ok(WorkflowExecutionEventPayload::StageRevised(
                    StageRevised::new(parse_slug(object.string("stage")?)?),
                ))
            }
            "StageSkipped" => {
                object.only(&["type", "stage", "reason", "next_stage"])?;
                Ok(WorkflowExecutionEventPayload::StageSkipped(
                    StageSkipped::new(
                        parse_slug(object.string("stage")?)?,
                        object.string("reason")?.to_string(),
                        optional_slug(&object, "next_stage")?,
                    ),
                ))
            }
            "Jumped" => {
                object.only(&[
                    "type",
                    "direction",
                    "source",
                    "target",
                    "stages_reset",
                    "stages_skipped",
                ])?;
                Ok(WorkflowExecutionEventPayload::Jumped(Jumped::new(
                    parse_direction(object.string("direction")?)?,
                    parse_slug(object.string("source")?)?,
                    parse_slug(object.string("target")?)?,
                    object.slugs("stages_reset")?,
                    object.slugs("stages_skipped")?,
                )))
            }
            "Parked" => {
                object.only(&["type", "stage"])?;
                Ok(WorkflowExecutionEventPayload::Parked(Parked::new(
                    parse_slug(object.string("stage")?)?,
                )))
            }
            "Unparked" => {
                object.only(&["type"])?;
                Ok(WorkflowExecutionEventPayload::Unparked)
            }
            "Recomposed" => {
                object.only(&["type", "skipped", "added", "stages_in_scope"])?;
                Ok(WorkflowExecutionEventPayload::Recomposed(Recomposed::new(
                    object.slugs("skipped")?,
                    object.slugs("added")?,
                    object.slugs("stages_in_scope")?,
                )))
            }
            "AutonomyModeSet" => {
                object.only(&["type", "mode"])?;
                Ok(WorkflowExecutionEventPayload::AutonomyModeSet(
                    AutonomyModeSet::new(super::parse_autonomy(object.string("mode")?)?),
                ))
            }
            // `EVENT_TYPES` の閉集合検査を先に通しているので到達しない。
            _ => Err(CorruptCause::UnknownEventType),
        }
    }
}

/// `slug | null` の読取。
fn optional_slug(object: &WireObject<'_>, key: &str) -> Result<Option<StageSlug>, CorruptCause> {
    match object.optional_string(key)? {
        None => Ok(None),
        Some(text) => parse_slug(text).map(Some),
    }
}

/// `phase_boundary` (入れ子オブジェクト or null) の読取。
fn decode_phase_boundary(object: &WireObject<'_>) -> Result<Option<PhaseBoundary>, CorruptCause> {
    let value = object.value("phase_boundary")?;
    match value {
        canon_json::JsonValue::Null => Ok(None),
        other => {
            let boundary = WireObject::new(other)?;
            boundary.only(&PHASE_BOUNDARY_KEYS)?;
            let from: PhaseId = parse_phase(boundary.string("from_phase")?)?;
            let to: PhaseId = parse_phase(boundary.string("to_phase")?)?;
            Ok(Some(PhaseBoundary::new(from, to)))
        }
    }
}

#[cfg(test)]
mod tests {
    // テストは固定長フィクスチャの添字参照を許容 (clippy.toml に相当設定が無いため file 単位で
    // allow)。panic! は想定外バリアントの即時失敗という検証用途で使っており、テスト失敗の
    // シグナルとして妥当なため同様に許容する。
    #![allow(clippy::indexing_slicing, clippy::panic)]

    use super::*;
    use core_domain::orchestration::{
        AutonomyMode, AutonomyModeSet, GateApproved, GateOpened, GateRejected, JumpDirection,
        Jumped, Parked, PhaseBoundary, Recomposed, StageCompleted, StageEntry, StageRevised,
        StageSkipped, StartRequest, Started, WorkflowExecutionEventPayload,
    };
    use core_domain::workflow_definition::{
        DefinitionRevision, PhaseId, PlanAction, StageSlug, WorkflowDefinitionId,
    };
    use core_use_case::orchestration::{CorruptCause, EventStoreError};
    use proptest::prelude::*;

    const AGG: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

    fn slug(s: &str) -> StageSlug {
        StageSlug::parse(s).unwrap()
    }

    fn revision() -> DefinitionRevision {
        DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap()
    }

    fn entry() -> StageEntry {
        StageEntry::new(
            slug("state-init"),
            PhaseId::Initialization,
            PlanAction::Execute,
            false,
        )
    }

    fn encode(payload: &WorkflowExecutionEventPayload) -> String {
        EventPayloadWire::encode(AGG, 3, payload).unwrap()
    }

    fn decode(
        event_type: &str,
        json: &str,
    ) -> Result<WorkflowExecutionEventPayload, EventStoreError> {
        EventPayloadWire::decode(AGG, 3, EventPayloadWire::SCHEMA_VERSION, event_type, json)
    }

    fn round_trip(payload: &WorkflowExecutionEventPayload) -> WorkflowExecutionEventPayload {
        let json = encode(payload);
        decode(EventPayloadWire::event_type(payload), &json).unwrap()
    }

    fn cause(err: &EventStoreError) -> CorruptCause {
        match err {
            EventStoreError::Corrupt { cause, .. } => *cause,
            other => panic!("expected Corrupt, got {other}"),
        }
    }

    fn samples() -> Vec<WorkflowExecutionEventPayload> {
        vec![
            WorkflowExecutionEventPayload::Started(Started::new(
                WorkflowDefinitionId::parse("claude").unwrap(),
                revision(),
                &StartRequest::new("classic", "build it")
                    .with_depth("standard")
                    .with_test_strategy("minimal"),
                vec![entry()],
            )),
            WorkflowExecutionEventPayload::StageCompleted(StageCompleted::new(
                slug("state-init"),
                Some(slug("intent-capture")),
            )),
            WorkflowExecutionEventPayload::GateOpened(GateOpened::new(
                slug("intent-capture"),
                vec!["intent.md".to_string()],
            )),
            WorkflowExecutionEventPayload::GateApproved(GateApproved::new(
                slug("intent-capture"),
                Some("looks good".to_string()),
                None,
                Some(PhaseBoundary::new(PhaseId::Ideation, PhaseId::Inception)),
            )),
            WorkflowExecutionEventPayload::GateRejected(GateRejected::new(
                slug("intent-capture"),
                None,
                2,
            )),
            WorkflowExecutionEventPayload::StageRevised(StageRevised::new(slug("intent-capture"))),
            WorkflowExecutionEventPayload::StageSkipped(StageSkipped::new(
                slug("market-research"),
                "out of scope".to_string(),
                Some(slug("intent-capture")),
            )),
            WorkflowExecutionEventPayload::Jumped(Jumped::new(
                JumpDirection::Backward,
                slug("intent-capture"),
                slug("state-init"),
                vec![slug("intent-capture")],
                Vec::new(),
            )),
            WorkflowExecutionEventPayload::Parked(Parked::new(slug("intent-capture"))),
            WorkflowExecutionEventPayload::Unparked,
            WorkflowExecutionEventPayload::Recomposed(Recomposed::new(
                vec![slug("market-research")],
                Vec::new(),
                vec![slug("state-init")],
            )),
            WorkflowExecutionEventPayload::AutonomyModeSet(AutonomyModeSet::new(
                AutonomyMode::Autonomous,
            )),
        ]
    }

    #[test]
    fn every_one_of_the_twelve_variants_round_trips() {
        let payloads = samples();
        assert_eq!(payloads.len(), 12);
        for payload in &payloads {
            assert_eq!(&round_trip(payload), payload);
        }
    }

    #[test]
    fn the_type_tag_is_the_first_member_and_names_the_variant() {
        let json = encode(&WorkflowExecutionEventPayload::Unparked);
        assert_eq!(json, r#"{"type":"Unparked"}"#);
        let json = encode(&WorkflowExecutionEventPayload::Parked(Parked::new(slug(
            "intent-capture",
        ))));
        assert_eq!(json, r#"{"type":"Parked","stage":"intent-capture"}"#);
    }

    #[test]
    fn the_fixed_tokens_keep_their_upstream_spelling() {
        let json = encode(&samples()[0]);
        assert!(json.contains(r#""plan_action":"EXECUTE""#), "{json}");
        assert!(json.contains(r#""phase":"initialization""#), "{json}");
        let json = encode(&WorkflowExecutionEventPayload::AutonomyModeSet(
            AutonomyModeSet::new(AutonomyMode::Autonomous),
        ));
        assert_eq!(json, r#"{"type":"AutonomyModeSet","mode":"autonomous"}"#);
    }

    #[test]
    fn an_unknown_type_tag_is_rejected_as_an_unknown_event_type() {
        let err = decode("Exploded", r#"{"type":"Exploded"}"#).unwrap_err();
        assert_eq!(cause(&err), CorruptCause::UnknownEventType);
    }

    #[test]
    fn an_event_type_column_that_disagrees_with_the_payload_tag_is_rejected() {
        let payload = WorkflowExecutionEventPayload::Unparked;
        let err = decode("Parked", &encode(&payload)).unwrap_err();
        assert_eq!(cause(&err), CorruptCause::UndecodablePayload);
    }

    #[test]
    fn an_unknown_field_is_rejected() {
        let json = r#"{"type":"Parked","stage":"intent-capture","extra":1}"#;
        let err = decode("Parked", json).unwrap_err();
        assert_eq!(cause(&err), CorruptCause::UndecodablePayload);
    }

    #[test]
    fn a_missing_field_is_rejected() {
        let err = decode("Parked", r#"{"type":"Parked"}"#).unwrap_err();
        assert_eq!(cause(&err), CorruptCause::UndecodablePayload);
    }

    #[test]
    fn a_schema_version_other_than_one_is_rejected() {
        let err = EventPayloadWire::decode(
            AGG,
            3,
            2,
            "Unparked",
            &encode(&WorkflowExecutionEventPayload::Unparked),
        )
        .unwrap_err();
        assert_eq!(cause(&err), CorruptCause::SchemaVersion);
    }

    #[test]
    fn a_json_type_mismatch_is_rejected() {
        let err = decode("Parked", r#"{"type":"Parked","stage":7}"#).unwrap_err();
        assert_eq!(cause(&err), CorruptCause::UndecodablePayload);
    }

    #[test]
    fn a_value_that_no_domain_primitive_accepts_is_rejected() {
        let err = decode("Parked", r#"{"type":"Parked","stage":"Not A Slug"}"#).unwrap_err();
        assert_eq!(cause(&err), CorruptCause::UndecodablePayload);
        let broken_plan = encode(&samples()[0]).replace(r#""EXECUTE""#, r#""execute""#);
        let err = decode("Started", &broken_plan).unwrap_err();
        assert_eq!(cause(&err), CorruptCause::UndecodablePayload);
    }

    #[test]
    fn a_syntactically_broken_payload_is_rejected() {
        let err = decode("Parked", "{not json").unwrap_err();
        assert_eq!(cause(&err), CorruptCause::UndecodablePayload);
    }

    #[test]
    fn a_payload_tag_outside_the_closed_set_is_rejected_even_when_the_column_is_inside_it() {
        // 閉集合の検査は列と payload の**両方**に掛かる。列だけを見て通すと、行の
        // どちらか一方が改竄された場合に検出できなくなる (NFR4.4)。
        // 「列も payload も閉集合内だが食い違う」場合とは原因が違う
        // (`UndecodablePayload` ではなく `UnknownEventType`)。
        let err = decode("Parked", r#"{"type":"Exploded","stage":"intent-capture"}"#).unwrap_err();
        assert_eq!(cause(&err), CorruptCause::UnknownEventType);
    }

    #[test]
    fn the_corrupt_material_carries_the_aggregate_and_the_sequence() {
        let err = decode("Parked", "{not json").unwrap_err();
        assert_eq!(
            err,
            EventStoreError::Corrupt {
                aggregate_id: AGG.to_string(),
                seq_nr: Some(3),
                cause: CorruptCause::UndecodablePayload,
            }
        );
    }

    // ---- PBT (BR2.5 / NFR2.2)。シードは `PROPTEST_RNG_SEED` で固定する ----

    fn text() -> impl Strategy<Value = String> {
        proptest::collection::vec(
            prop_oneof![
                Just('a'),
                Just('Z'),
                Just('0'),
                Just(' '),
                Just('"'),
                Just('\\'),
                Just('\n'),
                Just('\t'),
                Just('/'),
                Just('あ'),
                Just('\u{7f}'),
                Just('\u{2028}'),
            ],
            0..8,
        )
        .prop_map(|chars| chars.into_iter().collect())
    }

    fn slug_strategy() -> impl Strategy<Value = StageSlug> {
        "[a-z][a-z0-9-]{0,10}".prop_map(|s| StageSlug::parse(&s).unwrap())
    }

    fn slugs() -> impl Strategy<Value = Vec<StageSlug>> {
        proptest::collection::vec(slug_strategy(), 0..4)
    }

    fn phase_strategy() -> impl Strategy<Value = PhaseId> {
        prop_oneof![
            Just(PhaseId::Initialization),
            Just(PhaseId::Ideation),
            Just(PhaseId::Inception),
            Just(PhaseId::Construction),
            Just(PhaseId::Operation),
        ]
    }

    fn entry_strategy() -> impl Strategy<Value = StageEntry> {
        (
            slug_strategy(),
            phase_strategy(),
            prop_oneof![Just(PlanAction::Execute), Just(PlanAction::Skip)],
            any::<bool>(),
        )
            .prop_map(|(slug, phase, action, conditional)| {
                StageEntry::new(slug, phase, action, conditional)
            })
    }

    fn payload_strategy() -> impl Strategy<Value = WorkflowExecutionEventPayload> {
        let started = (
            "[a-z][a-z0-9-]{0,8}",
            "[0-9a-f]{64}",
            text(),
            text(),
            proptest::option::of(text()),
            proptest::option::of(text()),
            proptest::collection::vec(entry_strategy(), 1..4),
        )
            .prop_map(|(id, hex, scope, request, depth, strategy, stages)| {
                let mut req = StartRequest::new(scope, request);
                if let Some(depth) = depth {
                    req = req.with_depth(depth);
                }
                if let Some(strategy) = strategy {
                    req = req.with_test_strategy(strategy);
                }
                WorkflowExecutionEventPayload::Started(Started::new(
                    WorkflowDefinitionId::parse(&id).unwrap(),
                    DefinitionRevision::parse(&format!("sha256:{hex}")).unwrap(),
                    &req,
                    stages,
                ))
            });
        let boundary = (phase_strategy(), phase_strategy())
            .prop_map(|(from, to)| PhaseBoundary::new(from, to));
        prop_oneof![
            started,
            (slug_strategy(), proptest::option::of(slug_strategy())).prop_map(|(s, n)| {
                WorkflowExecutionEventPayload::StageCompleted(StageCompleted::new(s, n))
            }),
            (slug_strategy(), proptest::collection::vec(text(), 0..3)).prop_map(|(s, a)| {
                WorkflowExecutionEventPayload::GateOpened(GateOpened::new(s, a))
            }),
            (
                slug_strategy(),
                proptest::option::of(text()),
                proptest::option::of(slug_strategy()),
                proptest::option::of(boundary),
            )
                .prop_map(|(s, u, n, b)| {
                    WorkflowExecutionEventPayload::GateApproved(GateApproved::new(s, u, n, b))
                }),
            (slug_strategy(), proptest::option::of(text()), any::<u32>()).prop_map(|(s, f, c)| {
                WorkflowExecutionEventPayload::GateRejected(GateRejected::new(s, f, c))
            }),
            slug_strategy()
                .prop_map(|s| WorkflowExecutionEventPayload::StageRevised(StageRevised::new(s))),
            (
                slug_strategy(),
                text(),
                proptest::option::of(slug_strategy())
            )
                .prop_map(|(s, r, n)| {
                    WorkflowExecutionEventPayload::StageSkipped(StageSkipped::new(s, r, n))
                }),
            (
                prop_oneof![
                    Just(JumpDirection::Forward),
                    Just(JumpDirection::Backward),
                    Just(JumpDirection::Redo),
                ],
                slug_strategy(),
                slug_strategy(),
                slugs(),
                slugs(),
            )
                .prop_map(|(d, s, t, r, k)| {
                    WorkflowExecutionEventPayload::Jumped(Jumped::new(d, s, t, r, k))
                }),
            slug_strategy().prop_map(|s| WorkflowExecutionEventPayload::Parked(Parked::new(s))),
            Just(WorkflowExecutionEventPayload::Unparked),
            (slugs(), slugs(), slugs()).prop_map(|(s, a, i)| {
                WorkflowExecutionEventPayload::Recomposed(Recomposed::new(s, a, i))
            }),
            prop_oneof![Just(AutonomyMode::Autonomous), Just(AutonomyMode::Gated)].prop_map(|m| {
                WorkflowExecutionEventPayload::AutonomyModeSet(AutonomyModeSet::new(m))
            }),
        ]
    }

    proptest! {
        #[test]
        fn an_arbitrary_payload_survives_the_round_trip(payload in payload_strategy()) {
            prop_assert_eq!(round_trip(&payload), payload);
        }

        #[test]
        fn the_same_payload_always_encodes_to_the_same_bytes(payload in payload_strategy()) {
            prop_assert_eq!(encode(&payload), encode(&payload));
        }

        #[test]
        fn the_event_type_is_the_tag_written_into_the_payload(payload in payload_strategy()) {
            let json = encode(&payload);
            let tag = EventPayloadWire::event_type(&payload);
            prop_assert!(json.starts_with(&format!(r#"{{"type":"{tag}""#)), "{}", json);
        }
    }
}
