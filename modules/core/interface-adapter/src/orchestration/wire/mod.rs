//! ワイヤ形式 — 行のテキスト列とドメイン型のあいだの写像 (BR2.5、functional-spec §4)。
//!
//! 符号化はここに閉じる (ドメインは serde を知らない — ADR-004)。復号は
//! parse-don't-validate: JSON をワイヤの形に読んだうえで、値は必ずドメインの `parse` を
//! 通してから組み立てる (security-design §2 の検査点 1 / 2)。どの段で失敗しても
//! `EventStoreError::Corrupt` になり、panic はしない (NFR4.3)。
//!
//! 本 mod 自体は `orchestration` の private mod。公開は `pub(crate)` までで、クレート外へは
//! 出さない (module-visibility.md)。
//!
//! # 共有の部品
//!
//! ファサードの下の 2 つのワイヤ (`event_wire` / `state_wire`) は、同じ固定トークンの語彙と
//! 同じ JSON 読取の作法を使う。両者が食い違うと同じ値がイベントとスナップショットで別々の
//! 綴りになるため、写像とリーダは本 mod が 1 か所で持つ。

mod event_wire;
mod state_wire;

pub(crate) use event_wire::EventPayloadWire;
pub(crate) use state_wire::StateWire;

use canon_json::{JsonValue, Number, ObjectMembers, SerializationProfile};
use core_domain::orchestration::{AutonomyMode, IntentId, JumpDirection, StageEntry, Status};
use core_domain::workflow_definition::{
    DefinitionRevision, PhaseId, PlanAction, StageSlug, WorkflowDefinitionId,
};
use core_domain::workspace::CheckboxState;
use core_use_case::orchestration::{CorruptCause, EventStoreError};
use serde::Serialize;

/// ワイヤの版 (C5 `schema_version: 1`)。列の値がこれと違えば復号しない。
pub(super) const SCHEMA_VERSION: u32 = 1;

/// JSON (JS の `Number`) が整数を正確に保てる上限。これを超える整数は canon-json が
/// f64 経路で書くため、往復が非可逆になる (canon-json BR1.3)。書込前に弾く。
const MAX_EXACT_INTEGER: u64 = 9_007_199_254_740_992;

/// 行の材料を添えて `Corrupt` を組む。
pub(super) fn corrupt(
    aggregate_id: &str,
    seq_nr: Option<u64>,
    cause: CorruptCause,
) -> EventStoreError {
    EventStoreError::Corrupt {
        aggregate_id: aggregate_id.to_string(),
        seq_nr,
        cause,
    }
}

/// 正準 JSON (空白なし・宣言順) へ符号化する。
///
/// プロファイルは `ContractCompact` — キー順は構造体の宣言順で、同じ値は常に同じバイト列に
/// なる。`HashCanonical` (再帰ソート) はハッシュ入力のためのプロファイルなので使わない。
pub(super) fn to_canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String, CorruptCause> {
    let json = canon_json::to_value(value).map_err(|_| CorruptCause::UndecodablePayload)?;
    Ok(canon_json::serialize(
        &json,
        SerializationProfile::ContractCompact,
    ))
}

/// 信頼しない入力の唯一の読取口 (深さ上限つき — canon-json NFR4.3)。
pub(super) fn parse_json(text: &str) -> Result<JsonValue, CorruptCause> {
    canon_json::parse(text).map_err(|_| CorruptCause::UndecodablePayload)
}

/// JSON が正確に保てる整数か検査する (書込側の防波堤)。
///
/// 実運用の `seq_nr` / `version` は永続化済みイベント数なのでこの上限には遠く届かないが、
/// 超えた値を黙って丸めて書くと再構成が静かに壊れるため、書く前に拒否する。
pub(super) const fn exact_integer(value: u64) -> Result<u64, CorruptCause> {
    if value > MAX_EXACT_INTEGER {
        return Err(CorruptCause::InvariantViolation);
    }
    Ok(value)
}

