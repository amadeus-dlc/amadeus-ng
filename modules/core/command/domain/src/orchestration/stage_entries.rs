//! `StageEntries` — 解決済み計画そのもの (BR5.5)。

use core_infrastructure::collections::{Collection, FirstClassCollection};

use crate::workflow_definition::{PhaseId, PlanAction, StageSlug};

use super::plan_error::PlanError;
use super::stage_entry::StageEntry;
use super::stage_index::StageIndex;
use super::stage_index_set::StageIndexSet;
use super::stage_slug_set::StageSlugSet;

/// 文書順の解決済み計画 (`Intent.stages` / `Created.stages` / `Started.stages` が共有する型)。
///
/// 非空・slug 一意・initialization は EXECUTE かつ無条件、という計画そのものの不変条件を
/// 構築時に確かめる (BR2.2 / BR1.5)。検査の正本は [`StageEntry::check_plan`] であり、intent の
/// 鋳造も誕生記録の復号も同じ 1 か所を通る。
///
/// 位置は生の `usize` ではなく [`StageIndex`] で問う (BR5.1) — 添字が別の実行のものであれば
/// 範囲外として `None` になり、panic しない。絞込みは空になり得るので、結果は空を許す
/// [`Collection`] へ戻る (`coding-rules/first-class-collections.md`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageEntries {
    items: Vec<StageEntry>,
}

impl StageEntries {
    /// 文書順のステージから計画を組む (**この型の唯一の構築経路**)。
    ///
    /// # Errors
    ///
    /// 計画が空、先頭ステージが EXECUTE でない、initialization フェーズのステージが
    /// EXECUTE でないか CONDITIONAL、同じ slug が 2 回以上現れる場合は [`PlanError`]。
    pub fn new(items: Vec<StageEntry>) -> Result<StageEntries, PlanError> {
        StageEntry::check_plan(&items)?;
        Ok(StageEntries { items })
    }

    /// 文書順の位置で参照する。範囲外は `None` (panic しない)。
    #[must_use]
    pub fn at(&self, stage: StageIndex) -> Option<&StageEntry> {
        self.items.get(stage.to_usize())
    }

    /// その slug の文書順の位置。計画に無ければ `None`。
    #[must_use]
    pub fn position_of(&self, slug: &StageSlug) -> Option<StageIndex> {
        self.items
            .iter()
            .position(|entry| entry.slug() == slug)
            .map(StageIndex::new)
    }

    /// そのフェーズで最初にその計画を持つステージの位置 (skeleton ゲートの対象特定)。
    #[must_use]
    pub fn first_of(&self, phase: PhaseId, plan_action: PlanAction) -> Option<StageIndex> {
        self.items
            .iter()
            .position(|entry| entry.phase() == phase && entry.plan_action() == plan_action)
            .map(StageIndex::new)
    }

    /// 位置集合が名指すステージの slug 集合。計画に無い位置は写さない。
    #[must_use]
    pub fn slugs_at(&self, positions: &StageIndexSet) -> StageSlugSet {
        StageSlugSet::new(positions.fold_left(Vec::new(), |mut slugs, stage| {
            if let Some(entry) = self.at(stage) {
                slugs.push(entry.slug().clone());
            }
            slugs
        }))
    }

    /// ステージの件数 (計画の長さ)。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }

    /// 常に `false` — 計画は非空である (共通契約のための観測面)。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 文書順に左から畳み込む。
    pub fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, &'a StageEntry) -> A) -> A {
        self.items.iter().fold(initial, fold)
    }

    /// 条件に一致するステージを文書順のまま残す。結果は空になり得るので空を許す型へ戻る。
    #[must_use]
    pub fn filter(&self, mut predicate: impl FnMut(&StageEntry) -> bool) -> Collection<StageEntry> {
        Collection::new(
            self.items
                .iter()
                .filter(|entry| predicate(entry))
                .cloned()
                .collect(),
        )
    }
}

impl FirstClassCollection for StageEntries {
    type Item<'a> = &'a StageEntry;
    type Filtered = Collection<StageEntry>;
    fn len(&self) -> usize {
        Self::len(self)
    }
    fn at(&self, index: usize) -> Option<Self::Item<'_>> {
        self.items.get(index)
    }
    fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, Self::Item<'a>) -> A) -> A {
        Self::fold_left(self, initial, fold)
    }
    fn filter(&self, predicate: impl FnMut(Self::Item<'_>) -> bool) -> Self::Filtered {
        Self::filter(self, predicate)
    }
}

/// 絞込結果 ([`Collection<StageEntry>`]) と計画そのものを同じ要素列として比べる。
///
/// 共通契約のハーネス (`tests/collection_contract_test.rs`) が
/// `filter(|_| true) == collection` を要求するために要る
/// (`Filtered` が `Self` でない型の帰結)。
impl PartialEq<StageEntries> for Collection<StageEntry> {
    fn eq(&self, other: &StageEntries) -> bool {
        self.len() == other.len()
            && other
                .fold_left((true, 0usize), |(equal, index), entry| {
                    (equal && self.at(index) == Some(entry), index + 1)
                })
                .0
    }
}

#[cfg(test)]
mod tests {
    use super::StageEntries;
    use crate::orchestration::stage_index::StageIndex;
    use crate::orchestration::{PlanError, StageDisplay, StageEntry, StageIndexSet, StageSlugSet};
    use crate::workflow_definition::{PhaseId, PlanAction, StageNumber, StageSlug};
    use core_infrastructure::collections::{Collection, FirstClassCollection};

    fn slug(name: &str) -> StageSlug {
        StageSlug::parse(name).unwrap()
    }

