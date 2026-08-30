//! `ContinueTokenCodec` の実 Gateway — ドメイン型と upstream ワイヤ形式の**変換**だけを持つ。
//!
//! 封緘そのもの (HMAC-SHA256 封筒 `{p, m}` の base64url — 02 §4.4) は**純粋なコーデック**で
//! あり、言語拡張の [`core_infrastructure::codec`] が持つ (オーナー裁定 2026-08-30
//! 「I/O を含まない純粋なコーデックロジックをインフラストラクチャ層に配置せよ」)。ここに
//! 残るのは upstream 固有のもの — 18 キーの 1 文字綴り・センチネル・厳密型表・ドメイン型への
//! parse である (`coding-rules/upstream-contracts.md`「境界で変換」)。
//!
//! **本 Gateway は I/O を持たない** — 計算だけである (オーナー裁定 2026-08-30「コーデックの
//! 計算部分と I/O 部分を分離しろ」)。鍵はマシンローカルの `.aidlc-steering-token-key` を
//! 遅延鋳造した**結果のバイト列**を受け取る (I8 の例外 1 — "machine-local runtime state,
//! not a project-derived value an untrusted continuation can recompute")。鋳造そのものは
//! 鋳造そのものは同じアダプタ層の機構モジュール [`crate::read_or_mint_secret`] が持ち、
//! 配線するのは合成ルートである。
//! 検証は timing-safe (`Mac::verify_slice`)。デコードは厳密型表 —
//! 未知キー・型違反は serde の `deny_unknown_fields` と型で拒否し、ドメイン型へ上げる
//! parse が文法違反 (slug・部索引 0・予約フラグの真値・unit 対の片割れ) を拒否する。
//!
//! ダイジェストの素材は**名前付き構造体の JSON 直列化** (エスケープ付き) である — `Debug`
//! 表現への依存は derive 変更で黙ってダイジェストが変わる時限爆弾であり、区切り文字連結は
//! 区切り文字注入を許すので、どちらも使わない (オーナー裁定 2026-08-30)。

use core_command_domain::orchestration::{
    Bindings, BundleDigest, ContinueToken, ContinueTokenBuilder, DirectiveDigest, GateField,
    IntentExecution, PartIndex, RouteDigest, RunStageDirective, StageName, StateBinding,
    SteeringPlan, TokenVersion, UnitKind, UnitName, UnitRef,
};
use core_command_domain::workflow_definition::{ScopeSlug, StageRoute, StageSlug};
use core_command_use_case::orchestration::{ContinueTokenCodec, InvalidContinueToken};
use core_infrastructure::codec::{digest_hex, seal, unseal};
use serde::{Deserialize, Serialize};

/// state 束縛なしのときワイヤ `h` に置くセンチネル (輸送形の詳細 — ドメインへは出さない)。
const NO_STATE_SENTINEL: &str = "-";

/// `gate` のワイヤ形 — boolean か `"unresolved"` のみ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum WireGate {
    /// `true` = ゲート付き / `false` = ゲートなし。
    Flag(bool),
    /// `"unresolved"` (walking-skeleton 判定待ち)。
    Text(String),
}

/// ペイロードのワイヤ形 (18 キー、upstream の 1 文字綴り)。フィールドの**並びが封緘バイト**を
/// 決めるので変更は破壊的 (トークンはセッションローカルなので互換負債にはならない)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WirePayload {
    v: u32,
    s: String,
    c: String,
    i: u32,
    b: String,
    d: String,
    r: String,
    a: bool,
    u: Option<String>,
    k: Option<String>,
    f: bool,
    g: WireGate,
    n: Option<String>,
    x: bool,
    p: bool,
    w: bool,
    z: bool,
    h: String,
}

/// run-stage ダイジェストの素材 (キー項目の名前付き直列化)。
#[derive(Serialize)]
struct DirectiveMaterial<'a> {
    stage: &'a str,
    gate: WireGate,
    stage_file: &'a str,
    memory_path: &'a str,
    next_stage: Option<&'a str>,
    unit: Option<UnitMaterial<'a>>,
    single: bool,
}

/// unit の素材 (名前 + 種別)。
#[derive(Serialize)]
struct UnitMaterial<'a> {
    name: &'a str,
    kind: &'a str,
}

/// ルール束ダイジェストの素材 (piece の読み順)。
#[derive(Serialize)]
struct PieceMaterial<'a> {
    path: &'a str,
    text: &'a str,
}

