//! `ContinueToken` — steering 連鎖の継続ペイロード (18 キーの厳密型表 — 02 §4.4)。
//!
//! HMAC 封筒 `{p, m}` の**ペイロード側の型**である。暗号 (MAC の計算・検証・base64url) と
//! 直列化はアダプタ層の codec が持ち、ここは**型表**だけを持つ。デコード時に型表へ反する
//! ペイロードは codec が拒否する — この型に不正値は存在しない (Always Valid)。
//!
//! フィールドはすべて型付きの値で運ぶ: 同型プリミティブの隣接 (String 4 本のダイジェスト等)
//! は取り違えがコンパイルを通る温床なので、束縛は [`Bindings`]、unit は [`UnitRef`]、
//! 部索引は [`PartIndex`] で受ける。ワイヤ予約キー (`f`/`p`/`w`/`z` — force-persona /
//! per-unit / wave / settled-swarm) のうちエンジンが今日構築しない値は**フィールドを
//! 持たない** (構成不能で表す — 02 §4.1。`p` は unit の有無から導出される)。

use super::directive::GateField;
use super::stage_name::StageName;
use super::steering_binding::Bindings;
use super::steering_plan::PartIndex;
use super::token_version::TokenVersion;
use super::unit_ref::UnitRef;
use crate::workflow_view::{ScopeSlugView, StageSlugView};

/// steering 連鎖の継続ペイロード。キーは upstream の 1 文字綴り (`v`/`s`/`c`/…) に対応する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinueToken {
    version: TokenVersion,
    stage: StageSlugView,
    scope: ScopeSlugView,
    next_part_index: PartIndex,
    bindings: Bindings,
    gate: GateField,
    next_stage: Option<StageName>,
    unit: Option<UnitRef>,
    single: bool,
}

/// [`ContinueToken`] の組み立て器 — 必須 5 点を受け、残りは `with_*` で伴わせる。
/// `v` は常に現行版。
#[derive(Debug, Clone)]
pub struct ContinueTokenBuilder {
    token: ContinueToken,
}

impl ContinueTokenBuilder {
    /// 必須材料 (stage / scope / 部索引 / 束縛 / gate) を束ねる。
    #[must_use]
    pub const fn new(
        stage: StageSlugView,
        scope: ScopeSlugView,
        next_part_index: PartIndex,
        bindings: Bindings,
        gate: GateField,
    ) -> ContinueTokenBuilder {
        ContinueTokenBuilder {
            token: ContinueToken {
                version: TokenVersion::CURRENT,
                stage,
                scope,
                next_part_index,
                bindings,
                gate,
                next_stage: None,
                unit: None,
                single: false,
            },
        }
    }

    /// per-unit 反復の unit を伴う (per-unit フラグは unit の有無から導出される)。
    #[must_use]
    pub fn with_unit(mut self, unit: UnitRef) -> ContinueTokenBuilder {
        self.token.unit = Some(unit);
        self
    }

    /// 次ステージの表示名を伴う。
    #[must_use]
    pub fn with_next_stage(mut self, next_stage: StageName) -> ContinueTokenBuilder {
        self.token.next_stage = Some(next_stage);
        self
    }

    /// 単一ステージ隔離モードを伴う。
    #[must_use]
    pub const fn with_single(mut self) -> ContinueTokenBuilder {
        self.token.single = true;
        self
    }

    /// 組み上げる。
    #[must_use]
    pub fn build(self) -> ContinueToken {
        self.token
    }
}

impl ContinueToken {
    /// バージョン (常に現行版)。
    #[must_use]
    pub const fn version(&self) -> TokenVersion {
        self.version
    }

    /// 連鎖が属するステージ slug。
    #[must_use]
    pub const fn stage(&self) -> &StageSlugView {
        &self.stage
    }

    /// 解決済み scope。
    #[must_use]
    pub const fn scope(&self) -> &ScopeSlugView {
        &self.scope
    }

    /// 次に届けるパートの索引 (1 始まり。パート総数と等しければ終端 = run-stage)。
    #[must_use]
    pub const fn next_part_index(&self) -> PartIndex {
        self.next_part_index
    }

    /// 4 ダイジェスト束縛 (bundle / directive / route / state)。
    #[must_use]
    pub const fn bindings(&self) -> &Bindings {
        &self.bindings
    }

    /// per-unit 反復の unit (名前 + 種別)。
    #[must_use]
    pub const fn unit(&self) -> Option<&UnitRef> {
        self.unit.as_ref()
    }

    /// per-unit 反復か (unit の有無から導出 — 別フィールドは持たない)。
    #[must_use]
    pub const fn is_per_unit(&self) -> bool {
        self.unit.is_some()
    }

    /// ピン留めされたゲート判定。
    #[must_use]
    pub const fn gate(&self) -> GateField {
        self.gate
    }

    /// ピン留めされた次ステージ表示名。
    #[must_use]
    pub const fn next_stage(&self) -> Option<&StageName> {
        self.next_stage.as_ref()
    }

    /// 単一ステージ隔離モードか。
    #[must_use]
    pub const fn is_single(&self) -> bool {
        self.single
    }
}

#[cfg(test)]
mod tests {
    use super::super::steering_binding::{
        BundleDigest, DirectiveDigest, RouteDigest, StateBinding,
    };
    use super::super::unit_ref::{UnitKind, UnitName};
    use super::*;

    fn bindings() -> Bindings {
        Bindings::new(
            BundleDigest::new("sha256:aaaa"),
            DirectiveDigest::new("dddd"),
            RouteDigest::new("rrrr"),
            Some(StateBinding::new("hhhh")),
        )
    }

    #[test]
    fn the_builder_pins_the_delivery_context() {
        let token = ContinueTokenBuilder::new(
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
        .build();
        assert!(token.version().is_supported());
        assert_eq!(token.stage().as_str(), "functional-design");
        assert_eq!(token.scope().as_str(), "classic");
        assert_eq!(token.next_part_index().as_u32(), 2);
        assert_eq!(token.bindings(), &bindings());
        assert_eq!(
            token.unit().map(|unit| unit.name().as_str()),
            Some("u4-read-model-updater")
        );
        assert_eq!(token.unit().map(UnitRef::kind), Some(UnitKind::Library));
        assert!(token.is_per_unit());
        assert_eq!(
            token.next_stage().map(StageName::as_str),
            Some("NFR Requirements")
        );
        assert_eq!(token.gate(), GateField::Gated);
        assert!(!token.is_single());
    }

    #[test]
    fn a_stateless_binding_is_part_of_the_bindings_pair() {
        let stateless = Bindings::new(
            BundleDigest::new("b"),
            DirectiveDigest::new("d"),
            RouteDigest::new("r"),
            None,
        );
        let token = ContinueTokenBuilder::new(
            StageSlugView::parse("s").unwrap(),
            ScopeSlugView::parse("c").unwrap(),
            PartIndex::FIRST,
            stateless,
            GateField::Ungated,
        )
        .with_single()
        .build();
        assert!(token.bindings().state().is_none());
        assert!(token.is_single());
        assert!(!token.is_per_unit());
    }
}
