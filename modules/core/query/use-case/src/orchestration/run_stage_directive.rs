//! `RunStageDirective` — `run-stage` (ステージ本体の実行指示、公開言語 B14)。
//!
//! フィールドは private + アクセサ (`coding-rules/field-visibility.md`)。組み立ては
//! [`RunStageDirectiveBuilder`] が唯一の入口で、構造体リテラルはその `build()` にしか
//! 現れない (`coding-rules/factory-naming.md`)。ビルダーは本型の表現に仕える従属物なので
//! 子モジュールに置く — 兄弟ファイルからは private フィールドが見えず、全フィールドを
//! 位置引数で受け渡す基本コンストラクタを立てるしかなくなるためである。

use super::continue_token::ContinueToken;
use super::gate_field::GateField;
use super::unit_ref::UnitRef;
use crate::orchestration::{PhaseView, ReviewClassView, StageModeView, StageSlugView};

mod run_stage_directive_builder;

pub use run_stage_directive_builder::RunStageDirectiveBuilder;

/// `run-stage` — ステージ本体の実行指示。
///
/// フィールドは private + アクセサ (`coding-rules/field-visibility.md`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStageDirective {
    stage: StageSlugView,
    phase: PhaseView,
    lead_agent: String,
    support_agents: Vec<String>,
    mode: StageModeView,
    gate: GateField,
    stage_file: String,
    memory_path: String,
    inline_context_paths: Vec<String>,
    consumes: Vec<String>,
    produces: Vec<String>,
    sensors_applicable: Vec<String>,
    next_stage: Option<String>,
    reviewer: Option<String>,
    review_class: Option<ReviewClassView>,
    reviewer_max_iterations: Option<u32>,
    protocol_modules: Vec<String>,
    narration: Option<String>,
    single: bool,
    unit: Option<UnitRef>,
    rules_in_context: Vec<String>,
}

impl RunStageDirective {
    /// 走らせるステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlugView {
        &self.stage
    }

    /// フェーズ。
    #[must_use]
    pub const fn phase(&self) -> PhaseView {
        self.phase
    }

    /// リードエージェント。
    #[must_use]
    pub fn lead_agent(&self) -> &str {
        &self.lead_agent
    }

    /// 支援エージェント列。
    #[must_use]
    pub fn support_agents(&self) -> &[String] {
        &self.support_agents
    }

    /// 通信トポロジ。
    #[must_use]
    pub const fn mode(&self) -> StageModeView {
        self.mode
    }

    /// 承認ゲートの有無 (`unresolved` は walking-skeleton 判定待ち)。
    #[must_use]
    pub const fn gate(&self) -> GateField {
        self.gate
    }

    /// ステージ本体ファイルのパス。
    #[must_use]
    pub fn stage_file(&self) -> &str {
        &self.stage_file
    }

    /// ステージ日誌のパス。
    #[must_use]
    pub fn memory_path(&self) -> &str {
        &self.memory_path
    }

    /// 読み込み必須のコンテキストパス列。
    #[must_use]
    pub fn inline_context_paths(&self) -> &[String] {
        &self.inline_context_paths
    }

    /// 上流成果物のパス列。
    #[must_use]
    pub fn consumes(&self) -> &[String] {
        &self.consumes
    }

    /// 産出物のパス列。
    #[must_use]
    pub fn produces(&self) -> &[String] {
        &self.produces
    }

    /// 発火センサー列。
    #[must_use]
    pub fn sensors_applicable(&self) -> &[String] {
        &self.sensors_applicable
    }

    /// 次ステージの表示名。
    #[must_use]
    pub fn next_stage(&self) -> Option<&str> {
        self.next_stage.as_deref()
    }

    /// レビュアー (実効)。
    #[must_use]
    pub fn reviewer(&self) -> Option<&str> {
        self.reviewer.as_deref()
    }

    /// レビュークラス。
    #[must_use]
    pub const fn review_class(&self) -> Option<ReviewClassView> {
        self.review_class
    }

    /// レビュアーの最大反復。
    #[must_use]
    pub const fn reviewer_max_iterations(&self) -> Option<u32> {
        self.reviewer_max_iterations
    }

    /// プロトコルモジュールのヒント列。
    #[must_use]
    pub fn protocol_modules(&self) -> &[String] {
        &self.protocol_modules
    }

    /// ユーザ向けのひとこと。
    #[must_use]
    pub fn narration(&self) -> Option<&str> {
        self.narration.as_deref()
    }

    /// 単一ステージ隔離モードか。
    #[must_use]
    pub const fn is_single(&self) -> bool {
        self.single
    }

    /// per-unit 反復の unit。
    #[must_use]
    pub const fn unit(&self) -> Option<&UnitRef> {
        self.unit.as_ref()
    }

    /// 配信済みルール束のパス台帳。
    #[must_use]
    pub fn rules_in_context(&self) -> &[String] {
        &self.rules_in_context
    }

    /// パス台帳 (`rules_in_context`) だけを載せ替えた複製。
    ///
    /// 不変オブジェクトの部分更新は本型が持つ — 呼出側の全フィールド手動移送は
    /// フィールド追加時に黙って欠落するので禁止 (オーナー裁定 2026-08-30)。
    #[must_use]
    pub fn with_rules_in_context(&self, paths: Vec<String>) -> RunStageDirective {
        let mut copy = self.clone();
        copy.rules_in_context = paths;
        copy
    }

    /// トークンのピン (`gate` / `next_stage` / `unit` / `single`) を再適用した複製
    /// (再構築原則 `:5996-6037` — キャッシュを信用せず、ピンだけを引き継ぐ)。
    #[must_use]
    pub fn with_pins(&self, token: &ContinueToken) -> RunStageDirective {
        let mut copy = self.clone();
        copy.gate = token.gate();
        copy.next_stage = token.next_stage().map(|name| name.as_str().to_string());
        copy.unit = token.unit().cloned();
        copy.single = token.is_single();
        copy
    }
}

