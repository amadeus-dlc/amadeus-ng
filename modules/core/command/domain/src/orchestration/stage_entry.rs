//! `StageEntry` — `Started` に載る解決済みの 1 ステージ分の計画 (entities.md StageEntry)。

use super::stage_display::StageDisplay;
use crate::workflow_definition::{PhaseId, PlanAction, StageSlug};

/// 定義から解決済みの 1 ステージ分の計画。
///
/// `Started` がこの列を持つことでリプレイは `WorkflowDefinition` を要さない (BR2.2)。
/// ゲート判定はこの型が所有する — 索引ではなく `phase` から決まる (BR1.3、Tell-Don't-Ask)。
///
/// **投影も定義を要さない** — 監査行と状態ファイルに現れる表示属性 3 値は [`StageDisplay`] が
/// 運ぶ (オーナー裁定 2026-08-29)。投影がジャーナルだけで描けることが、クラッシュ再構成で
/// 当時と同一のバイトを得る条件である (NFR3)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageEntry {
    slug: StageSlug,
    phase: PhaseId,
    plan_action: PlanAction,
    conditional: bool,
    greenfield_adjusted: bool,
    display: StageDisplay,
}

impl StageEntry {
    /// 解決済みの 5 成分を束ねる (**この型の唯一の構造体リテラル** — 完全コンストラクタ)。
    ///
    /// `plan_action` はグリッドの 3 値 `Option<PlanAction>` を `None → SKIP` で畳んだ 2 値、
    /// `conditional` は同じ文書順の `StageNode::execution() == CONDITIONAL` (BR2.2)、
    /// `display` は投影がリードモデルを描くのに要る表示属性 3 値 ([`StageDisplay`])。
    /// greenfield 調整は組んだ後に [`StageEntry::adjusted_for_greenfield`] で畳む — 調整前の
    /// エントリはグリッドの値そのものである。
    #[must_use]
    pub const fn new(
        slug: StageSlug,
        phase: PhaseId,
        plan_action: PlanAction,
        conditional: bool,
        display: StageDisplay,
    ) -> StageEntry {
        StageEntry {
            slug,
            phase,
            plan_action,
            conditional,
            greenfield_adjusted: false,
            display,
        }
    }

    /// greenfield のワークスペース向けに SKIP へ畳んだ同じステージ (upstream `intent-create` の
    /// greenfield 調整 — `aidlc-utility.ts:5784-5793` @a277af21)。
    ///
    /// グリッドが EXECUTE と言っていた事実は失われるが、畳んだ事実は残る — 状態ファイルの
    /// `Stages to Skip` はこのステージだけを `<番号> (<slug> — greenfield)` と注釈付きで書く
    /// (素のグリッド SKIP と描き分けが要る)。`mut self -> Self` の変換であって setter ではない
    /// (`coding-rules/factory-naming.md`)。
    #[must_use]
    pub const fn adjusted_for_greenfield(mut self) -> StageEntry {
        self.plan_action = PlanAction::Skip;
        self.greenfield_adjusted = true;
        self
    }

    /// ステージ slug (イベントのステージ参照はすべてこの値)。
    #[must_use]
    pub const fn slug(&self) -> &StageSlug {
        &self.slug
    }

    /// このステージのフェーズ。
    #[must_use]
    pub const fn phase(&self) -> PhaseId {
        self.phase
    }

    /// 静的グリッド由来の計画 (`plan`)。recompose オーバレイはここには載らない。
    #[must_use]
    pub const fn plan_action(&self) -> PlanAction {
        self.plan_action
    }

    /// ステージ著者側の適用可否が CONDITIONAL か。
    #[must_use]
    pub const fn is_conditional(&self) -> bool {
        self.conditional
    }

    /// グリッドは EXECUTE だったが greenfield 調整で SKIP に畳まれたか
    /// (`Stages to Skip` の ` — greenfield` 注釈の材料)。
    #[must_use]
    pub const fn is_greenfield_adjusted(&self) -> bool {
        self.greenfield_adjusted
    }

    /// 投影がリードモデルを描くのに要る表示属性 (ステージ番号・表題・担当エージェント)。
    #[must_use]
    pub const fn display(&self) -> &StageDisplay {
        &self.display
    }

    /// ゲート付きか — `phase != initialization` (BR1.3)。索引 0 の特別扱いはしない。
    #[must_use]
    pub fn is_gated(&self) -> bool {
        self.phase != PhaseId::Initialization
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_definition::{PhaseId, PlanAction, StageNumber, StageSlug};

    fn display() -> StageDisplay {
        StageDisplay::new(
            StageNumber::parse("0.1").unwrap(),
            "State Init",
            "orchestrator",
        )
        .unwrap()
    }

    fn entry(phase: PhaseId, action: PlanAction, conditional: bool) -> StageEntry {
        StageEntry::new(
            StageSlug::parse("state-init").unwrap(),
            phase,
            action,
            conditional,
            display(),
        )
    }

    #[test]
    fn the_entry_carries_the_resolved_plan_of_one_stage() {
        let e = entry(PhaseId::Inception, PlanAction::Execute, true);
        assert_eq!(e.slug().as_str(), "state-init");
        assert_eq!(e.phase(), PhaseId::Inception);
        assert_eq!(e.plan_action(), PlanAction::Execute);
        assert!(e.is_conditional());
        assert_eq!(e.display().number().as_str(), "0.1");
        assert_eq!(e.display().name(), "State Init");
        assert_eq!(e.display().lead_agent(), "orchestrator");
    }

    #[test]
    fn an_initialization_stage_is_not_gated() {
        let e = entry(PhaseId::Initialization, PlanAction::Execute, false);
        assert!(!e.is_gated());
    }

    #[test]
    fn every_other_phase_is_gated() {
        for phase in [
            PhaseId::Ideation,
            PhaseId::Inception,
            PhaseId::Construction,
            PhaseId::Operation,
        ] {
            assert!(
                entry(phase, PlanAction::Execute, false).is_gated(),
                "{phase:?}"
            );
        }
    }

    #[test]
    fn an_unconditional_entry_reports_it() {
        assert!(!entry(PhaseId::Inception, PlanAction::Skip, false).is_conditional());
    }

    #[test]
    fn entries_compare_by_value() {
        let a = entry(PhaseId::Inception, PlanAction::Execute, false);
        let b = entry(PhaseId::Inception, PlanAction::Execute, false);
        let c = entry(PhaseId::Inception, PlanAction::Skip, false);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    /// greenfield 調整は SKIP に畳み、畳んだ事実を残す — 素のグリッド SKIP とは別の値である。
    #[test]
    fn the_greenfield_adjustment_folds_to_skip_and_remembers_it() {
        let adjusted =
            entry(PhaseId::Inception, PlanAction::Execute, true).adjusted_for_greenfield();
        assert_eq!(adjusted.plan_action(), PlanAction::Skip);
        assert!(adjusted.is_greenfield_adjusted());
        assert!(adjusted.is_conditional(), "宣言は変わらない");
        let plain_skip = entry(PhaseId::Inception, PlanAction::Skip, true);
        assert!(!plain_skip.is_greenfield_adjusted());
        assert_ne!(adjusted, plain_skip, "注釈の有無は値の一部");
    }
}
