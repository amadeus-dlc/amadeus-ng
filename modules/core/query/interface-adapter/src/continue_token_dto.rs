//! `continue_token` の封緘 (mint) と開封 (verify) — クエリモデルと upstream ワイヤ形式の変換。
//!
//! **ポートではない** — 封緘は Presenter・開封は Controller (どちらも U7) の変換であり、
//! `next` / `continue` はどちらも型付きの [`ContinueToken`] だけを見る。
//!
//! 封緘そのもの (HMAC-SHA256 封筒 `{p, m}` の base64url — 02 §4.4) は**純粋なコーデック**で
//! あり、言語拡張の [`core_infrastructure::codec`] が持つ (オーナー裁定 2026-08-30
//! 「I/O を含まない純粋なコーデックロジックをインフラストラクチャ層に配置せよ」)。ここに
//! 残るのは upstream 固有のもの — 18 キーの 1 文字綴り・センチネル・厳密型表・クエリモデルへの
//! parse である (`coding-rules/upstream-contracts.md`「境界で変換」)。
//!
//! **I/O は無い** — 計算だけである。鍵はマシンローカルの `.aidlc-steering-token-key` を
//! 遅延鋳造した**結果のバイト列**を受け取る (I8 の例外 1 — "machine-local runtime state,
//! not a project-derived value an untrusted continuation can recompute")。鋳造そのものは
//! 合成ルートの責務である。検証は timing-safe (`Mac::verify_slice`)。デコードは厳密型表 —
//! 未知キー・型違反は serde の `deny_unknown_fields` と型で拒否し、クエリモデルへ上げる
//! parse が文法違反 (slug・部索引 0・予約フラグの真値・unit 対の片割れ) を拒否する。

use core_infrastructure::codec::{seal, unseal};
use core_query_use_case::orchestration::{
    Bindings, BundleDigest, ContinueToken, ContinueTokenBuilder, DirectiveDigest, GateField,
    PartIndex, RouteDigest, ScopeSlugView, StageName, StageSlugView, StateBinding, TokenVersion,
    UnitKind, UnitName, UnitRef,
};
use serde::{Deserialize, Serialize};

/// 無効なトークン (材料なし — 「無効」だけを約束する。fail-closed の逐語文言は呼出側の
/// wording が組む)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidContinueToken;

impl std::fmt::Display for InvalidContinueToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid continue token")
    }
}

impl std::error::Error for InvalidContinueToken {}

/// state 束縛なしのときワイヤ `h` に置くセンチネル (輸送形の詳細 — クエリモデルへは出さない)。
const NO_STATE_SENTINEL: &str = "-";

/// `gate` のワイヤ形 — boolean か `"unresolved"` のみ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum ContinueTokenGateDto {
    /// `true` = ゲート付き / `false` = ゲートなし。
    Flag(bool),
    /// `"unresolved"` (walking-skeleton 判定待ち)。
    Text(String),
}

/// ペイロードのワイヤ形 (18 キー、upstream の 1 文字綴り)。フィールドの**並びが封緘バイト**を
/// 決めるので変更は破壊的 (トークンはセッションローカルなので互換負債にはならない)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinueTokenPayloadDto {
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
    g: ContinueTokenGateDto,
    n: Option<String>,
    x: bool,
    p: bool,
    w: bool,
    z: bool,
    h: String,
}

/// ペイロードを封緘して encode 済みトークンを返す (U7 Presenter の変換)。
///
/// 鍵の取得 (ファイル読み・乱数鋳造・書込) は**この関数の責務ではない** — 合成ルートが
/// 用意した鍵バイト列を渡す。おかげでここは I/O を持たず、テストは一時ディレクトリを
/// 作らずに鍵を直接渡せる。
#[must_use]
pub fn mint_continue_token(key: &[u8], token: &ContinueToken) -> String {
    // ここが持つのはクエリモデル → upstream ワイヤ形式の変換だけ。封緘は純粋コーデック。
    seal(key, &to_payload_dto(token))
}