#[cfg(test)]
mod tests {
    use super::super::bindings::Bindings;
    use super::super::bundle_digest::BundleDigest;
    use super::super::continue_token::ContinueTokenBuilder;
    use super::super::directive_digest::DirectiveDigest;
    use super::super::part_index::PartIndex;
    use super::super::route_digest::RouteDigest;
    use super::super::stage_name::StageName;
    use super::super::unit_kind::UnitKind;
    use super::super::unit_name::UnitName;
    use super::*;
    use crate::orchestration::ScopeSlugView;

    fn slug() -> StageSlugView {
        StageSlugView::parse("requirements-analysis").unwrap()
    }

    fn token(gate: GateField) -> ContinueTokenBuilder {
        ContinueTokenBuilder::new(
            slug(),
            ScopeSlugView::parse("classic").unwrap(),
            PartIndex::FIRST,
            Bindings::new(
                BundleDigest::new("sha256:bbbb"),
                DirectiveDigest::new("d"),
                RouteDigest::new("r"),
                None,
            ),
            gate,
        )
    }

    #[test]
    fn the_builder_carries_every_optional_face() {
        let directive = RunStageDirectiveBuilder::new(
            slug(),
            PhaseView::Inception,
            "aidlc-product-agent",
            StageModeView::Inline,
            GateField::Unresolved,
            "stage.md",
            "memory.md",
        )
        .with_support_agents(vec!["aidlc-design-agent".to_string()])
        .with_inline_context_paths(vec!["agents/aidlc-product-agent.md".to_string()])
        .with_consumes(vec!["a.md".to_string()])
        .with_produces(vec!["b.md".to_string()])
        .with_sensors(vec!["traceability".to_string()])
        .with_next_stage("User Stories")
        .with_reviewer("aidlc-product-lead-agent", ReviewClassView::Advisory, 1)
        .with_protocol_modules(vec!["reviewer".to_string()])
        .with_narration("Now working on requirements.")
        .with_single()
        .build();
        assert_eq!(directive.stage().as_str(), "requirements-analysis");
        assert_eq!(directive.phase(), PhaseView::Inception);
        assert_eq!(directive.lead_agent(), "aidlc-product-agent");
        assert_eq!(directive.mode(), StageModeView::Inline);
        assert_eq!(directive.stage_file(), "stage.md");
        assert_eq!(directive.memory_path(), "memory.md");
        assert_eq!(directive.support_agents(), ["aidlc-design-agent"]);
        assert_eq!(directive.inline_context_paths().len(), 1);
        assert_eq!(directive.consumes(), ["a.md"]);
        assert_eq!(directive.produces(), ["b.md"]);
        assert_eq!(directive.sensors_applicable(), ["traceability"]);
        assert_eq!(directive.next_stage(), Some("User Stories"));
        assert_eq!(directive.reviewer(), Some("aidlc-product-lead-agent"));
        assert_eq!(directive.review_class(), Some(ReviewClassView::Advisory));
        assert_eq!(directive.reviewer_max_iterations(), Some(1));
        assert_eq!(directive.protocol_modules(), ["reviewer"]);
        assert_eq!(directive.narration(), Some("Now working on requirements."));
        assert!(directive.is_single());
        assert_eq!(directive.gate(), GateField::Unresolved);
    }

    #[test]
    fn a_run_stage_carries_its_unit_and_rule_ledger() {
        let unit = UnitRef::new(
            UnitName::parse("u6-next-continue-use-case").unwrap(),
            UnitKind::Library,
        );
        let directive = RunStageDirectiveBuilder::new(
            slug(),
            PhaseView::Construction,
            "aidlc-developer-agent",
            StageModeView::Inline,
            GateField::Gated,
            "stage.md",
            "memory.md",
        )
        .with_unit(unit.clone())
        .with_rules_in_context(vec!["memory/org.md".to_string()])
        .build();
        assert_eq!(directive.unit(), Some(&unit));
        assert_eq!(directive.rules_in_context(), ["memory/org.md"]);
    }

    #[test]
    fn the_ledger_swap_and_the_pin_reapplication_are_owned_by_the_directive() {
        let directive = RunStageDirectiveBuilder::new(
            slug(),
            PhaseView::Construction,
            "aidlc-developer-agent",
            StageModeView::Inline,
            GateField::Gated,
            "stage.md",
            "memory.md",
        )
        .with_narration("keep me")
        .build();
        let swapped = directive.with_rules_in_context(vec!["memory/org.md".to_string()]);
        assert_eq!(swapped.rules_in_context(), ["memory/org.md"]);
        assert_eq!(
            swapped.narration(),
            Some("keep me"),
            "他フィールドは保存される"
        );

        let unit = UnitRef::new(
            UnitName::parse("u6-next-continue-use-case").unwrap(),
            UnitKind::Library,
        );
        let pinned = token(GateField::Unresolved)
            .with_unit(unit.clone())
            .with_next_stage(StageName::parse("User Stories").unwrap())
            .with_single()
            .build();
        let reapplied = directive.with_pins(&pinned);
        assert_eq!(reapplied.gate(), GateField::Unresolved);
        assert_eq!(reapplied.next_stage(), Some("User Stories"));
        assert_eq!(reapplied.unit(), Some(&unit));
        assert!(reapplied.is_single());
        assert_eq!(
            reapplied.narration(),
            Some("keep me"),
            "ピン以外は保存される"
        );
    }
}
