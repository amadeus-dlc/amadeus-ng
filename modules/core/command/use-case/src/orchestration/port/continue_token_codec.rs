//! `ContinueTokenCodec` — `continue_token` の mint / verify と型付きダイジェストのポート。
//!
//! HMAC-SHA256・base64url・直列化 (エスケープ付き — 区切り文字注入を許さない) という
//! **機構**はアダプタ層の実装が持ち、ユースケースは型付きペイロード ([`ContinueToken`]) と
//! 型付きダイジェストの授受だけを行う。ダイジェストの**対象**は名前付きの VO
//! ([`SteeringPlan`] / [`RunStageDirective`] / [`StageRoute`] / [`StatePosition`]) で渡し、
//! 素材文字列はポート面に現れない (オーナー裁定 2026-08-30)。検証は timing-safe (実装の
//! 責務)。鍵 `.aidlc-steering-token-key` はマシンローカルで、実装が遅延鋳造する
//! (I8 の例外 1 — 02 §3.1)。

use core_command_domain::orchestration::{
    BundleDigest, ContinueToken, DirectiveDigest, RouteDigest, RunStageDirective, StateBinding,
    SteeringPlan,
};
use core_command_domain::workflow_definition::StageRoute;

use super::state_position::StatePosition;

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

/// トークンの封緘・検証・型付きダイジェスト計算。
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

    /// ルール束のダイジェスト (`b`)。
    fn bundle_digest(&self, plan: &SteeringPlan) -> BundleDigest;

    /// 届けようとしている run-stage のダイジェスト (`d`)。
    fn directive_digest(&self, run_stage: &RunStageDirective) -> DirectiveDigest;

    /// グラフノードと scope メンバーシップの route ダイジェスト (`r`)。
    fn route_digest(&self, route: &StageRoute) -> RouteDigest;

    /// state 束縛のダイジェスト (`h`)。
    fn state_binding(&self, position: &StatePosition) -> StateBinding;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_invalid_token_error_has_a_stable_face() {
        assert_eq!(InvalidContinueToken.to_string(), "invalid continue token");
        let boxed: Box<dyn std::error::Error> = Box::new(InvalidContinueToken);
        assert_eq!(boxed.to_string(), "invalid continue token");
    }
}
