//! `StageEntry` — `Started` に載る解決済みの 1 ステージ分の計画 (entities.md StageEntry)。

use std::collections::BTreeSet;

use super::plan_error::PlanError;
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
    display: StageDisplay,
}

impl StageEntry {
    /// 解決済みの 5 成分を束ねる。
    ///
    /// `plan_action` はグリッドの 3 値 `Option<PlanAction>` を `None → SKIP` で畳んだ 2 値、
    /// `conditional` は同じ文書順の `StageNode::execution() == CONDITIONAL` (BR2.2)、
    /// `display` は投影がリードモデルを描くのに要る表示属性 3 値 ([`StageDisplay`])。
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
            display,
        }
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

    /// 文書順の計画そのものが満たすべき不変条件 (計画を所有する型の関連関数)。
    ///
    /// 同じ計画は intent の鋳造 ([`Intent::create`]) でも、実行の誕生記録 (`Started`) の
    /// 復号でも同じ形を要求される。判断の正本をここ 1 か所に置き、呼び手 (集約・DTO) は
    /// 複製しない (`coding-rules/domain-services.md` — 導出・判断はまず所有する型の関連
    /// メソッドへ)。復号の境界で呼ぶのは、破れた計画をそのまま通すと集約の再構成
    /// (`IntentExecution` の `From<(Started, _)>`) まで届いてクラッシュするからである
    /// (再構成は失敗を返さない — オーナー裁定 2026-08-30)。
    ///
    /// # Errors
    ///
    /// 計画が空、先頭ステージが EXECUTE でない、initialization フェーズのステージが
    /// EXECUTE でないか CONDITIONAL、同じ slug が 2 回以上現れる。
    ///
    /// [`Intent::create`]: crate::orchestration::Intent::create
    pub fn check_plan(stages: &[StageEntry]) -> Result<(), PlanError> {
        match stages.first() {
            None => return Err(PlanError::Empty),
            Some(first) if first.plan_action != PlanAction::Execute => {
                return Err(PlanError::InitializationMustExecute);
            }
            Some(_) => {}
        }
        for entry in stages {
            if entry.phase != PhaseId::Initialization {
                continue;
            }
            if entry.plan_action != PlanAction::Execute {
                return Err(PlanError::InitializationMustExecute);
            }
            if entry.conditional {
                return Err(PlanError::InitializationMustBeUnconditional);
            }
        }
        // slug はイベントのステージ参照の解決先 — 重複すると解決が常に前方だけを返し、
        // 静かに誤った集約になる (BR1.5。集約側の同じ検査は `IntentExecution::new` が
        // 添字帳 `StageKey` に対して持つ)。
        let mut seen = BTreeSet::new();
        for entry in stages {
            if !seen.insert(entry.slug.as_str()) {
                return Err(PlanError::DuplicateSlug {
                    slug: entry.slug.as_str().to_string(),
                });
            }
        }
        Ok(())
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

    /// 名前の違う 1 ステージ (計画の検査は slug の一意性も見る)。
    fn entry_of(name: &str, phase: PhaseId, action: PlanAction, conditional: bool) -> StageEntry {
        StageEntry::new(
            StageSlug::parse(name).unwrap(),
            phase,
            action,
            conditional,
            display(),
        )
    }

    #[test]
    fn a_sound_plan_passes_the_check() {
        let plan = vec![
            entry_of(
                "state-init",
                PhaseId::Initialization,
                PlanAction::Execute,
                false,
            ),
            entry_of(
                "intent-capture",
                PhaseId::Ideation,
                PlanAction::Execute,
                false,
            ),
            entry_of(
                "scope-definition",
                PhaseId::Ideation,
                PlanAction::Skip,
                true,
            ),
        ];
        assert_eq!(StageEntry::check_plan(&plan), Ok(()));
    }

    #[test]
    fn an_empty_plan_is_refused() {
        assert_eq!(StageEntry::check_plan(&[]), Err(PlanError::Empty));
    }

    #[test]
    fn a_plan_whose_head_is_not_execute_is_refused() {
        let plan = vec![entry_of(
            "intent-capture",
            PhaseId::Ideation,
            PlanAction::Skip,
            false,
        )];
        assert_eq!(
            StageEntry::check_plan(&plan),
            Err(PlanError::InitializationMustExecute)
        );
    }

    #[test]
    fn a_plan_that_skips_an_initialization_stage_is_refused() {
        // 状態ファイルを起こす工程そのものなので SKIP にできない (BR2.2)。
        let plan = vec![
            entry_of(
                "state-init",
                PhaseId::Initialization,
                PlanAction::Execute,
                false,
            ),
            entry_of(
                "workspace-detection",
                PhaseId::Initialization,
                PlanAction::Skip,
                false,
            ),
        ];
        assert_eq!(
            StageEntry::check_plan(&plan),
            Err(PlanError::InitializationMustExecute)
        );
    }

    #[test]
    fn a_plan_whose_initialization_stage_is_conditional_is_refused() {
        let plan = vec![
            entry_of(
                "state-init",
                PhaseId::Initialization,
                PlanAction::Execute,
                false,
            ),
            entry_of(
                "workspace-detection",
                PhaseId::Initialization,
                PlanAction::Execute,
                true,
            ),
        ];
        assert_eq!(
            StageEntry::check_plan(&plan),
            Err(PlanError::InitializationMustBeUnconditional)
        );
    }

    #[test]
    fn a_plan_that_names_the_same_stage_twice_is_refused() {
        // slug はステージ参照の解決先 — 重複すると解決が常に前方だけを返す (BR1.5)。
        let plan = vec![
            entry_of(
                "state-init",
                PhaseId::Initialization,
                PlanAction::Execute,
                false,
            ),
            entry_of(
                "intent-capture",
                PhaseId::Ideation,
                PlanAction::Execute,
                false,
            ),
            entry_of("intent-capture", PhaseId::Ideation, PlanAction::Skip, false),
        ];
        assert_eq!(
            StageEntry::check_plan(&plan),
            Err(PlanError::DuplicateSlug {
                slug: "intent-capture".to_string(),
            })
        );
    }

    #[test]
    fn entries_compare_by_value() {
        let a = entry(PhaseId::Inception, PlanAction::Execute, false);
        let b = entry(PhaseId::Inception, PlanAction::Execute, false);
        let c = entry(PhaseId::Inception, PlanAction::Skip, false);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
