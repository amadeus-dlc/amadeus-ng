//! `ContinueToken` — steering 連鎖の継続ペイロード (18 キーの厳密型表 — 02 §4.4)。
//!
//! HMAC 封筒 `{p, m}` の**ペイロード側の型**である。暗号 (MAC の計算・検証・base64url) と
//! 直列化はアダプタ層の codec が持ち、ドメインは**型表**だけを持つ
//! (`coding-rules/domain-persistence-neutrality.md`)。デコード時に型表へ反するペイロードは
//! codec が拒否する — この型に不正値は存在しない (Always Valid)。

use super::directive::GateField;

/// steering 連鎖の継続ペイロード。キーは upstream の 1 文字綴り (`v`/`s`/`c`/…) に対応する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinueToken {
    version: u32,
    stage: String,
    scope: String,
    next_part_index: u32,
    bundle_digest: String,
    directive_digest: String,
    route_hash: String,
    state_aware: bool,
    unit: Option<String>,
    unit_kind: Option<String>,
    force_persona: bool,
    gate: GateField,
    next_stage: Option<String>,
    single: bool,
    per_unit: bool,
    wave: bool,
    swarm_settled: bool,
    state_hash: String,
}

/// [`ContinueToken`] の組み立て器 — 必須 8 点を受け、残りは `with_*` で伴わせる。
#[derive(Debug, Clone)]
pub struct ContinueTokenBuilder {
    token: ContinueToken,
}

impl ContinueTokenBuilder {
    /// 必須材料 (stage / scope / 次パート索引 / 4 ダイジェスト束縛 / gate) を束ねる。
    /// `v` は常に 1。
    #[must_use]
    #[allow(
        clippy::too_many_arguments,
        reason = "トークンの必須材料 (4 ダイジェスト束縛 + 文脈 4 点) そのもの — 束ねる中間型は複製にしかならない"
    )]
    pub fn new(
        stage: impl Into<String>,
        scope: impl Into<String>,
        next_part_index: u32,
        bundle_digest: impl Into<String>,
        directive_digest: impl Into<String>,
        route_hash: impl Into<String>,
        state_hash: impl Into<String>,
        gate: GateField,
    ) -> ContinueTokenBuilder {
        ContinueTokenBuilder {
            token: ContinueToken {
                version: 1,
                stage: stage.into(),
                scope: scope.into(),
                next_part_index,
                bundle_digest: bundle_digest.into(),
                directive_digest: directive_digest.into(),
                route_hash: route_hash.into(),
                state_aware: true,
                unit: None,
                unit_kind: None,
                force_persona: false,
                gate,
                next_stage: None,
                single: false,
                per_unit: false,
                wave: false,
                swarm_settled: false,
                state_hash: state_hash.into(),
            },
        }
    }

    /// per-unit 反復の unit と種別を伴う。
    #[must_use]
    pub fn with_unit(
        mut self,
        unit: impl Into<String>,
        kind: impl Into<String>,
    ) -> ContinueTokenBuilder {
        self.token.unit = Some(unit.into());
        self.token.unit_kind = Some(kind.into());
        self.token.per_unit = true;
        self
    }

    /// 次ステージの表示名を伴う。
    #[must_use]
    pub fn with_next_stage(mut self, next_stage: impl Into<String>) -> ContinueTokenBuilder {
        self.token.next_stage = Some(next_stage.into());
        self
    }

    /// 単一ステージ隔離モードを伴う。
    #[must_use]
    pub const fn with_single(mut self) -> ContinueTokenBuilder {
        self.token.single = true;
        self
    }

    /// state 束縛なし (state なしのジャンプ等) にする。
    #[must_use]
    pub const fn without_state_binding(mut self) -> ContinueTokenBuilder {
        self.token.state_aware = false;
        self
    }

    /// 組み上げる。
    #[must_use]
    pub fn build(self) -> ContinueToken {
        self.token
    }
}