/// JSON オブジェクトの読取り (未知フィールドを許さない parse-don't-validate のリーダ)。
///
/// 取り出しはすべて `Result` で、型不一致・欠落・値の形式違反は
/// [`CorruptCause::UndecodablePayload`] になる。添字アクセスは使わない。
pub(super) struct WireObject<'a> {
    members: &'a ObjectMembers,
}

impl<'a> WireObject<'a> {
    /// 値がオブジェクトであることを確かめて読取口を開く。
    pub(super) const fn new(value: &'a JsonValue) -> Result<WireObject<'a>, CorruptCause> {
        match value {
            JsonValue::Object(members) => Ok(WireObject { members }),
            _ => Err(CorruptCause::UndecodablePayload),
        }
    }

    /// 許可キー以外が 1 つでもあれば拒否する (`deny_unknown_fields` 相当 — 検査点 1)。
    pub(super) fn only(&self, allowed: &[&str]) -> Result<(), CorruptCause> {
        for (key, _) in self.members.iter() {
            if !allowed.contains(&key) {
                return Err(CorruptCause::UndecodablePayload);
            }
        }
        Ok(())
    }

    fn value(&self, key: &str) -> Result<&'a JsonValue, CorruptCause> {
        self.members
            .get(key)
            .ok_or(CorruptCause::UndecodablePayload)
    }

    /// 必須の文字列。
    pub(super) fn string(&self, key: &str) -> Result<&'a str, CorruptCause> {
        match self.value(key)? {
            JsonValue::String(text) => Ok(text),
            _ => Err(CorruptCause::UndecodablePayload),
        }
    }

    /// `string | null` の文字列 (`null` は `None`)。キー自体の欠落は拒否する。
    pub(super) fn optional_string(&self, key: &str) -> Result<Option<&'a str>, CorruptCause> {
        match self.value(key)? {
            JsonValue::String(text) => Ok(Some(text)),
            JsonValue::Null => Ok(None),
            _ => Err(CorruptCause::UndecodablePayload),
        }
    }

    /// 必須の非負整数。
    pub(super) fn u64(&self, key: &str) -> Result<u64, CorruptCause> {
        match self.value(key)? {
            JsonValue::Number(Number::PosInt(value)) => Ok(*value),
            _ => Err(CorruptCause::UndecodablePayload),
        }
    }

    /// `number | null` の非負整数 (`null` は `None`)。
    pub(super) fn optional_u64(&self, key: &str) -> Result<Option<u64>, CorruptCause> {
        match self.value(key)? {
            JsonValue::Number(Number::PosInt(value)) => Ok(Some(*value)),
            JsonValue::Null => Ok(None),
            _ => Err(CorruptCause::UndecodablePayload),
        }
    }

    /// 必須の `u32` (範囲外は拒否 — 索引・回数の値域)。
    pub(super) fn u32(&self, key: &str) -> Result<u32, CorruptCause> {
        u32::try_from(self.u64(key)?).map_err(|_| CorruptCause::UndecodablePayload)
    }

    /// 必須の `usize` (範囲外は拒否)。
    pub(super) fn usize(&self, key: &str) -> Result<usize, CorruptCause> {
        usize::try_from(self.u64(key)?).map_err(|_| CorruptCause::UndecodablePayload)
    }

    /// `number | null` の `usize`。
    pub(super) fn optional_usize(&self, key: &str) -> Result<Option<usize>, CorruptCause> {
        match self.optional_u64(key)? {
            None => Ok(None),
            Some(value) => usize::try_from(value)
                .map(Some)
                .map_err(|_| CorruptCause::UndecodablePayload),
        }
    }

    /// 必須の配列。
    pub(super) fn array(&self, key: &str) -> Result<&'a [JsonValue], CorruptCause> {
        match self.value(key)? {
            JsonValue::Array(items) => Ok(items),
            _ => Err(CorruptCause::UndecodablePayload),
        }
    }

    /// 配列の各要素を写して集める。
    pub(super) fn list<T>(
        &self,
        key: &str,
        map: impl Fn(&'a JsonValue) -> Result<T, CorruptCause>,
    ) -> Result<Vec<T>, CorruptCause> {
        self.array(key)?.iter().map(map).collect()
    }

    /// 文字列の配列。
    pub(super) fn texts(&self, key: &str) -> Result<Vec<String>, CorruptCause> {
        self.list(key, |item| match item {
            JsonValue::String(text) => Ok(text.clone()),
            _ => Err(CorruptCause::UndecodablePayload),
        })
    }

    /// stage slug の配列。
    pub(super) fn slugs(&self, key: &str) -> Result<Vec<StageSlug>, CorruptCause> {
        self.list(key, |item| match item {
            JsonValue::String(text) => parse_slug(text),
            _ => Err(CorruptCause::UndecodablePayload),
        })
    }

    /// 真偽値の配列。
    pub(super) fn bools(&self, key: &str) -> Result<Vec<bool>, CorruptCause> {
        self.list(key, |item| match item {
            JsonValue::Bool(value) => Ok(*value),
            _ => Err(CorruptCause::UndecodablePayload),
        })
    }

    /// `u32` の配列。
    pub(super) fn u32s(&self, key: &str) -> Result<Vec<u32>, CorruptCause> {
        self.list(key, |item| match item {
            JsonValue::Number(Number::PosInt(value)) => {
                u32::try_from(*value).map_err(|_| CorruptCause::UndecodablePayload)
            }
            _ => Err(CorruptCause::UndecodablePayload),
        })
    }
}