/// route ダイジェストの素材。
#[derive(Serialize)]
struct RouteMaterial<'a> {
    stage: &'a str,
    stages: Vec<&'a str>,
}

/// state 束縛の素材。`version` はストアが採番した楽観 version
/// ([`core_command_domain::orchestration::IntentExecution::version`]) である。
#[derive(Serialize)]
struct StateMaterial<'a> {
    intent_id: &'a str,
    seq_nr: usize,
    version: usize,
}

/// HMAC-SHA256 で封緘する codec の実 Gateway。
#[derive(Debug)]
pub struct ContinueTokenCodecImpl {
    key: Vec<u8>,
}

impl ContinueTokenCodecImpl {
    /// 鋳造済みの鍵バイト列を受け取って組む。
    ///
    /// 鍵の取得 (ファイル読み・乱数鋳造・書込) は**本型の責務ではない** — 合成ルートが
    /// 用意した鍵バイト列を渡す。おかげで本型は I/O を持たず、テストは一時ディレクトリを
    /// 作らずに鍵を直接渡せる。
    #[must_use]
    pub const fn new(key: Vec<u8>) -> ContinueTokenCodecImpl {
        ContinueTokenCodecImpl { key }
    }
}

fn wire_gate(gate: GateField) -> WireGate {
    match gate {
        GateField::Gated => WireGate::Flag(true),
        GateField::Ungated => WireGate::Flag(false),
        GateField::Unresolved => WireGate::Text("unresolved".to_string()),
    }
}

fn domain_gate(gate: &WireGate) -> Result<GateField, InvalidContinueToken> {
    match gate {
        WireGate::Flag(true) => Ok(GateField::Gated),
        WireGate::Flag(false) => Ok(GateField::Ungated),
        WireGate::Text(text) if text == "unresolved" => Ok(GateField::Unresolved),
        WireGate::Text(_) => Err(InvalidContinueToken),
    }
}

fn wire_payload(token: &ContinueToken) -> WirePayload {
    WirePayload {
        v: token.version().as_u32(),
        s: token.stage().as_str().to_string(),
        c: token.scope().as_str().to_string(),
        i: token.next_part_index().as_u32(),
        b: token.bindings().bundle().as_str().to_string(),
        d: token.bindings().directive().as_str().to_string(),
        r: token.bindings().route().as_str().to_string(),
        a: token.bindings().state().is_some(),
        u: token.unit().map(|unit| unit.name().as_str().to_string()),
        k: token.unit().map(|unit| unit.kind().as_str().to_string()),
        f: false,
        g: wire_gate(token.gate()),
        n: token.next_stage().map(|name| name.as_str().to_string()),
        x: token.is_single(),
        p: token.is_per_unit(),
        w: false,
        z: false,
        h: token.bindings().state().map_or_else(
            || NO_STATE_SENTINEL.to_string(),
            |state| state.as_str().to_string(),
        ),
    }
}

fn domain_token(payload: &WirePayload) -> Result<ContinueToken, InvalidContinueToken> {
    if !TokenVersion::from_raw(payload.v).is_supported() {
        return Err(InvalidContinueToken);
    }
    let stage = StageSlug::parse(&payload.s).map_err(|_| InvalidContinueToken)?;
    let scope = ScopeSlug::parse(&payload.c).map_err(|_| InvalidContinueToken)?;
    let index = PartIndex::from_raw(payload.i).ok_or(InvalidContinueToken)?;
    // 予約フラグ (`f`/`w`/`z`) — エンジンは今日この真値を構築しない (fail-closed)。
    if payload.f || payload.w || payload.z {
        return Err(InvalidContinueToken);
    }
    // state 束縛の対 (`a` + `h`) — aware なのにセンチネル、非 aware なのに実値は型表違反。
    let state = match (payload.a, payload.h.as_str()) {
        (true, NO_STATE_SENTINEL) | (false, _) if payload.a || payload.h != NO_STATE_SENTINEL => {
            return Err(InvalidContinueToken);
        }
        (true, digest) => Some(StateBinding::new(digest)),
        (false, _) => None,
    };
    // unit の対 (`u` + `k` + `p`) — 片割れ・per-unit フラグの不整合は型表違反。
    let unit = match (&payload.u, &payload.k) {
        (Some(name), Some(kind)) => Some(UnitRef::new(
            UnitName::parse(name).map_err(|_| InvalidContinueToken)?,
            UnitKind::parse(kind).map_err(|_| InvalidContinueToken)?,
        )),
        (None, None) => None,
        (Some(_), None) | (None, Some(_)) => return Err(InvalidContinueToken),
    };
    if payload.p != unit.is_some() {
        return Err(InvalidContinueToken);
    }
    let bindings = Bindings::new(
        BundleDigest::new(payload.b.clone()),
        DirectiveDigest::new(payload.d.clone()),
        RouteDigest::new(payload.r.clone()),
        state,
    );
    let mut builder =
        ContinueTokenBuilder::new(stage, scope, index, bindings, domain_gate(&payload.g)?);
    if let Some(unit) = unit {
        builder = builder.with_unit(unit);
    }
    if let Some(next_stage) = &payload.n {
        builder = builder
            .with_next_stage(StageName::parse(next_stage).map_err(|_| InvalidContinueToken)?);
    }
    if payload.x {
        builder = builder.with_single();
    }
    Ok(builder.build())
}