/// encode 済みトークンを検証してペイロードへ戻す (U7 Controller の変換)。
///
/// # Errors
///
/// デコード不能・MAC 不一致・厳密型表への違反はすべて [`InvalidContinueToken`]
/// (fail-closed — 区別は診断に不要で、契約は fresh `next` からのやり直しだけを指示する)。
pub fn verify_continue_token(
    key: &[u8],
    encoded: &str,
) -> Result<ContinueToken, InvalidContinueToken> {
    // 封緘を解くのは純粋コーデック (MAC 不一致・復号不能はどちらも `None` = fail-closed)。
    // ここが持つのは、解けた upstream ワイヤ形式をクエリモデルへ上げる厳密型表である。
    let payload: ContinueTokenPayloadDto = unseal(key, encoded).ok_or(InvalidContinueToken)?;
    query_token(&payload)
}

fn to_gate_dto(gate: GateField) -> ContinueTokenGateDto {
    match gate {
        GateField::Gated => ContinueTokenGateDto::Flag(true),
        GateField::Ungated => ContinueTokenGateDto::Flag(false),
        GateField::Unresolved => ContinueTokenGateDto::Text("unresolved".to_string()),
    }
}

fn query_gate(gate: &ContinueTokenGateDto) -> Result<GateField, InvalidContinueToken> {
    match gate {
        ContinueTokenGateDto::Flag(true) => Ok(GateField::Gated),
        ContinueTokenGateDto::Flag(false) => Ok(GateField::Ungated),
        ContinueTokenGateDto::Text(text) if text == "unresolved" => Ok(GateField::Unresolved),
        ContinueTokenGateDto::Text(_) => Err(InvalidContinueToken),
    }
}