// ---- 固定トークンの写像 (境界での写像は Tell-Don't-Ask の対象外 — tell-dont-ask.md) ----

/// stage slug。
pub(super) fn parse_slug(token: &str) -> Result<StageSlug, CorruptCause> {
    StageSlug::parse(token).map_err(|_| CorruptCause::UndecodablePayload)
}

/// 集約識別子 (UUIDv7)。
pub(super) fn parse_intent_id(token: &str) -> Result<IntentId, CorruptCause> {
    IntentId::parse(token).map_err(|_| CorruptCause::UndecodablePayload)
}

/// 定義の系譜 ID。
pub(super) fn parse_definition_id(token: &str) -> Result<WorkflowDefinitionId, CorruptCause> {
    WorkflowDefinitionId::parse(token).map_err(|_| CorruptCause::UndecodablePayload)
}

/// 定義の内容版 (`sha256:<hex64>`)。
pub(super) fn parse_definition_revision(token: &str) -> Result<DefinitionRevision, CorruptCause> {
    DefinitionRevision::parse(token).map_err(|_| CorruptCause::UndecodablePayload)
}

/// フェーズ (upstream 綴りの 5 語 — ドメインの写像をそのまま使う)。
pub(super) fn parse_phase(token: &str) -> Result<PhaseId, CorruptCause> {
    PhaseId::parse(token).map_err(|_| CorruptCause::UndecodablePayload)
}

/// 計画 (`EXECUTE` / `SKIP` — upstream 綴り)。
pub(super) fn parse_plan(token: &str) -> Result<PlanAction, CorruptCause> {
    PlanAction::parse(token).ok_or(CorruptCause::UndecodablePayload)
}

/// checkbox マーカー 1 文字 (upstream の 6 マーク)。ドメインの 1:1 写像に委ねる。
pub(super) fn checkbox_token(state: CheckboxState) -> String {
    state.marker().to_string()
}

/// checkbox マーカーの復号 (1 文字ちょうどでなければ拒否)。
pub(super) fn parse_checkbox(token: &str) -> Result<CheckboxState, CorruptCause> {
    let mut chars = token.chars();
    match (chars.next(), chars.next()) {
        (Some(marker), None) => {
            CheckboxState::from_marker(marker).ok_or(CorruptCause::UndecodablePayload)
        }
        _ => Err(CorruptCause::UndecodablePayload),
    }
}

/// 自律モード (upstream 綴り)。
///
/// ドメインの `AutonomyMode::read_state` は**状態ファイル読取用の fail-closed リーダ**で、
/// 未知値を gated へ畳む。破損検出の境界にその寛容さを持ち込むと改竄が検出できなくなるため
/// (NFR3.2 / NFR4.4)、ここは 2 語の閉集合として厳密に写す。
pub(super) const fn autonomy_token(mode: AutonomyMode) -> &'static str {
    match mode {
        AutonomyMode::Autonomous => "autonomous",
        AutonomyMode::Gated => "gated",
    }
}

