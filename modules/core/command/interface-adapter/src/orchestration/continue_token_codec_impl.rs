//! `ContinueTokenCodec` の実 Gateway — HMAC-SHA256 封筒 `{p, m}` の base64url (02 §4.4)。
//!
//! 鍵はマシンローカルの `.aidlc-steering-token-key` を遅延鋳造する (I8 の例外 1 —
//! "machine-local runtime state, not a project-derived value an untrusted continuation can
//! recompute")。検証は timing-safe (`Mac::verify_slice`)。デコードは厳密型表 —
//! 未知キー・型違反は serde の `deny_unknown_fields` と型で拒否する。

use std::io;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use core_command_domain::orchestration::{ContinueToken, ContinueTokenBuilder, GateField};
use core_command_use_case::orchestration::{ContinueTokenCodec, InvalidContinueToken};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

type HmacSha256 = Hmac<Sha256>;

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

/// 封筒 `{p, m}`。
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireEnvelope {
    p: WirePayload,
    m: String,
}

/// HMAC-SHA256 で封緘する codec の実 Gateway。
#[derive(Debug)]
pub struct ContinueTokenCodecImpl {
    key: Vec<u8>,
}

impl ContinueTokenCodecImpl {
    /// 鍵ファイルを開く (無ければ 32 バイトの乱数を鋳造して書く)。
    ///
    /// # Errors
    ///
    /// 鍵ファイルの読み書きの失敗 (I/O)。
    pub fn open(key_path: &Path) -> io::Result<ContinueTokenCodecImpl> {
        let key = match std::fs::read(key_path) {
            Ok(key) if !key.is_empty() => key,
            Ok(_) | Err(_) => {
                let mut minted = vec![0u8; 32];
                getrandom::fill(&mut minted).map_err(io::Error::other)?;
                std::fs::write(key_path, &minted)?;
                minted
            }
        };
        Ok(ContinueTokenCodecImpl { key })
    }

    /// ペイロードの MAC (HMAC-SHA256 は任意長鍵を受けるため失敗しないが、防御的に
    /// 失敗時は空 MAC を返す — 空 MAC のトークンは決して検証を通らない = fail-closed)。
    fn mac_of(&self, payload_bytes: &[u8]) -> Vec<u8> {
        let Ok(mut mac) = HmacSha256::new_from_slice(&self.key) else {
            return Vec::new();
        };
        mac.update(payload_bytes);
        mac.finalize().into_bytes().to_vec()
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
        v: token.version(),
        s: token.stage().to_string(),
        c: token.scope().to_string(),
        i: token.next_part_index(),
        b: token.bundle_digest().to_string(),
        d: token.directive_digest().to_string(),
        r: token.route_hash().to_string(),
        a: token.is_state_aware(),
        u: token.unit().map(str::to_string),
        k: token.unit_kind().map(str::to_string),
        f: token.is_force_persona(),
        g: wire_gate(token.gate()),
        n: token.next_stage().map(str::to_string),
        x: token.is_single(),
        p: token.is_per_unit(),
        w: token.is_wave(),
        z: token.is_swarm_settled(),
        h: token.state_hash().to_string(),
    }
}

fn domain_token(payload: &WirePayload) -> Result<ContinueToken, InvalidContinueToken> {
    if payload.v != 1 {
        return Err(InvalidContinueToken);
    }
    let mut builder = ContinueTokenBuilder::new(
        payload.s.clone(),
        payload.c.clone(),
        payload.i,
        payload.b.clone(),
        payload.d.clone(),
        payload.r.clone(),
        payload.h.clone(),
        domain_gate(&payload.g)?,
    );
    if !payload.a {
        builder = builder.without_state_binding();
    }
    if let (Some(unit), Some(kind)) = (&payload.u, &payload.k) {
        builder = builder.with_unit(unit, kind);
    }
    if let Some(next_stage) = &payload.n {
        builder = builder.with_next_stage(next_stage);
    }
    if payload.x {
        builder = builder.with_single();
    }
    Ok(builder.build())
}