impl ContinueToken {
    /// バージョン (常に 1)。
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// 連鎖が属するステージ slug。
    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }

    /// 解決済み scope。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// 次に届けるパートの索引 (1 始まり。パート総数と等しければ終端 = run-stage)。
    #[must_use]
    pub const fn next_part_index(&self) -> u32 {
        self.next_part_index
    }

    /// ルール束のダイジェスト。
    #[must_use]
    pub fn bundle_digest(&self) -> &str {
        &self.bundle_digest
    }

    /// 届けようとしている run-stage のダイジェスト。
    #[must_use]
    pub fn directive_digest(&self) -> &str {
        &self.directive_digest
    }

    /// グラフノードと scope メンバーシップのルートハッシュ。
    #[must_use]
    pub fn route_hash(&self) -> &str {
        &self.route_hash
    }

    /// state 束縛の有無。
    #[must_use]
    pub const fn is_state_aware(&self) -> bool {
        self.state_aware
    }

    /// per-unit 反復の unit。
    #[must_use]
    pub fn unit(&self) -> Option<&str> {
        self.unit.as_deref()
    }

    /// unit の種別。
    #[must_use]
    pub fn unit_kind(&self) -> Option<&str> {
        self.unit_kind.as_deref()
    }

    /// ペルソナ強制フラグ。
    #[must_use]
    pub const fn is_force_persona(&self) -> bool {
        self.force_persona
    }

    /// ピン留めされたゲート判定。
    #[must_use]
    pub const fn gate(&self) -> GateField {
        self.gate
    }

    /// ピン留めされた次ステージ表示名。
    #[must_use]
    pub fn next_stage(&self) -> Option<&str> {
        self.next_stage.as_deref()
    }

    /// 単一ステージ隔離モードか。
    #[must_use]
    pub const fn is_single(&self) -> bool {
        self.single
    }

    /// per-unit 反復か。
    #[must_use]
    pub const fn is_per_unit(&self) -> bool {
        self.per_unit
    }

    /// wave 並列面か。
    #[must_use]
    pub const fn is_wave(&self) -> bool {
        self.wave
    }

    /// settled swarm 再入か。
    #[must_use]
    pub const fn is_swarm_settled(&self) -> bool {
        self.swarm_settled
    }

    /// state ダイジェスト (`state_aware` のときだけ照合する)。
    #[must_use]
    pub fn state_hash(&self) -> &str {
        &self.state_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_builder_pins_the_delivery_context() {
        let token = ContinueTokenBuilder::new(
            "functional-design",
            "classic",
            2,
            "sha256:aaaa",
            "dddd",
            "rrrr",
            "hhhh",
            GateField::Gated,
        )
        .with_unit("u4-read-model-updater", "library")
        .with_next_stage("NFR Requirements")
        .build();
        assert_eq!(token.version(), 1);
        assert_eq!(token.stage(), "functional-design");
        assert_eq!(token.scope(), "classic");
        assert_eq!(token.next_part_index(), 2);
        assert_eq!(token.bundle_digest(), "sha256:aaaa");
        assert_eq!(token.directive_digest(), "dddd");
        assert_eq!(token.route_hash(), "rrrr");
        assert_eq!(token.state_hash(), "hhhh");
        assert!(token.is_state_aware());
        assert_eq!(token.unit(), Some("u4-read-model-updater"));
        assert_eq!(token.unit_kind(), Some("library"));
        assert!(token.is_per_unit());
        assert_eq!(token.next_stage(), Some("NFR Requirements"));
        assert_eq!(token.gate(), GateField::Gated);
        assert!(!token.is_single());
        assert!(!token.is_wave());
        assert!(!token.is_swarm_settled());
        assert!(!token.is_force_persona());
    }

    #[test]
    fn the_state_binding_can_be_dropped() {
        let token = ContinueTokenBuilder::new("s", "c", 1, "b", "d", "r", "h", GateField::Ungated)
            .without_state_binding()
            .with_single()
            .build();
        assert!(!token.is_state_aware());
        assert!(token.is_single());
    }
}