/// 自律モードの復号 (閉集合の外は拒否)。
pub(super) fn parse_autonomy(token: &str) -> Result<AutonomyMode, CorruptCause> {
    match token {
        "autonomous" => Ok(AutonomyMode::Autonomous),
        "gated" => Ok(AutonomyMode::Gated),
        _ => Err(CorruptCause::UndecodablePayload),
    }
}

/// ワークフロー全体の状態 (状態ファイル `Status` 行と同じ 2 語)。
pub(super) const fn status_token(status: Status) -> &'static str {
    match status {
        Status::Running => "running",
        Status::Completed => "completed",
    }
}

/// 状態の復号 (閉集合の外は拒否)。
pub(super) fn parse_status(token: &str) -> Result<Status, CorruptCause> {
    match token {
        "running" => Ok(Status::Running),
        "completed" => Ok(Status::Completed),
        _ => Err(CorruptCause::UndecodablePayload),
    }
}

/// jump の方向 (3 値)。
pub(super) const fn direction_token(direction: JumpDirection) -> &'static str {
    match direction {
        JumpDirection::Forward => "forward",
        JumpDirection::Backward => "backward",
        JumpDirection::Redo => "redo",
    }
}

/// jump 方向の復号 (閉集合の外は拒否)。
pub(super) fn parse_direction(token: &str) -> Result<JumpDirection, CorruptCause> {
    match token {
        "forward" => Ok(JumpDirection::Forward),
        "backward" => Ok(JumpDirection::Backward),
        "redo" => Ok(JumpDirection::Redo),
        _ => Err(CorruptCause::UndecodablePayload),
    }
}

/// 解決済み計画 1 ステージ分のワイヤ表現 (`Started` と状態の両方に現れる)。
#[derive(Debug, Serialize)]
pub(super) struct StageEntryWire {
    slug: String,
    phase: &'static str,
    plan_action: &'static str,
    conditional: bool,
}

/// `StageEntryWire` が持つキーの閉集合。
const STAGE_ENTRY_KEYS: [&str; 4] = ["slug", "phase", "plan_action", "conditional"];

impl StageEntryWire {
    /// ドメイン値をワイヤの形へ。
    pub(super) fn from_entry(entry: &StageEntry) -> StageEntryWire {
        StageEntryWire {
            slug: entry.slug().as_str().to_string(),
            phase: entry.phase().as_str(),
            plan_action: entry.plan_action().as_str(),
            conditional: entry.is_conditional(),
        }
    }