impl ContinueTokenCodec for ContinueTokenCodecImpl {
    fn mint(&self, token: &ContinueToken) -> String {
        // ここが持つのはドメイン型 → upstream ワイヤ形式の変換だけ。封緘は純粋コーデック。
        seal(&self.key, &wire_payload(token))
    }

    fn verify(&self, encoded: &str) -> Result<ContinueToken, InvalidContinueToken> {
        // 封緘を解くのは純粋コーデック (MAC 不一致・復号不能はどちらも `None` = fail-closed)。
        // ここが持つのは、解けた upstream ワイヤ形式をドメイン型へ上げる厳密型表である。
        let payload: WirePayload = unseal(&self.key, encoded).ok_or(InvalidContinueToken)?;
        domain_token(&payload)
    }

    fn bundle_digest(&self, plan: &SteeringPlan) -> BundleDigest {
        let pieces: Vec<PieceMaterial<'_>> = plan
            .chunks()
            .iter()
            .flatten()
            .map(|piece| PieceMaterial {
                path: piece.path(),
                text: piece.text(),
            })
            .collect();
        BundleDigest::new(digest_hex(&pieces))
    }

    fn directive_digest(&self, run_stage: &RunStageDirective) -> DirectiveDigest {
        let material = DirectiveMaterial {
            stage: run_stage.stage().as_str(),
            gate: wire_gate(run_stage.gate()),
            stage_file: run_stage.stage_file(),
            memory_path: run_stage.memory_path(),
            next_stage: run_stage.next_stage(),
            unit: run_stage.unit().map(|unit| UnitMaterial {
                name: unit.name().as_str(),
                kind: unit.kind().as_str(),
            }),
            single: run_stage.is_single(),
        };
        DirectiveDigest::new(digest_hex(&material))
    }

    fn route_digest(&self, route: &StageRoute) -> RouteDigest {
        let material = RouteMaterial {
            stage: route.stage().as_str(),
            stages: route
                .stages_in_scope()
                .iter()
                .map(StageSlug::as_str)
                .collect(),
        };
        RouteDigest::new(digest_hex(&material))
    }