fn to_payload_dto(token: &ContinueToken) -> ContinueTokenPayloadDto {
    ContinueTokenPayloadDto {
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
        g: to_gate_dto(token.gate()),
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

fn query_token(payload: &ContinueTokenPayloadDto) -> Result<ContinueToken, InvalidContinueToken> {
    if !TokenVersion::from_raw(payload.v).is_supported() {
        return Err(InvalidContinueToken);
    }
    let stage = StageSlugView::parse(&payload.s).map_err(|_| InvalidContinueToken)?;
    let scope = ScopeSlugView::parse(&payload.c).map_err(|_| InvalidContinueToken)?;
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
        ContinueTokenBuilder::new(stage, scope, index, bindings, query_gate(&payload.g)?);
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

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;

    /// 試験用の鍵 (鋳造は `core_infrastructure::secret_file` の責務なのでここでは要らない)。
    const KEY: &[u8] = &[7u8; 32];

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
            StageSlugView::parse("functional-design").unwrap(),
            ScopeSlugView::parse("classic").unwrap(),
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
        let encoded = mint_continue_token(KEY, &token());
        assert_eq!(verify_continue_token(KEY, &encoded).unwrap(), token());
    }

    #[test]
    fn an_ungated_token_round_trips_with_the_boolean_gate() {
        let ungated = ContinueTokenBuilder::new(
            StageSlugView::parse("functional-design").unwrap(),
            ScopeSlugView::parse("classic").unwrap(),
            PartIndex::FIRST,
            Bindings::new(
                BundleDigest::new("b"),
                DirectiveDigest::new("d"),
                RouteDigest::new("r"),
                None,
            ),
            GateField::Ungated,
        )
        .build();
        assert_eq!(
            to_payload_dto(&ungated).g,
            ContinueTokenGateDto::Flag(false)
        );
        let encoded = mint_continue_token(KEY, &ungated);
        assert_eq!(
            verify_continue_token(KEY, &encoded).unwrap().gate(),
            GateField::Ungated
        );
    }

    #[test]
    fn the_invalid_token_error_has_a_stable_face() {
        assert_eq!(InvalidContinueToken.to_string(), "invalid continue token");
        let boxed: Box<dyn std::error::Error> = Box::new(InvalidContinueToken);
        assert_eq!(boxed.to_string(), "invalid continue token");
    }

    #[test]
    fn a_stateless_token_round_trips_with_the_sentinel_on_the_wire() {
        let stateless = ContinueTokenBuilder::new(
            StageSlugView::parse("functional-design").unwrap(),
            ScopeSlugView::parse("classic").unwrap(),
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
        let payload = to_payload_dto(&stateless);
        assert!(!payload.a);
        assert_eq!(payload.h, NO_STATE_SENTINEL, "センチネルは輸送形の詳細");
        assert_eq!(
            payload.g,
            ContinueTokenGateDto::Text("unresolved".to_string())
        );
        let encoded = mint_continue_token(KEY, &stateless);
        let verified = verify_continue_token(KEY, &encoded).unwrap();
        assert!(verified.bindings().state().is_none());
        assert!(verified.is_single());
    }

    #[test]
    fn a_tampered_payload_fails_the_mac() {
        let encoded = mint_continue_token(KEY, &token());
        let bytes = URL_SAFE_NO_PAD.decode(&encoded).unwrap();
        let tampered = String::from_utf8(bytes)
            .unwrap()
            .replace("functional-design", "domain-design");
        let reencoded = URL_SAFE_NO_PAD.encode(tampered.as_bytes());
        assert_eq!(
            verify_continue_token(KEY, &reencoded),
            Err(InvalidContinueToken)
        );
    }

    #[test]
    fn garbage_and_wrong_key_fail_closed() {
        assert_eq!(
            verify_continue_token(KEY, "not-base64url!!"),
            Err(InvalidContinueToken)
        );
        assert_eq!(verify_continue_token(KEY, ""), Err(InvalidContinueToken));
        let encoded = mint_continue_token(&[9u8; 32], &token());
        assert_eq!(
            verify_continue_token(KEY, &encoded),
            Err(InvalidContinueToken)
        );
    }

    #[test]
    fn the_strict_type_table_rejects_foreign_shapes() {
        // v ≠ 1、未知の gate 語、部索引 0、予約フラグの真値、unit 対の不整合は fail-closed。
        let mut wrong_version = to_payload_dto(&token());
        wrong_version.v = 2;
        assert_eq!(query_token(&wrong_version), Err(InvalidContinueToken));
        assert_eq!(
            query_gate(&ContinueTokenGateDto::Text("weird".to_string())),
            Err(InvalidContinueToken)
        );
        assert_eq!(
            query_gate(&ContinueTokenGateDto::Flag(true)),
            Ok(GateField::Gated)
        );
        assert_eq!(
            query_gate(&ContinueTokenGateDto::Flag(false)),
            Ok(GateField::Ungated)
        );

        let mut zero_index = to_payload_dto(&token());
        zero_index.i = 0;
        assert_eq!(query_token(&zero_index), Err(InvalidContinueToken));

        for flag in ["f", "w", "z"] {
            let mut reserved = to_payload_dto(&token());
            match flag {
                "f" => reserved.f = true,
                "w" => reserved.w = true,
                _ => reserved.z = true,
            }
            assert_eq!(query_token(&reserved), Err(InvalidContinueToken));
        }

        let mut half_unit = to_payload_dto(&token());
        half_unit.k = None;
        assert_eq!(query_token(&half_unit), Err(InvalidContinueToken));

        let mut per_unit_mismatch = to_payload_dto(&token());
        per_unit_mismatch.p = false;
        assert_eq!(query_token(&per_unit_mismatch), Err(InvalidContinueToken));

        let mut unknown_kind = to_payload_dto(&token());
        unknown_kind.k = Some("weird".to_string());
        assert_eq!(query_token(&unknown_kind), Err(InvalidContinueToken));

        let mut aware_without_digest = to_payload_dto(&token());
        aware_without_digest.h = NO_STATE_SENTINEL.to_string();
        assert_eq!(
            query_token(&aware_without_digest),
            Err(InvalidContinueToken)
        );

        let mut stateless_with_digest = to_payload_dto(&token());
        stateless_with_digest.a = false;
        assert_eq!(
            query_token(&stateless_with_digest),
            Err(InvalidContinueToken)
        );
    }

    #[test]
    fn an_unknown_key_is_rejected_by_the_strict_table() {
        let encoded = mint_continue_token(KEY, &token());
        let bytes = URL_SAFE_NO_PAD.decode(&encoded).unwrap();
        let smuggled = String::from_utf8(bytes)
            .unwrap()
            .replace("\"v\":1", "\"v\":1,\"q\":1");
        let reencoded = URL_SAFE_NO_PAD.encode(smuggled.as_bytes());
        assert_eq!(
            verify_continue_token(KEY, &reencoded),
            Err(InvalidContinueToken)
        );
    }

    #[test]
    fn the_same_key_verifies_a_token_minted_elsewhere() {
        // 封緘は鍵だけに依る — 同じ鍵バイト列を渡した別の呼出しが検証できる (プロセスを
        // またいだ継続の本体)。鍵ファイルの遅延鋳造そのものは
        // `core_infrastructure::secret_file` のテストが固定している。
        let encoded = mint_continue_token(&[7u8; 32], &token());
        assert_eq!(verify_continue_token(KEY, &encoded).unwrap(), token());
    }
}