    /// ワイヤの形からドメイン値へ (parse-don't-validate)。
    pub(super) fn to_entry(value: &JsonValue) -> Result<StageEntry, CorruptCause> {
        let object = WireObject::new(value)?;
        object.only(&STAGE_ENTRY_KEYS)?;
        Ok(StageEntry::new(
            parse_slug(object.string("slug")?)?,
            parse_phase(object.string("phase")?)?,
            parse_plan(object.string("plan_action")?)?,
            match object.value("conditional")? {
                JsonValue::Bool(value) => *value,
                _ => return Err(CorruptCause::UndecodablePayload),
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用の JSON リテラルを読取口に渡せる形へ読む。
    fn value(text: &str) -> JsonValue {
        parse_json(text).unwrap()
    }

    #[test]
    fn the_reader_opens_only_on_an_object() {
        // 配列やスカラの payload は「オブジェクトである」という前提の外なので、
        // 添字アクセスへ進む前にここで落とす。
        assert_eq!(
            WireObject::new(&value("[1]")).err(),
            Some(CorruptCause::UndecodablePayload)
        );
        assert_eq!(
            WireObject::new(&value("1")).err(),
            Some(CorruptCause::UndecodablePayload)
        );
        let object = value(r#"{"a":1}"#);
        assert!(WireObject::new(&object).is_ok());
    }

    #[test]
    fn a_nullable_string_accepts_null_but_refuses_another_type() {
        let json = value(r#"{"a":"x","b":null,"c":1}"#);
        let object = WireObject::new(&json).unwrap();
        assert_eq!(object.optional_string("a").unwrap(), Some("x"));
        assert_eq!(object.optional_string("b").unwrap(), None);
        assert_eq!(
            object.optional_string("c").err(),
            Some(CorruptCause::UndecodablePayload)
        );
    }

    #[test]
    fn a_nullable_number_accepts_null_but_refuses_another_type() {
        let json = value(r#"{"a":1,"b":null,"c":"1"}"#);
        let object = WireObject::new(&json).unwrap();
        assert_eq!(object.optional_u64("a").unwrap(), Some(1));
        assert_eq!(object.optional_u64("b").unwrap(), None);
        assert_eq!(
            object.optional_u64("c").err(),
            Some(CorruptCause::UndecodablePayload)
        );
        // `optional_usize` は同じ読取を通すので、型不一致も同じ原因になる。
        assert_eq!(
            object.optional_usize("c").err(),
            Some(CorruptCause::UndecodablePayload)
        );
        assert_eq!(object.optional_usize("b").unwrap(), None);
    }

    #[test]
    fn a_required_array_refuses_a_scalar() {
        let json = value(r#"{"a":[],"b":1}"#);
        let object = WireObject::new(&json).unwrap();
        assert!(object.array("a").unwrap().is_empty());
        assert_eq!(
            object.array("b").err(),
            Some(CorruptCause::UndecodablePayload)
        );
    }

    #[test]
    fn every_typed_array_reader_refuses_an_element_of_the_wrong_type() {
        // 配列そのものが読めても、要素の型が違えば行としては復号できない
        // (parse-don't-validate — 要素まで検査してからドメイン値を組む)。
        let json = value(
            r#"{"texts":["a",1],"slugs":["a",1],"bools":[true,1],"u32s":[1,"x"],"big":[4294967296]}"#,
        );
        let object = WireObject::new(&json).unwrap();
        assert_eq!(
            object.texts("texts").err(),
            Some(CorruptCause::UndecodablePayload)
        );
        assert_eq!(
            object.slugs("slugs").err(),
            Some(CorruptCause::UndecodablePayload)
        );
        assert_eq!(
            object.bools("bools").err(),
            Some(CorruptCause::UndecodablePayload)
        );
        assert_eq!(
            object.u32s("u32s").err(),
            Some(CorruptCause::UndecodablePayload)
        );
        assert_eq!(
            object.u32s("big").err(),
            Some(CorruptCause::UndecodablePayload),
            "u32 の値域外も同じ拒否"
        );
    }

    #[test]
    fn a_checkbox_marker_must_be_exactly_one_character() {
        assert_eq!(parse_checkbox("x").unwrap(), CheckboxState::Completed);
        assert_eq!(parse_checkbox(""), Err(CorruptCause::UndecodablePayload));
        assert_eq!(parse_checkbox("xx"), Err(CorruptCause::UndecodablePayload));
    }

    #[test]
    fn a_jump_direction_outside_the_three_words_is_rejected() {
        assert_eq!(parse_direction("redo").unwrap(), JumpDirection::Redo);
        assert_eq!(
            parse_direction("sideways"),
            Err(CorruptCause::UndecodablePayload)
        );
        // 綴りは upstream 固定トークンなので、大文字化も閉集合の外である。
        assert_eq!(
            parse_direction("Forward"),
            Err(CorruptCause::UndecodablePayload)
        );
    }

    #[test]
    fn a_stage_entry_needs_a_boolean_conditional() {
        let ok = value(
            r#"{"slug":"intent-capture","phase":"ideation","plan_action":"EXECUTE","conditional":true}"#,
        );
        let entry = StageEntryWire::to_entry(&ok).unwrap();
        assert_eq!(entry.slug().as_str(), "intent-capture");
        assert!(entry.is_conditional());

        let broken = value(
            r#"{"slug":"intent-capture","phase":"ideation","plan_action":"EXECUTE","conditional":1}"#,
        );
        assert_eq!(
            StageEntryWire::to_entry(&broken).err(),
            Some(CorruptCause::UndecodablePayload)
        );
    }
}
