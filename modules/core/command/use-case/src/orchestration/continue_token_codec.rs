//! `ContinueTokenCodec` — `continue_token` の mint / verify とダイジェストのポート。
//!
//! HMAC-SHA256・base64url・正準 JSON ダイジェストという**機構**はアダプタ層の実装が持ち、
//! ユースケースは型付きペイロード ([`ContinueToken`]) の授受だけを行う。検証は
//! timing-safe (実装の責務)。鍵 `.aidlc-steering-token-key` はマシンローカルで、実装が
//! 遅延鋳造する (I8 の例外 1 — 02 §3.1)。

use core_command_domain::orchestration::ContinueToken;

/// 無効なトークン (材料なし — 契約は「無効」だけを約束する。fail-closed の逐語文言は
/// 呼出側の wording が組む)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidContinueToken;

impl std::fmt::Display for InvalidContinueToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid continue token")
    }
}

impl std::error::Error for InvalidContinueToken {}

/// トークンの封緘・検証・ダイジェスト計算。
pub trait ContinueTokenCodec {
    /// ペイロードを封緘して encode 済みトークンを返す。
    fn mint(&self, token: &ContinueToken) -> String;

    /// encode 済みトークンを検証してペイロードへ戻す。
    ///
    /// # Errors
    ///
    /// デコード不能・MAC 不一致・厳密型表への違反はすべて `InvalidContinueToken`
    /// (fail-closed — 区別は診断に不要で、契約は fresh `next` からのやり直しだけを指示する)。
    fn verify(&self, encoded: &str) -> Result<ContinueToken, InvalidContinueToken>;

    /// 素材文字列のダイジェスト (sha256 hex — bundle / directive / route / state 束縛用)。
    fn digest(&self, material: &str) -> String;
}
