//! 鍵付き封緘の純粋コーデック — バイト列に HMAC-SHA256 の MAC を添えて base64url で運ぶ。
//!
//! **I/O を持たない** (オーナー裁定 2026-08-30「コーデックとは I/O は本来依存しない」)。鍵は
//! 引数で受け取るだけで、鍵の取得・保管はここの責務ではない (出所の設計は U7 の合成ルートで
//! 裁定する — コマンド側は Repository 以外の I/O 責務を持てない)。
//!
//! 相手方システムの契約も知らない — 封緘する中身は `Serialize` な何かであり、キーの綴りも
//! 語彙も呼出側が決める (`coding-rules/infrastructure-layer.md`)。ここが知るのは
//! 「payload の直列化バイトに MAC を付け、`{p, m}` の封筒に入れて base64url にする」という
//! 機構だけである。検証は timing-safe (`Mac::verify_slice`)。

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

type HmacSha256 = Hmac<Sha256>;

/// 封筒 — 封緘した payload (`p`) と、その直列化バイトの MAC (`m`、小文字 16 進)。
#[derive(Serialize, Deserialize)]
struct Envelope<T> {
    p: T,
    m: String,
}

/// payload を封緘して base64url の 1 本の文字列にする。
///
/// MAC の対象は payload の直列化バイトである。したがって `T` の直列化が決定的であること
/// (構造体のフィールド順が固定であること) が前提になる — 呼出側の DTO がそれを満たす。
#[must_use]
#[expect(
    clippy::disallowed_methods,
    reason = "契約 JSON ではなくマシンローカルな封筒のワイヤ形式 (BR1.7 の射程外) — canon-json を通す契約面ではない"
)]
pub fn seal<T: Serialize>(key: &[u8], payload: &T) -> String {
    let payload_bytes = serde_json::to_vec(payload).unwrap_or_default();
    let envelope = Envelope {
        p: payload,
        m: hex(&mac_of(key, &payload_bytes)),
    };
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(&envelope).unwrap_or_default())
}

/// 封緘を解いて payload へ戻す。復号不能・MAC 不一致は `None` (fail-closed)。
///
/// 区別を返さないのは、呼出側にとって「無効」以外に打つ手が無いからである。
#[must_use]
#[expect(
    clippy::disallowed_methods,
    reason = "契約 JSON ではなくマシンローカルな封筒のワイヤ形式 (BR1.7 の射程外) — canon-json を通す契約面ではない"
)]
pub fn unseal<T: DeserializeOwned + Serialize>(key: &[u8], encoded: &str) -> Option<T> {
    let bytes = URL_SAFE_NO_PAD.decode(encoded).ok()?;
    let envelope: Envelope<T> = serde_json::from_slice(&bytes).ok()?;
    let payload_bytes = serde_json::to_vec(&envelope.p).ok()?;
    let mut mac = HmacSha256::new_from_slice(key).ok()?;
    mac.update(&payload_bytes);
    // timing-safe 比較は `Mac::verify_slice` が行う。
    mac.verify_slice(&unhex(&envelope.m)?).ok()?;
    Some(envelope.p)
}

/// 名前付き素材の直列化バイトの sha256 (64 桁小文字 16 進)。
///
/// `Debug` 表現への依存は derive 変更で黙ってダイジェストが変わる時限爆弾であり、区切り文字
/// 連結は区切り文字注入を許すので、素材は**名前付き構造体の直列化**で与える
/// (オーナー裁定 2026-08-30)。
#[must_use]
#[expect(
    clippy::disallowed_methods,
    reason = "契約 JSON ではなくダイジェスト素材のエスケープ付き直列化 (BR1.7 の射程外) — canon-json を通す契約面ではない"
)]
pub fn digest_hex<T: Serialize>(material: &T) -> String {
    let bytes = serde_json::to_vec(material).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    hex(&hasher.finalize())
}

/// HMAC-SHA256 (任意長鍵を受けるため失敗しないが、防御的に失敗時は空 MAC を返す —
/// 空 MAC の封筒は決して検証を通らない = fail-closed)。
fn mac_of(key: &[u8], payload_bytes: &[u8]) -> Vec<u8> {
    let Ok(mut mac) = HmacSha256::new_from_slice(key) else {
        return Vec::new();
    };
    mac.update(payload_bytes);
    mac.finalize().into_bytes().to_vec()
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
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Payload {
        v: u32,
        s: String,
    }

    fn payload() -> Payload {
        Payload {
            v: 1,
            s: "functional-design".to_string(),
        }
    }

    const KEY: &[u8] = &[7u8; 32];

    #[test]
    fn a_sealed_payload_round_trips_under_the_same_key() {
        let sealed = seal(KEY, &payload());
        assert_eq!(unseal::<Payload>(KEY, &sealed), Some(payload()));
    }

    #[test]
    fn sealing_is_deterministic() {
        assert_eq!(seal(KEY, &payload()), seal(KEY, &payload()));
    }

    #[test]
    fn another_key_cannot_unseal() {
        let sealed = seal(KEY, &payload());
        assert_eq!(unseal::<Payload>(&[9u8; 32], &sealed), None);
    }

    #[test]
    fn a_tampered_payload_fails_the_mac() {
        use base64::Engine as _;
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let sealed = seal(KEY, &payload());
        let text = String::from_utf8(URL_SAFE_NO_PAD.decode(&sealed).unwrap()).unwrap();
        let tampered = text.replace("functional-design", "domain-design");
        let reencoded = URL_SAFE_NO_PAD.encode(tampered.as_bytes());
        assert_eq!(unseal::<Payload>(KEY, &reencoded), None);
    }

    #[test]
    fn garbage_fails_closed() {
        assert_eq!(unseal::<Payload>(KEY, "not-base64url!!"), None);
        assert_eq!(unseal::<Payload>(KEY, ""), None);
    }

    #[test]
    fn a_digest_is_deterministic_and_material_specific() {
        assert_eq!(digest_hex(&payload()), digest_hex(&payload()));
        let other = Payload {
            v: 1,
            s: "domain-design".to_string(),
        };
        assert_ne!(digest_hex(&payload()), digest_hex(&other));
        assert_eq!(digest_hex(&payload()).len(), 64, "sha256 の 64 桁 hex");
    }
}
