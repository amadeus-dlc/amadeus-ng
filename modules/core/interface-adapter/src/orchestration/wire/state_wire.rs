//! `StateWire` — `WorkflowExecutionState` (16 属性) の JSON 表現 (functional-spec §4.2)。

use core_domain::orchestration::{WorkflowExecutionState, WorkflowExecutionStateBuilder};
use core_domain::workspace::CheckboxState;
use core_use_case::orchestration::{CorruptCause, EventStoreError};
use serde::Serialize;

use super::{
    SCHEMA_VERSION, StageEntryWire, WireObject, autonomy_token, checkbox_token, corrupt,
    exact_integer, parse_autonomy, parse_checkbox, parse_definition_id, parse_definition_revision,
    parse_intent_id, parse_json, parse_plan, parse_status, status_token, to_canonical_json,
};

/// `WorkflowExecutionState` (16 属性) のワイヤ表現 (functional-spec §4.2)。
///
/// フィールドの宣言順がそのまま正準 JSON のキー順になる (canon-json の
/// `ContractCompact` は挿入順を保つ)。`revision_count` は列を増やさずここに含む (P4)。
#[derive(Debug, Serialize)]
pub(crate) struct StateWire {
    intent_id: String,
    definition_id: String,
    definition_revision: String,
    stages: Vec<StageEntryWire>,
    plan: Vec<&'static str>,
    overlay: Vec<&'static str>,
    conditional: Vec<bool>,
    checkbox: Vec<String>,
    cursor: u64,
    status: &'static str,
    parked_at: Option<u64>,
    autonomy: &'static str,
    approved: Vec<bool>,
    revision_count: Vec<u32>,
    seq_nr: u64,
    version: u64,
}

/// 16 属性のキー (閉集合 — 未知フィールドはここに無い名前すべて)。
const STATE_KEYS: [&str; 16] = [
    "intent_id",
    "definition_id",
    "definition_revision",
    "stages",
    "plan",
    "overlay",
    "conditional",
    "checkbox",
    "cursor",
    "status",
    "parked_at",
    "autonomy",
    "approved",
    "revision_count",
    "seq_nr",
    "version",
];

impl StateWire {
    /// ワイヤの版 (状態ワイヤも C5 と同じ 1)。
    pub(crate) const SCHEMA_VERSION: u32 = SCHEMA_VERSION;

    /// 状態を正準 JSON へ符号化する (`snapshot.payload` 列の値)。
    ///
    /// # Errors
    ///
    /// JSON が正確に保てない大きさの整数 (`Corrupt(InvariantViolation)`)、正準 JSON へ
    /// 写せない場合 (`Corrupt(UndecodablePayload)`) を返す。
    pub(crate) fn encode(
        aggregate_id: &str,
        state: &WorkflowExecutionState,
    ) -> Result<String, EventStoreError> {
        StateWire::encode_inner(state).map_err(|cause| corrupt(aggregate_id, None, cause))
    }

    /// スナップショット行の材料から状態を復元する (検査点 1 / 2 — security-design §2)。
    ///
    /// 集約不変条件の検査 (検査点 3) は呼出側の `WorkflowExecution::from_state` が行う。
    ///
    /// # Errors
    ///
    /// 版不一致 (`Corrupt(SchemaVersion)`)、構文・型・未知フィールド・値の形式違反
    /// (`Corrupt(UndecodablePayload)`) を返す。
    pub(crate) fn decode(
        aggregate_id: &str,
        schema_version: u32,
        payload: &str,
    ) -> Result<WorkflowExecutionState, EventStoreError> {
        StateWire::decode_inner(schema_version, payload)
            .map_err(|cause| corrupt(aggregate_id, None, cause))
    }

    /// 符号化の本体 (材料の付与は呼出側)。
    fn encode_inner(state: &WorkflowExecutionState) -> Result<String, CorruptCause> {
        let cursor = exact_integer(
            u64::try_from(state.cursor().value()).map_err(|_| CorruptCause::InvariantViolation)?,
        )?;
        let parked_at = match state.parked_at() {
            None => None,
            Some(index) => Some(exact_integer(
                u64::try_from(index.value()).map_err(|_| CorruptCause::InvariantViolation)?,
            )?),
        };
        let wire = StateWire {
            intent_id: state.intent_id().as_str().to_string(),
            definition_id: state.definition_id().as_str().to_string(),
            definition_revision: state.definition_revision().as_str().to_string(),
            stages: state
                .stages()
                .iter()
                .map(StageEntryWire::from_entry)
                .collect(),
            plan: state.plan().iter().map(|action| action.as_str()).collect(),
            overlay: state
                .overlay()
                .iter()
                .map(|action| action.as_str())
                .collect(),
            conditional: state.conditional().to_vec(),
            checkbox: state
                .checkbox()
                .iter()
                .map(|state| checkbox_token(*state))
                .collect(),
            cursor,
            status: status_token(state.status()),
            parked_at,
            autonomy: autonomy_token(state.autonomy()),
            approved: state.approved().to_vec(),
            revision_count: state.revision_count().to_vec(),
            seq_nr: exact_integer(state.seq_nr())?,
            version: exact_integer(state.version())?,
        };
        to_canonical_json(&wire)
    }