impl ContinueTokenCodec for ContinueTokenCodecImpl {
    #[expect(
        clippy::disallowed_methods,
        reason = "契約 JSON ではなくマシンローカルなトークンのワイヤ形式 (BR1.7 の射程外) — canon-json を通す契約面ではない"
    )]
    fn mint(&self, token: &ContinueToken) -> String {
        let payload = wire_payload(token);
        let payload_bytes = serde_json::to_vec(&payload).unwrap_or_default();
        let mac = hex(&self.mac_of(&payload_bytes));
        let envelope = WireEnvelope { p: payload, m: mac };
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&envelope).unwrap_or_default())
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "契約 JSON ではなくマシンローカルなトークンのワイヤ形式 (BR1.7 の射程外) — canon-json を通す契約面ではない"
    )]
    fn verify(&self, encoded: &str) -> Result<ContinueToken, InvalidContinueToken> {
        let bytes = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| InvalidContinueToken)?;
        let envelope: WireEnvelope =
            serde_json::from_slice(&bytes).map_err(|_| InvalidContinueToken)?;
        let payload_bytes = serde_json::to_vec(&envelope.p).map_err(|_| InvalidContinueToken)?;
        let mut mac = HmacSha256::new_from_slice(&self.key).map_err(|_| InvalidContinueToken)?;
        mac.update(&payload_bytes);
        let expected = unhex(&envelope.m).ok_or(InvalidContinueToken)?;
        // timing-safe 比較は Mac::verify_slice が行う。
        mac.verify_slice(&expected)
            .map_err(|_| InvalidContinueToken)?;
        domain_token(&envelope.p)
    }

    fn digest(&self, material: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(material.as_bytes());
        hex(&hasher.finalize())
    }
}

/// 16 進小文字。
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// 16 進小文字の復号 (奇数長・非 16 進は `None`)。
fn unhex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(text.get(index..index + 2)?, 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_command_domain::orchestration::GateField;

    fn codec() -> (tempfile::TempDir, ContinueTokenCodecImpl) {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let codec =
            ContinueTokenCodecImpl::open(&dir.path().join(".aidlc-steering-token-key")).unwrap();
        (dir, codec)
    }

    fn token() -> ContinueToken {
        ContinueTokenBuilder::new(
            "functional-design",
            "classic",
            2,
            "sha256:bbbb",
            "dddd",
            "rrrr",
            "hhhh",
            GateField::Gated,
        )
        .with_unit("u4-read-model-updater", "library")
        .with_next_stage("NFR Requirements")
        .build()
    }

    #[test]
    fn a_minted_token_round_trips() {
        let (_dir, codec) = codec();
        let encoded = codec.mint(&token());
        assert_eq!(codec.verify(&encoded).unwrap(), token());
    }

    #[test]
    fn a_tampered_token_fails_closed() {
        let (_dir, codec) = codec();
        let encoded = codec.mint(&token());
        let bytes = URL_SAFE_NO_PAD.decode(&encoded).unwrap();
        let tampered_json = String::from_utf8(bytes)
            .unwrap()
            .replace("\"i\":2", "\"i\":3");
        let tampered = URL_SAFE_NO_PAD.encode(tampered_json.as_bytes());
        assert_eq!(codec.verify(&tampered), Err(InvalidContinueToken));
    }

    #[test]
    fn a_foreign_key_fails_closed() {
        let (_dir, minting) = codec();
        let (_dir2, other) = codec();
        let encoded = minting.mint(&token());
        assert_eq!(other.verify(&encoded), Err(InvalidContinueToken));
    }

    #[test]
    fn garbage_fails_closed() {
        let (_dir, codec) = codec();
        assert_eq!(codec.verify("garbage"), Err(InvalidContinueToken));
        assert_eq!(codec.verify(""), Err(InvalidContinueToken));
    }

    #[test]
    fn the_key_is_minted_once_and_reused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".aidlc-steering-token-key");
        let first = ContinueTokenCodecImpl::open(&path).unwrap();
        let encoded = first.mint(&token());
        let second = ContinueTokenCodecImpl::open(&path).unwrap();
        assert_eq!(second.verify(&encoded).unwrap(), token());
    }

    #[test]
    fn the_digest_is_a_lowercase_sha256() {
        let (_dir, codec) = codec();
        assert_eq!(
            codec.digest("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn an_unresolved_gate_round_trips() {
        let (_dir, codec) = codec();
        let token =
            ContinueTokenBuilder::new("s", "c", 1, "b", "d", "r", "h", GateField::Unresolved)
                .without_state_binding()
                .build();
        let encoded = codec.mint(&token);
        assert_eq!(
            codec.verify(&encoded).unwrap().gate(),
            GateField::Unresolved
        );
    }

    #[test]
    fn the_strict_type_table_rejects_foreign_shapes() {
        // v ≠ 1、未知の gate 語はいずれも fail-closed。
        let mut wrong_version = wire_payload(&token());
        wrong_version.v = 2;
        assert_eq!(domain_token(&wrong_version), Err(InvalidContinueToken));
        assert_eq!(
            domain_gate(&WireGate::Text("weird".to_string())),
            Err(InvalidContinueToken)
        );
        assert_eq!(domain_gate(&WireGate::Flag(true)), Ok(GateField::Gated));
        assert_eq!(domain_gate(&WireGate::Flag(false)), Ok(GateField::Ungated));
    }

    #[test]
    fn malformed_hex_is_rejected() {
        assert_eq!(unhex("abc"), None, "奇数長");
        assert_eq!(unhex("zz"), None, "非 16 進");
        assert_eq!(unhex("0a1f"), Some(vec![0x0a, 0x1f]));
    }
}