    fn entry(name: &str, phase: PhaseId, action: PlanAction, conditional: bool) -> StageEntry {
        StageEntry::new(
            slug(name),
            phase,
            action,
            conditional,
            StageDisplay::new(StageNumber::parse("0.1").unwrap(), "Stage", "orchestrator").unwrap(),
        )
    }

    fn plan() -> StageEntries {
        StageEntries::new(vec![
            entry(
                "state-init",
                PhaseId::Initialization,
                PlanAction::Execute,
                false,
            ),
            entry(
                "intent-capture",
                PhaseId::Ideation,
                PlanAction::Execute,
                false,
            ),
            entry("market-research", PhaseId::Ideation, PlanAction::Skip, true),
            entry(
                "user-stories",
                PhaseId::Inception,
                PlanAction::Execute,
                false,
            ),
        ])
        .unwrap()
    }

    #[test]
    fn an_empty_plan_is_rejected() {
        assert_eq!(StageEntries::new(Vec::new()).unwrap_err(), PlanError::Empty);
    }

    #[test]
    fn an_initialization_stage_that_is_not_execute_is_rejected() {
        assert_eq!(
            StageEntries::new(vec![entry(
                "state-init",
                PhaseId::Initialization,
                PlanAction::Skip,
                false
            )])
            .unwrap_err(),
            PlanError::InitializationMustExecute
        );
    }

    #[test]
    fn a_conditional_initialization_stage_is_rejected() {
        assert_eq!(
            StageEntries::new(vec![entry(
                "state-init",
                PhaseId::Initialization,
                PlanAction::Execute,
                true
            )])
            .unwrap_err(),
            PlanError::InitializationMustBeUnconditional
        );
    }

    #[test]
    fn a_repeated_slug_is_rejected_so_stage_references_stay_resolvable() {
        assert_eq!(
            StageEntries::new(vec![
                entry(
                    "state-init",
                    PhaseId::Initialization,
                    PlanAction::Execute,
                    false
                ),
                entry(
                    "intent-capture",
                    PhaseId::Ideation,
                    PlanAction::Execute,
                    false
                ),
                entry(
                    "intent-capture",
                    PhaseId::Inception,
                    PlanAction::Execute,
                    false
                ),
            ])
            .unwrap_err(),
            PlanError::DuplicateSlug {
                slug: "intent-capture".to_string(),
            }
        );
    }

    #[test]
    fn the_plan_answers_by_document_position_and_never_panics_past_the_end() {
        let plan = plan();
        assert_eq!(plan.len(), 4);
        assert!(!plan.is_empty());
        assert_eq!(
            plan.at(StageIndex::new(0)).map(StageEntry::slug),
            Some(&slug("state-init"))
        );
        assert_eq!(
            plan.at(StageIndex::new(3)).map(StageEntry::slug),
            Some(&slug("user-stories"))
        );
        assert_eq!(plan.at(StageIndex::new(4)), None);
        assert_eq!(plan.at(StageIndex::new(usize::MAX)), None);
    }

    #[test]
    fn a_slug_resolves_to_its_document_position_and_an_unknown_one_does_not() {
        let plan = plan();
        assert_eq!(
            plan.position_of(&slug("market-research")),
            Some(StageIndex::new(2))
        );
        assert_eq!(plan.position_of(&slug("deployment-execution")), None);
    }

    #[test]
    fn the_first_stage_of_a_phase_and_action_is_the_one_the_gate_looks_at() {
        let plan = plan();
        assert_eq!(
            plan.first_of(PhaseId::Ideation, PlanAction::Execute),
            Some(StageIndex::new(1))
        );
        assert_eq!(
            plan.first_of(PhaseId::Ideation, PlanAction::Skip),
            Some(StageIndex::new(2))
        );
        assert_eq!(plan.first_of(PhaseId::Operation, PlanAction::Execute), None);
    }

    #[test]
    fn a_position_set_maps_to_the_slug_set_it_names() {
        let plan = plan();
        let positions =
            StageIndexSet::new([StageIndex::new(2), StageIndex::new(1), StageIndex::new(9)]);
        assert_eq!(
            plan.slugs_at(&positions),
            StageSlugSet::new([slug("intent-capture"), slug("market-research")]),
            "計画に無い位置は写らない"
        );
        assert_eq!(
            plan.slugs_at(&StageIndexSet::empty()),
            StageSlugSet::empty()
        );
    }

    #[test]
    fn folding_and_filtering_walk_the_document_order() {
        let plan = plan();
        assert_eq!(
            plan.fold_left(Vec::new(), |mut acc, entry| {
                acc.push(entry.slug().as_str().to_string());
                acc
            }),
            [
                "state-init",
                "intent-capture",
                "market-research",
                "user-stories"
            ]
        );
        let gated = plan.filter(|entry| entry.plan_action() == PlanAction::Skip);
        assert_eq!(
            gated,
            Collection::new(vec![entry(
                "market-research",
                PhaseId::Ideation,
                PlanAction::Skip,
                true
            )])
        );
        assert!(plan.filter(|_| false).is_empty());
        assert_eq!(plan.len(), 4, "元の計画は変わらない");
    }

    #[test]
    fn the_shared_traversal_contract_sees_the_same_plan() {
        let plan = plan();
        assert_eq!(FirstClassCollection::len(&plan), 4);
        assert!(!FirstClassCollection::is_empty(&plan));
        assert_eq!(
            FirstClassCollection::at(&plan, 1).map(StageEntry::slug),
            Some(&slug("intent-capture"))
        );
        assert_eq!(FirstClassCollection::at(&plan, 4), None);
        assert_eq!(
            FirstClassCollection::fold_left(&plan, 0, |count, _| count + 1),
            4
        );
        assert_eq!(FirstClassCollection::filter(&plan, |_| true), plan);
    }
}