    /// 復号の本体 (材料の付与は呼出側)。
    fn decode_inner(
        schema_version: u32,
        payload: &str,
    ) -> Result<WorkflowExecutionState, CorruptCause> {
        if schema_version != SCHEMA_VERSION {
            return Err(CorruptCause::SchemaVersion);
        }
        let json = parse_json(payload)?;
        let object = WireObject::new(&json)?;
        object.only(&STATE_KEYS)?;
        let stages = object
            .array("stages")?
            .iter()
            .map(StageEntryWire::to_entry)
            .collect::<Result<Vec<_>, _>>()?;
        let plan = object.list("plan", |item| match item {
            canon_json::JsonValue::String(text) => parse_plan(text),
            _ => Err(CorruptCause::UndecodablePayload),
        })?;
        let overlay = object.list("overlay", |item| match item {
            canon_json::JsonValue::String(text) => parse_plan(text),
            _ => Err(CorruptCause::UndecodablePayload),
        })?;
        let checkbox: Vec<CheckboxState> = object.list("checkbox", |item| match item {
            canon_json::JsonValue::String(text) => parse_checkbox(text),
            _ => Err(CorruptCause::UndecodablePayload),
        })?;
        Ok(WorkflowExecutionStateBuilder::new(
            parse_intent_id(object.string("intent_id")?)?,
            parse_definition_id(object.string("definition_id")?)?,
            parse_definition_revision(object.string("definition_revision")?)?,
            stages,
        )
        .plan(plan)
        .overlay(overlay)
        .conditional(object.bools("conditional")?)
        .checkbox(checkbox)
        .cursor(object.usize("cursor")?)
        .status(parse_status(object.string("status")?)?)
        .parked_at(object.optional_usize("parked_at")?)
        .autonomy(parse_autonomy(object.string("autonomy")?)?)
        .approved(object.bools("approved")?)
        .revision_count(object.u32s("revision_count")?)
        .seq_nr(object.u64("seq_nr")?)
        .version(object.u64("version")?)
        .build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_domain::orchestration::{
        AutonomyMode, IntentId, StageEntry, Status, WorkflowExecutionState,
        WorkflowExecutionStateBuilder,
    };
    use core_domain::workflow_definition::{
        DefinitionRevision, PhaseId, PlanAction, StageSlug, WorkflowDefinitionId,
    };
    use core_domain::workspace::CheckboxState;
    use core_use_case::orchestration::{CorruptCause, EventStoreError};
    use proptest::prelude::*;

    const AGG: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

    fn slug(s: &str) -> StageSlug {
        StageSlug::parse(s).unwrap()
    }

    fn entries() -> Vec<StageEntry> {
        vec![
            StageEntry::new(
                slug("state-init"),
                PhaseId::Initialization,
                PlanAction::Execute,
                false,
            ),
            StageEntry::new(
                slug("intent-capture"),
                PhaseId::Ideation,
                PlanAction::Skip,
                true,
            ),
        ]
    }

    fn state() -> WorkflowExecutionState {
        WorkflowExecutionStateBuilder::new(
            IntentId::parse(AGG).unwrap(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            entries(),
        )
        .checkbox(vec![CheckboxState::Completed, CheckboxState::Revising])
        .cursor(1)
        .status(Status::Completed)
        .parked_at(Some(1))
        .autonomy(AutonomyMode::Autonomous)
        .approved(vec![true, false])
        .revision_count(vec![0, 3])
        .seq_nr(9)
        .version(8)
        .build()
    }

    fn encode(state: &WorkflowExecutionState) -> String {
        StateWire::encode(AGG, state).unwrap()
    }

    fn decode(json: &str) -> Result<WorkflowExecutionState, EventStoreError> {
        StateWire::decode(AGG, StateWire::SCHEMA_VERSION, json)
    }

    fn cause(err: &EventStoreError) -> CorruptCause {
        match err {
            EventStoreError::Corrupt { cause, .. } => *cause,
            other => panic!("expected Corrupt, got {other}"),
        }
    }

    #[test]
    fn the_sixteen_attributes_round_trip() {
        let original = state();
        let decoded = decode(&encode(&original)).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn the_encoded_object_lists_the_sixteen_attributes_in_the_declared_order() {
        let json = encode(&state());
        let value = canon_json::parse(&json).unwrap();
        let canon_json::JsonValue::Object(members) = &value else {
            panic!("the snapshot payload must be a JSON object: {json}");
        };
        let keys: Vec<&str> = members.iter().map(|(key, _)| key).collect();
        assert_eq!(keys, STATE_KEYS);
    }

    #[test]
    fn the_fixed_tokens_keep_their_upstream_spelling() {
        let json = encode(&state());
        assert!(json.contains(r#""plan":["EXECUTE","SKIP"]"#), "{json}");
        assert!(json.contains(r#""phase":"initialization""#), "{json}");
        assert!(json.contains(r#""checkbox":["x","R"]"#), "{json}");
        assert!(json.contains(r#""autonomy":"autonomous""#), "{json}");
        assert!(json.contains(r#""status":"completed""#), "{json}");
    }

    #[test]
    fn an_unknown_field_is_rejected() {
        let json = encode(&state()).replace(r#""version":8}"#, r#""version":8,"extra":1}"#);
        assert_eq!(
            cause(&decode(&json).unwrap_err()),
            CorruptCause::UndecodablePayload
        );
    }

    #[test]
    fn a_missing_field_is_rejected() {
        let json = encode(&state()).replace(r#","version":8"#, "");
        assert_eq!(
            cause(&decode(&json).unwrap_err()),
            CorruptCause::UndecodablePayload
        );
    }

    #[test]
    fn a_schema_version_other_than_one_is_rejected() {
        let err = StateWire::decode(AGG, 2, &encode(&state())).unwrap_err();
        assert_eq!(cause(&err), CorruptCause::SchemaVersion);
    }

    #[test]
    fn an_unknown_checkbox_marker_is_rejected() {
        let json = encode(&state()).replace(r#"["x","R"]"#, r#"["x","!"]"#);
        assert_eq!(
            cause(&decode(&json).unwrap_err()),
            CorruptCause::UndecodablePayload
        );
    }

    #[test]
    fn an_unknown_status_token_is_rejected() {
        let json = encode(&state()).replace(r#""status":"completed""#, r#""status":"halted""#);
        assert_eq!(
            cause(&decode(&json).unwrap_err()),
            CorruptCause::UndecodablePayload
        );
    }

    #[test]
    fn an_unknown_autonomy_token_is_rejected_instead_of_falling_back_to_gated() {
        // ドメインの `AutonomyMode::read_state` は状態ファイル読取用の fail-closed リーダで、
        // 未知値を gated に畳む。破損検出の境界ではその寛容さを持ち込まない (NFR3.2)。
        let json = encode(&state()).replace(r#""autonomy":"autonomous""#, r#""autonomy":"turbo""#);
        assert_eq!(
            cause(&decode(&json).unwrap_err()),
            CorruptCause::UndecodablePayload
        );
    }

    #[test]
    fn an_invalid_intent_id_is_rejected() {
        let json = encode(&state()).replace(AGG, "260822-stage1-selfhost");
        assert_eq!(
            cause(&decode(&json).unwrap_err()),
            CorruptCause::UndecodablePayload
        );
    }

    #[test]
    fn a_json_type_mismatch_is_rejected() {
        let json = encode(&state()).replace(r#""cursor":1"#, r#""cursor":"1""#);
        assert_eq!(
            cause(&decode(&json).unwrap_err()),
            CorruptCause::UndecodablePayload
        );
    }

    #[test]
    fn a_syntactically_broken_payload_is_rejected() {
        assert_eq!(
            cause(&decode("{oops").unwrap_err()),
            CorruptCause::UndecodablePayload
        );
    }

    #[test]
    fn the_corrupt_material_carries_the_aggregate_but_no_sequence() {
        let err = decode("{oops").unwrap_err();
        assert_eq!(
            err,
            EventStoreError::Corrupt {
                aggregate_id: AGG.to_string(),
                seq_nr: None,
                cause: CorruptCause::UndecodablePayload,
            }
        );
    }

    /// JSON (JS の `Number`) が整数を正確に保てる上限 — ワイヤの整数の値域。
    const JSON_EXACT_INTEGER_LIMIT: u64 = 9_007_199_254_740_992;

    #[test]
    fn an_integer_at_the_json_exact_limit_still_round_trips() {
        let original = WorkflowExecutionStateBuilder::new(
            IntentId::parse(AGG).unwrap(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            entries(),
        )
        .seq_nr(JSON_EXACT_INTEGER_LIMIT)
        .version(JSON_EXACT_INTEGER_LIMIT)
        .build();
        assert_eq!(decode(&encode(&original)).unwrap(), original);
    }

    #[test]
    fn an_integer_beyond_the_json_exact_limit_is_refused_instead_of_being_rounded() {
        // canon-json は 2^53 超の整数を JS と同じく f64 経由で書く (バイト一致のため)。
        // 黙って丸めて書くと再構成が静かに壊れるので、書く前に拒否する (NFR3.1)。
        let original = WorkflowExecutionStateBuilder::new(
            IntentId::parse(AGG).unwrap(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            entries(),
        )
        .seq_nr(JSON_EXACT_INTEGER_LIMIT + 1)
        .build();
        let err = StateWire::encode(AGG, &original).unwrap_err();
        assert_eq!(cause(&err), CorruptCause::InvariantViolation);
    }

    // ---- PBT (BR2.5 / NFR2.2)。シードは `PROPTEST_RNG_SEED` で固定する ----

    fn plan_strategy() -> impl Strategy<Value = PlanAction> {
        prop_oneof![Just(PlanAction::Execute), Just(PlanAction::Skip)]
    }

    fn checkbox_strategy() -> impl Strategy<Value = CheckboxState> {
        prop_oneof![
            Just(CheckboxState::Pending),
            Just(CheckboxState::InProgress),
            Just(CheckboxState::AwaitingApproval),
            Just(CheckboxState::Revising),
            Just(CheckboxState::Completed),
            Just(CheckboxState::Skipped),
        ]
    }

    fn entry_strategy() -> impl Strategy<Value = StageEntry> {
        (
            "[a-z][a-z0-9-]{0,10}",
            prop_oneof![
                Just(PhaseId::Initialization),
                Just(PhaseId::Ideation),
                Just(PhaseId::Inception),
                Just(PhaseId::Construction),
                Just(PhaseId::Operation),
            ],
            plan_strategy(),
            any::<bool>(),
        )
            .prop_map(|(s, phase, action, conditional)| {
                StageEntry::new(StageSlug::parse(&s).unwrap(), phase, action, conditional)
            })
    }

    fn state_strategy() -> impl Strategy<Value = WorkflowExecutionState> {
        proptest::collection::vec(entry_strategy(), 1..5)
            .prop_flat_map(|stages| {
                let count = stages.len();
                (
                    Just(stages),
                    "[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}",
                    "[a-z][a-z0-9-]{0,8}",
                    "[0-9a-f]{64}",
                    proptest::collection::vec(plan_strategy(), count..=count),
                    proptest::collection::vec(checkbox_strategy(), count..=count),
                    proptest::collection::vec(any::<bool>(), count..=count),
                    proptest::collection::vec(any::<u32>(), count..=count),
                    (
                        0usize..count,
                        proptest::option::of(0usize..count),
                        prop_oneof![Just(Status::Running), Just(Status::Completed)],
                        prop_oneof![Just(AutonomyMode::Autonomous), Just(AutonomyMode::Gated)],
                        0u64..=JSON_EXACT_INTEGER_LIMIT,
                        0u64..=JSON_EXACT_INTEGER_LIMIT,
                    ),
                )
            })
            .prop_map(
                |(
                    stages,
                    intent,
                    definition,
                    hex,
                    overlay,
                    checkbox,
                    approved,
                    revision_count,
                    (cursor, parked_at, status, autonomy, seq_nr, version),
                )| {
                    WorkflowExecutionStateBuilder::new(
                        IntentId::parse(&intent).unwrap(),
                        WorkflowDefinitionId::parse(&definition).unwrap(),
                        DefinitionRevision::parse(&format!("sha256:{hex}")).unwrap(),
                        stages,
                    )
                    .overlay(overlay)
                    .checkbox(checkbox)
                    .approved(approved)
                    .revision_count(revision_count)
                    .cursor(cursor)
                    .parked_at(parked_at)
                    .status(status)
                    .autonomy(autonomy)
                    .seq_nr(seq_nr)
                    .version(version)
                    .build()
                },
            )
    }

    proptest! {
        #[test]
        fn an_arbitrary_state_survives_the_round_trip(state in state_strategy()) {
            prop_assert_eq!(decode(&encode(&state)).unwrap(), state);
        }

        #[test]
        fn the_same_state_always_encodes_to_the_same_bytes(state in state_strategy()) {
            prop_assert_eq!(encode(&state), encode(&state));
        }
    }
}