    fn state_binding(&self, execution: &IntentExecution) -> StateBinding {
        let material = StateMaterial {
            intent_id: execution.intent_id().as_str(),
            seq_nr: execution.seq_nr(),
            version: execution.version(),
        };
        StateBinding::new(digest_hex(&material))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use core_command_domain::orchestration::GateField;

    /// 試験用の鍵 (鋳造は `core_infrastructure::secret_file` の責務なのでここでは要らない)。
    fn codec() -> ContinueTokenCodecImpl {
        ContinueTokenCodecImpl::new(vec![7u8; 32])
    }

    fn bindings() -> Bindings {
        Bindings::new(
            BundleDigest::new("sha256:bbbb"),
            DirectiveDigest::new("dddd"),
            RouteDigest::new("rrrr"),
            Some(StateBinding::new("hhhh")),
        )
    }

    fn token() -> ContinueToken {
        ContinueTokenBuilder::new(
            StageSlug::parse("functional-design").unwrap(),
            ScopeSlug::parse("classic").unwrap(),
            PartIndex::FIRST.next(),
            bindings(),
            GateField::Gated,
        )
        .with_unit(UnitRef::new(
            UnitName::parse("u4-read-model-updater").unwrap(),
            UnitKind::Library,
        ))
        .with_next_stage(StageName::parse("NFR Requirements").unwrap())
        .build()
    }

    #[test]
    fn a_minted_token_round_trips() {
        let codec = codec();
        let encoded = codec.mint(&token());
        assert_eq!(codec.verify(&encoded).unwrap(), token());
    }

    #[test]
    fn a_stateless_token_round_trips_with_the_sentinel_on_the_wire() {
        let codec = codec();
        let stateless = ContinueTokenBuilder::new(
            StageSlug::parse("functional-design").unwrap(),
            ScopeSlug::parse("classic").unwrap(),
            PartIndex::FIRST,
            Bindings::new(
                BundleDigest::new("b"),
                DirectiveDigest::new("d"),
                RouteDigest::new("r"),
                None,
            ),
            GateField::Unresolved,
        )
        .with_single()
        .build();
        let payload = wire_payload(&stateless);
        assert!(!payload.a);
        assert_eq!(payload.h, NO_STATE_SENTINEL, "センチネルは輸送形の詳細");
        assert_eq!(payload.g, WireGate::Text("unresolved".to_string()));
        let encoded = codec.mint(&stateless);
        let verified = codec.verify(&encoded).unwrap();
        assert!(verified.bindings().state().is_none());
        assert!(verified.is_single());
    }

    #[test]
    fn a_tampered_payload_fails_the_mac() {
        let codec = codec();
        let encoded = codec.mint(&token());
        let bytes = URL_SAFE_NO_PAD.decode(&encoded).unwrap();
        let tampered = String::from_utf8(bytes)
            .unwrap()
            .replace("functional-design", "domain-design");
        let reencoded = URL_SAFE_NO_PAD.encode(tampered.as_bytes());
        assert_eq!(codec.verify(&reencoded), Err(InvalidContinueToken));
    }

    #[test]
    fn garbage_and_wrong_key_fail_closed() {
        let subject = codec();
        assert_eq!(subject.verify("not-base64url!!"), Err(InvalidContinueToken));
        assert_eq!(subject.verify(""), Err(InvalidContinueToken));
        let other = ContinueTokenCodecImpl::new(vec![9u8; 32]);
        let encoded = other.mint(&token());
        assert_eq!(subject.verify(&encoded), Err(InvalidContinueToken));
    }

    #[test]
    fn the_strict_type_table_rejects_foreign_shapes() {
        // v ≠ 1、未知の gate 語、部索引 0、予約フラグの真値、unit 対の不整合は fail-closed。
        let mut wrong_version = wire_payload(&token());
        wrong_version.v = 2;
        assert_eq!(domain_token(&wrong_version), Err(InvalidContinueToken));
        assert_eq!(
            domain_gate(&WireGate::Text("weird".to_string())),
            Err(InvalidContinueToken)
        );
        assert_eq!(domain_gate(&WireGate::Flag(true)), Ok(GateField::Gated));
        assert_eq!(domain_gate(&WireGate::Flag(false)), Ok(GateField::Ungated));

        let mut zero_index = wire_payload(&token());
        zero_index.i = 0;
        assert_eq!(domain_token(&zero_index), Err(InvalidContinueToken));

        for flag in ["f", "w", "z"] {
            let mut reserved = wire_payload(&token());
            match flag {
                "f" => reserved.f = true,
                "w" => reserved.w = true,
                _ => reserved.z = true,
            }
            assert_eq!(domain_token(&reserved), Err(InvalidContinueToken));
        }

        let mut half_unit = wire_payload(&token());
        half_unit.k = None;
        assert_eq!(domain_token(&half_unit), Err(InvalidContinueToken));

        let mut per_unit_mismatch = wire_payload(&token());
        per_unit_mismatch.p = false;
        assert_eq!(domain_token(&per_unit_mismatch), Err(InvalidContinueToken));

        let mut unknown_kind = wire_payload(&token());
        unknown_kind.k = Some("weird".to_string());
        assert_eq!(domain_token(&unknown_kind), Err(InvalidContinueToken));

        let mut aware_without_digest = wire_payload(&token());
        aware_without_digest.h = NO_STATE_SENTINEL.to_string();
        assert_eq!(
            domain_token(&aware_without_digest),
            Err(InvalidContinueToken)
        );

        let mut stateless_with_digest = wire_payload(&token());
        stateless_with_digest.a = false;
        assert_eq!(
            domain_token(&stateless_with_digest),
            Err(InvalidContinueToken)
        );
    }

    #[test]
    fn an_unknown_key_is_rejected_by_the_strict_table() {
        let codec = codec();
        let encoded = codec.mint(&token());
        let bytes = URL_SAFE_NO_PAD.decode(&encoded).unwrap();
        let smuggled = String::from_utf8(bytes)
            .unwrap()
            .replace("\"v\":1", "\"v\":1,\"q\":1");
        let reencoded = URL_SAFE_NO_PAD.encode(smuggled.as_bytes());
        assert_eq!(codec.verify(&reencoded), Err(InvalidContinueToken));
    }

    #[test]
    fn the_same_key_verifies_a_token_minted_by_another_instance() {
        // 封緘は鍵だけに依る — 同じ鍵を渡した別インスタンスが検証できる (プロセスをまたいだ
        // 継続の本体)。鍵ファイルの遅延鋳造そのものは `core_infrastructure::secret_file` の
        // テストが固定している。
        let first = codec();
        let encoded = first.mint(&token());
        let second = codec();
        assert_eq!(second.verify(&encoded).unwrap(), token());
    }

    /// state 束縛の素材になる集約 — 版だけ差し替えられる形で組む。
    fn execution(version: usize) -> IntentExecution {
        use chrono::{DateTime, Utc};
        use core_command_domain::orchestration::{
            Created, Intent, IntentExecutionId, IntentId, StageDisplay, StageEntry, StartRequest,
            WorkspaceScan,
        };
        use core_command_domain::workflow_definition::{
            BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber,
            WorkflowDefinitionId,
        };
        let intent = Intent::from(Created::new(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            StartRequest::new("classic", "state binding"),
            vec![StageEntry::new(
                StageSlug::parse("state-init").unwrap(),
                PhaseId::Initialization,
                PlanAction::Execute,
                false,
                StageDisplay::new(StageNumber::parse("0.1").unwrap(), "Stage", "orchestrator")
                    .unwrap(),
            )],
            WorkspaceScan::new(
                BrownfieldGreenfield::Greenfield,
                "Unknown",
                "Unknown",
                "Unknown",
            )
            .unwrap(),
        ));
        let (execution, _event) = IntentExecution::start(
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
            &intent,
            DateTime::<Utc>::UNIX_EPOCH,
        );
        execution.with_version(version)
    }

    #[test]
    fn digests_are_deterministic_and_subject_specific() {
        use core_command_domain::orchestration::RuleContent;
        let codec = codec();
        let plan = SteeringPlan::new(vec![vec![RuleContent::new(
            "org.md".to_string(),
            "# Org\n".to_string(),
        )]]);
        assert_eq!(codec.bundle_digest(&plan), codec.bundle_digest(&plan));
        let other = SteeringPlan::new(vec![vec![RuleContent::new(
            "org.md".to_string(),
            "# Org2\n".to_string(),
        )]]);
        assert_ne!(codec.bundle_digest(&plan), codec.bundle_digest(&other));

        let route = StageRoute::new(
            StageSlug::parse("functional-design").unwrap(),
            vec![StageSlug::parse("intent-capture").unwrap()],
        );
        assert_eq!(codec.route_digest(&route), codec.route_digest(&route));

        let held = execution(4);
        assert_eq!(codec.state_binding(&held), codec.state_binding(&held));
        // 版が動けば束縛も動く — 通番が同じでも別の state である。
        let moved = execution(5);
        assert_ne!(codec.state_binding(&held), codec.state_binding(&moved));
    }

    #[test]
    fn the_directive_digest_reads_the_named_material_not_a_debug_dump() {
        use core_command_domain::orchestration::RunStageDirectiveBuilder;
        use core_command_domain::workflow_definition::PhaseId;
        use core_command_domain::workflow_definition::StageMode;
        let codec = codec();
        let run_stage = RunStageDirectiveBuilder::new(
            StageSlug::parse("functional-design").unwrap(),
            PhaseId::Inception,
            "aidlc-architect-agent",
            StageMode::Inline,
            GateField::Gated,
            "stage.md",
            "memory.md",
        )
        .build();
        let same = codec.directive_digest(&run_stage);
        assert_eq!(codec.directive_digest(&run_stage), same);
        let pinned_single = ContinueTokenBuilder::new(
            StageSlug::parse("functional-design").unwrap(),
            ScopeSlug::parse("classic").unwrap(),
            PartIndex::FIRST,
            Bindings::new(
                BundleDigest::new("b"),
                DirectiveDigest::new("d"),
                RouteDigest::new("r"),
                None,
            ),
            GateField::Gated,
        )
        .with_single()
        .build();
        let single = codec.directive_digest(&run_stage.with_pins(&pinned_single));
        assert_ne!(single, same, "single ピンはダイジェストに効く");
    }
}
