//! `StageSlots` — 実行の位置ごとの記録の列 (BR5.5)。

use std::collections::BTreeSet;

use core_infrastructure::collections::{Collection, FirstClassCollection};

use crate::workflow_definition::{PlanAction, StageSlug};
use crate::workspace::CheckboxState;

use super::review_verdict::ReviewVerdict;
use super::stage_entries::StageEntries;
use super::stage_index::StageIndex;
use super::stage_index_set::StageIndexSet;
use super::stage_key::StageKey;
use super::stage_slot::StageSlot;
use super::stage_slots_error::StageSlotsError;

/// 実行が持つ位置ごとの記録の列 (添字 = [`StageIndex`]、文書順)。
///
/// 旧実装の **7 並列列**を 1 要素 1 位置へ統合したもので、「7 列の長さが等しい」という
/// 不変条件はこの型で構造的に消える。残る不変条件は非空と slug 一意である。
///
/// 位置指定コマンドは範囲外を [`StageSlotsError::OutOfRange`] で**拒否**する (無言 no-op に
/// しない)。集合を受ける一括コマンドは、集合演算としてこの列に**在る位置だけ**を動かす —
/// 位置集合は区間や述語から組むので、列より広い集合を渡すことは正常な使い方である。
///
/// 絞込みは空になり得るので、結果は空を許す [`Collection`] へ戻る。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageSlots {
    items: Vec<StageSlot>,
}

impl StageSlots {
    /// `in_plan` は跳躍に使う計画で EXECUTE の位置集合 (集約が導く)。
    pub(super) fn apply_jump(
        &mut self,
        event: &super::intent_execution_event::Jumped,
        source: StageIndex,
        target: StageIndex,
        in_plan: &super::StageIndexSet,
    ) {
        for (position, slot) in self.items.iter_mut().enumerate() {
            let position = StageIndex::new(position);
            slot.apply_jump(event, position, source, target, in_plan.contains(position));
        }
    }

    pub(super) fn from_started(event: &super::intent_execution_event::Started) -> Self {
        let (_, items) =
            event
                .stages()
                .fold_left((false, Vec::new()), |(active, mut items), entry| {
                    let initialization =
                        entry.phase() == crate::workflow_definition::PhaseId::Initialization;
                    let in_scope = entry.plan_action() == PlanAction::Execute;
                    let starts = !initialization && in_scope && !active;
                    let checkbox = if initialization && in_scope {
                        CheckboxState::Completed
                    } else if starts {
                        CheckboxState::InProgress
                    } else {
                        CheckboxState::Pending
                    };
                    items.push(StageSlot::new(
                        StageKey::new(entry.slug().clone(), entry.phase()),
                        entry.plan_action(),
                        checkbox,
                        false,
                        0,
                        super::ReviewAttempt::default(),
                        false,
                        false,
                    ));
                    (active || starts, items)
                });
        Self::of_items(items)
    }

    /// 記録されたステージ遷移だけから、該当する位置の進捗を適用する。
    pub(super) fn apply_progress(&mut self, event: &super::IntentExecutionEvent) {
        for slot in &mut self.items {
            slot.apply_progress(event);
        }
        if let Some(stage) = event.advancing_stage()
            && let Some(position) = self.position_of(stage)
            && let Some(next) = self
                .items
                .iter_mut()
                .skip(position.to_usize().saturating_add(1))
                .find(|slot| slot.plan_action() == PlanAction::Execute)
        {
            next.apply_predecessor_completion(event);
        }
    }
    /// いまの承認について日誌の空記録がまだ無い承認済みの位置を、計画順に選ぶ。
    ///
    /// 「日誌が空」は観測 (`survey`) が答え、「承認済みか・記録済みか」はこの列が答える。
    /// 日誌そのものが無い位置は観測に載らないので選ばれない (件数 0 とは区別する)。
    pub(super) fn empty_memory_stages(
        &self,
        survey: &super::MemoryJournalSurvey,
    ) -> super::EmptyMemoryStages {
        super::EmptyMemoryStages::new(self.items.iter().fold(Vec::new(), |mut chosen, slot| {
            if slot.awaits_memory_empty()
                && survey
                    .find(slot.key().slug())
                    .is_some_and(super::StageMemoryJournal::is_empty)
            {
                chosen.push(slot.key().slug().clone());
            }
            chosen
        }))
    }

    /// 選ばれた位置へ「この承認について記録済み」を印す。
    pub(super) fn apply_memory_empty(&mut self, stages: &super::EmptyMemoryStages) {
        for slot in &mut self.items {
            if stages.contains(slot.key().slug()) {
                slot.record_memory_empty();
            }
        }
    }
    const fn of_items(items: Vec<StageSlot>) -> Self {
        Self { items }
    }

    /// 保存された行から列を組み直す (**永続化境界からの再構成専用**)。
    ///
    /// # Errors
    ///
    /// 位置が 0 件なら [`StageSlotsError::Empty`]、同じ slug が 2 回以上現れるなら
    /// [`StageSlotsError::DuplicateSlug`]。
    pub fn new(items: Vec<StageSlot>) -> Result<StageSlots, StageSlotsError> {
        if items.is_empty() {
            return Err(StageSlotsError::Empty);
        }
        let mut seen = BTreeSet::new();
        for slot in &items {
            if !seen.insert(slot.key().slug().as_str()) {
                return Err(StageSlotsError::DuplicateSlug {
                    slug: slot.key().slug().as_str().to_string(),
                });
            }
        }
        Ok(Self::of_items(items))
    }

    /// 誕生時の列 — 計画の各位置に未着手の記録を 1 つずつ置く。
    ///
    /// 計画 ([`StageEntries`]) が非空・slug 一意を保証しているので、この経路は失敗しない。
    #[must_use]
    pub fn genesis(stages: &StageEntries) -> StageSlots {
        Self::of_items(
            stages.fold_left(Vec::with_capacity(stages.len()), |mut slots, entry| {
                slots.push(StageSlot::genesis(
                    StageKey::new(entry.slug().clone(), entry.phase()),
                    entry.plan_action(),
                ));
                slots
            }),
        )
    }

    /// 文書順の位置で参照する。範囲外は `None` (panic しない)。
    #[must_use]
    pub fn at(&self, stage: StageIndex) -> Option<&StageSlot> {
        self.items.get(stage.to_usize())
    }

    /// その位置のイベント適用の添字 (slug + phase)。範囲外は `None`。
    #[must_use]
    pub fn stage_key(&self, stage: StageIndex) -> Option<&StageKey> {
        self.at(stage).map(StageSlot::key)
    }

    /// その slug の文書順の位置。列に無ければ `None`。
    #[must_use]
    pub fn position_of(&self, slug: &StageSlug) -> Option<StageIndex> {
        self.items
            .iter()
            .position(|slot| slot.key().slug() == slug)
            .map(StageIndex::new)
    }

    /// 位置の件数 (実行の stage_count)。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }

    /// 常に `false` — 列は非空である (共通契約のための観測面)。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 文書順に左から畳み込む。
    pub fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, &'a StageSlot) -> A) -> A {
        self.items.iter().fold(initial, fold)
    }

    /// 条件に一致する記録を文書順のまま残す。結果は空になり得るので空を許す型へ戻る。
    #[must_use]
    pub fn filter(&self, mut predicate: impl FnMut(&StageSlot) -> bool) -> Collection<StageSlot> {
        Collection::new(
            self.items
                .iter()
                .filter(|slot| predicate(slot))
                .cloned()
                .collect(),
        )
    }

    /// ゲート通過を記録する。
    ///
    /// # Errors
    ///
    /// 位置が列の外なら [`StageSlotsError::OutOfRange`]。
    pub fn record_approval(&mut self, stage: StageIndex) -> Result<(), StageSlotsError> {
        self.slot_mut(stage)?.record_approval();
        Ok(())
    }

    /// ゲート通過の記録を取り消す。
    ///
    /// # Errors
    ///
    /// 位置が列の外なら [`StageSlotsError::OutOfRange`]。
    pub fn invalidate_approval(&mut self, stage: StageIndex) -> Result<(), StageSlotsError> {
        self.slot_mut(stage)?.invalidate_approval();
        Ok(())
    }

    /// 差し戻しを 1 回数える。
    ///
    /// # Errors
    ///
    /// 位置が列の外なら [`StageSlotsError::OutOfRange`]。
    pub fn bump_revision(&mut self, stage: StageIndex) -> Result<(), StageSlotsError> {
        self.slot_mut(stage)?.bump_revision();
        Ok(())
    }

    /// その位置の現在の試行を空へ戻す (フロア)。
    ///
    /// # Errors
    ///
    /// 位置が列の外なら [`StageSlotsError::OutOfRange`]。
    pub fn reset_attempt(&mut self, stage: StageIndex) -> Result<(), StageSlotsError> {
        self.slot_mut(stage)?.reset_attempt();
        Ok(())
    }

    /// レビュー依頼を 1 件数える。
    ///
    /// # Errors
    ///
    /// 位置が列の外なら [`StageSlotsError::OutOfRange`]。
    pub fn record_review_request(
        &mut self,
        stage: StageIndex,
        iteration: u32,
        binding: super::ReviewBinding,
        retry: bool,
    ) -> Result<(), StageSlotsError> {
        self.slot_mut(stage)?
            .record_review_request(iteration, binding, retry);
        Ok(())
    }

    /// レビュー判定を 1 件閉じる。
    ///
    /// # Errors
    ///
    /// 位置が列の外なら [`StageSlotsError::OutOfRange`]。
    pub fn record_review_verdict(
        &mut self,
        stage: StageIndex,
        iteration: u32,
        verdict: ReviewVerdict,
        completion: super::ReviewCompletion,
    ) -> Result<(), StageSlotsError> {
        self.slot_mut(stage)?
            .record_review_verdict(iteration, verdict, completion);
        Ok(())
    }

    /// practices の昇格を受領済みにする。
    ///
    /// # Errors
    ///
    /// 位置が列の外なら [`StageSlotsError::OutOfRange`]。
    pub fn affirm_practices(&mut self, stage: StageIndex) -> Result<(), StageSlotsError> {
        self.slot_mut(stage)?.affirm_practices();
        Ok(())
    }

    /// 記録済みの再構成が名指すslugだけに、実効計画の反転を適用する。
    pub(super) fn apply_recomposition(
        &mut self,
        event: &super::intent_execution_event::Recomposed,
    ) {
        for slot in &mut self.items {
            slot.apply_recomposition(event);
        }
    }

    /// 名指された位置のゲート通過の記録をまとめて取り消す (巻き戻し・再合成)。
    ///
    /// [`StageSlots::mark_all`] と同じく、この列に在る位置だけを動かす。
    pub fn invalidate_approvals(&mut self, stages: &StageIndexSet) {
        stages.fold_left((), |(), stage| {
            if let Some(slot) = self.items.get_mut(stage.to_usize()) {
                slot.invalidate_approval();
            }
        });
    }

    /// 全位置の現在の試行を空へ戻す (jump のフロア)。
    pub fn reset_attempts_all(&mut self) {
        for slot in &mut self.items {
            slot.reset_attempt();
        }
    }

    /// 位置指定コマンドの共通の入口 — 範囲外は拒否する。
    fn slot_mut(&mut self, stage: StageIndex) -> Result<&mut StageSlot, StageSlotsError> {
        self.items
            .get_mut(stage.to_usize())
            .ok_or(StageSlotsError::OutOfRange { stage })
    }
}

impl FirstClassCollection for StageSlots {
    type Item<'a> = &'a StageSlot;
    type Filtered = Collection<StageSlot>;
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

/// 絞込結果 ([`Collection<StageSlot>`]) と列そのものを同じ要素列として比べる。
///
/// 共通契約のハーネス (`tests/collection_contract_test.rs`) が
/// `filter(|_| true) == collection` を要求するために要る
/// (`Filtered` が `Self` でない型の帰結)。
impl PartialEq<StageSlots> for Collection<StageSlot> {
    fn eq(&self, other: &StageSlots) -> bool {
        self.len() == other.len()
            && other
                .fold_left((true, 0usize), |(equal, index), slot| {
                    (equal && self.at(index) == Some(slot), index + 1)
                })
                .0
    }
}

#[cfg(test)]
mod tests {
    use super::StageSlots;
    use crate::orchestration::stage_index::StageIndex;
    use crate::orchestration::{
        ReviewAttempt, ReviewVerdict, StageDisplay, StageEntries, StageEntry, StageIndexSet,
        StageKey, StageSlot, StageSlotsError,
    };
    use crate::workflow_definition::{PhaseId, PlanAction, StageNumber, StageSlug};
    use crate::workspace::CheckboxState;
    use core_infrastructure::collections::{Collection, FirstClassCollection};

    fn slug(name: &str) -> StageSlug {
        StageSlug::parse(name).unwrap()
    }

    fn entry(name: &str, phase: PhaseId, action: PlanAction) -> StageEntry {
        StageEntry::new(
            slug(name),
            phase,
            action,
            false,
            StageDisplay::new(StageNumber::parse("0.1").unwrap(), "Stage", "orchestrator").unwrap(),
        )
    }

    fn plan() -> StageEntries {
        StageEntries::new(vec![
            entry("state-init", PhaseId::Initialization, PlanAction::Execute),
            entry("intent-capture", PhaseId::Ideation, PlanAction::Execute),
            entry("user-stories", PhaseId::Inception, PlanAction::Skip),
        ])
        .unwrap()
    }

    fn slots() -> StageSlots {
        StageSlots::genesis(&plan())
    }

    fn key(name: &str, phase: PhaseId) -> StageKey {
        StageKey::new(slug(name), phase)
    }

    #[test]
    fn an_empty_list_of_slots_is_rejected() {
        assert_eq!(
            StageSlots::new(Vec::new()).unwrap_err(),
            StageSlotsError::Empty
        );
    }

    #[test]
    fn a_repeated_slug_is_rejected_so_stage_references_stay_resolvable() {
        assert_eq!(
            StageSlots::new(vec![
                StageSlot::genesis(
                    key("state-init", PhaseId::Initialization),
                    PlanAction::Execute
                ),
                StageSlot::genesis(key("state-init", PhaseId::Ideation), PlanAction::Execute),
            ])
            .unwrap_err(),
            StageSlotsError::DuplicateSlug {
                slug: "state-init".to_string(),
            }
        );
    }

    #[test]
    fn started_constructs_completed_initialization_and_the_first_active_stage() {
        use crate::orchestration::{IntentExecutionEventId, IntentExecutionId, IntentId};
        let event = super::super::intent_execution_event::Started::new(
            IntentExecutionEventId::generate(),
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
            IntentId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0001").unwrap(),
            plan(),
        );
        let slots = StageSlots::from_started(&event);
        assert_eq!(
            slots.at(StageIndex::new(0)).unwrap().checkbox(),
            CheckboxState::Completed
        );
        assert_eq!(
            slots.at(StageIndex::new(1)).unwrap().checkbox(),
            CheckboxState::InProgress
        );
        assert_eq!(
            slots.at(StageIndex::new(2)).unwrap().checkbox(),
            CheckboxState::Pending
        );
        assert!(!(0..3).any(|position| slots.at(StageIndex::new(position)).unwrap().approved()));
    }

    #[test]
    fn a_recorded_completion_starts_only_the_next_effective_stage() {
        use crate::orchestration::intent_execution_event::GateApproved;
        use crate::orchestration::{
            IntentExecutionEvent, IntentExecutionEventId, IntentExecutionId,
        };
        let mut slots = StageSlots::new(vec![
            StageSlot::genesis(key("first", PhaseId::Inception), PlanAction::Execute),
            StageSlot::genesis(key("omitted", PhaseId::Inception), PlanAction::Skip),
            StageSlot::genesis(key("next", PhaseId::Inception), PlanAction::Execute),
        ])
        .unwrap();
        let event = IntentExecutionEvent::GateApproved(GateApproved::new(
            IntentExecutionEventId::generate(),
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
            slug("first"),
            None,
        ));
        slots.apply_progress(&event);
        assert_eq!(
            slots.at(StageIndex::new(0)).unwrap().checkbox(),
            CheckboxState::Completed
        );
        assert_eq!(
            slots.at(StageIndex::new(1)).unwrap().checkbox(),
            CheckboxState::Pending
        );
        assert_eq!(
            slots.at(StageIndex::new(2)).unwrap().checkbox(),
            CheckboxState::InProgress
        );
    }

    #[test]
    fn jump_events_preserve_forward_skips_and_backward_resets() {
        use crate::orchestration::intent_execution_event::Jumped;
        use crate::orchestration::{IntentExecutionEventId, IntentExecutionId};
        let mut slots = StageSlots::new(
            ["first", "middle", "last"]
                .into_iter()
                .enumerate()
                .map(|(position, name)| {
                    StageSlot::new(
                        key(name, PhaseId::Inception),
                        PlanAction::Execute,
                        if position == 0 {
                            CheckboxState::InProgress
                        } else {
                            CheckboxState::Pending
                        },
                        false,
                        0,
                        ReviewAttempt::default(),
                        false,
                        false,
                    )
                })
                .collect(),
        )
        .unwrap();
        let execution = IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap();
        let forward = Jumped::new(
            IntentExecutionEventId::generate(),
            execution.clone(),
            slug("last"),
            crate::orchestration::JumpDirection::Forward,
            None,
        );
        let all = super::StageIndexSet::range(StageIndex::new(0), StageIndex::new(3));
        slots.apply_jump(&forward, StageIndex::new(0), StageIndex::new(2), &all);
        assert_eq!(
            slots.at(StageIndex::new(0)).unwrap().checkbox(),
            CheckboxState::Skipped
        );
        assert_eq!(
            slots.at(StageIndex::new(1)).unwrap().checkbox(),
            CheckboxState::Skipped
        );
        assert_eq!(
            slots.at(StageIndex::new(2)).unwrap().checkbox(),
            CheckboxState::InProgress
        );
        let back = Jumped::new(
            IntentExecutionEventId::generate(),
            execution,
            slug("first"),
            crate::orchestration::JumpDirection::Backward,
            None,
        );
        slots.apply_jump(&back, StageIndex::new(2), StageIndex::new(0), &all);
        assert_eq!(
            slots.at(StageIndex::new(0)).unwrap().checkbox(),
            CheckboxState::InProgress
        );
        assert_eq!(
            slots.at(StageIndex::new(1)).unwrap().checkbox(),
            CheckboxState::Pending
        );
        assert_eq!(
            slots.at(StageIndex::new(2)).unwrap().checkbox(),
            CheckboxState::Pending
        );
    }

    #[test]
    fn genesis_gives_every_position_of_the_plan_a_pending_slot() {
        let slots = slots();
        assert_eq!(slots.len(), 3);
        assert!(!slots.is_empty());
        for position in 0..3 {
            let slot = slots.at(StageIndex::new(position)).unwrap();
            assert_eq!(slot.checkbox(), CheckboxState::Pending);
            assert!(!slot.approved());
            assert_eq!(slot.revision_count(), 0);
            assert_eq!(slot.review_attempt(), &ReviewAttempt::default());
            assert!(!slot.practices_affirmed());
        }
        assert_eq!(
            slots.at(StageIndex::new(2)).map(StageSlot::plan_action),
            Some(PlanAction::Skip),
            "誕生時の実効計画は静的計画そのもの"
        );
    }

    #[test]
    fn positions_answer_the_key_and_stop_at_the_end() {
        let slots = slots();
        assert_eq!(
            slots.stage_key(StageIndex::new(1)),
            Some(&key("intent-capture", PhaseId::Ideation))
        );
        assert_eq!(slots.stage_key(StageIndex::new(3)), None);
        assert_eq!(slots.at(StageIndex::new(3)), None);
        assert_eq!(slots.at(StageIndex::new(usize::MAX)), None);
    }

    #[test]
    fn a_slug_resolves_to_its_position_and_an_unknown_one_does_not() {
        let slots = slots();
        assert_eq!(
            slots.position_of(&slug("user-stories")),
            Some(StageIndex::new(2))
        );
        assert_eq!(slots.position_of(&slug("deployment-execution")), None);
    }

    #[test]
    fn a_command_aimed_past_the_end_is_refused_instead_of_being_dropped() {
        let mut slots = slots();
        let out_of_range = StageIndex::new(3);
        assert_eq!(
            slots.record_approval(out_of_range).unwrap_err(),
            StageSlotsError::OutOfRange {
                stage: out_of_range
            }
        );
        assert_eq!(
            slots.bump_revision(out_of_range).unwrap_err(),
            StageSlotsError::OutOfRange {
                stage: out_of_range
            }
        );
        assert_eq!(slots, slots_unchanged(), "拒否は状態を動かさない");
    }

    fn slots_unchanged() -> StageSlots {
        StageSlots::genesis(&plan())
    }

    #[test]
    fn each_positional_command_lands_on_exactly_that_position() {
        let mut slots = slots();
        let target = StageIndex::new(1);
        slots.apply_progress(&crate::orchestration::IntentExecutionEvent::GateOpened(
            crate::orchestration::intent_execution_event::GateOpened::new(
                crate::orchestration::IntentExecutionEventId::generate(),
                crate::orchestration::IntentExecutionId::parse(
                    "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000",
                )
                .unwrap(),
                slots.stage_key(target).unwrap().slug().clone(),
                crate::orchestration::ArtifactPaths::empty(),
            ),
        ));
        slots.record_approval(target).unwrap();
        slots.bump_revision(target).unwrap();
        slots.apply_recomposition(
            &crate::orchestration::intent_execution_event::Recomposed::new(
                crate::orchestration::IntentExecutionEventId::generate(),
                crate::orchestration::IntentExecutionId::parse(
                    "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000",
                )
                .unwrap(),
                crate::orchestration::StageSlugSet::new([slots
                    .stage_key(target)
                    .unwrap()
                    .slug()
                    .clone()]),
                crate::orchestration::StageSlugSet::empty(),
            ),
        );
        slots
            .record_review_request(
                target,
                1,
                crate::orchestration::review_test_fixture::binding(),
                false,
            )
            .unwrap();
        slots
            .record_review_verdict(
                target,
                1,
                ReviewVerdict::Ready,
                crate::orchestration::review_test_fixture::completion(),
            )
            .unwrap();
        slots.affirm_practices(target).unwrap();

        let slot = slots.at(target).unwrap();
        assert_eq!(slot.checkbox(), CheckboxState::AwaitingApproval);
        assert!(slot.approved());
        assert_eq!(slot.revision_count(), 1);
        assert_eq!(slot.plan_action(), PlanAction::Skip);
        assert_eq!(slot.review_attempt().request_count(), 1);
        assert!(slot.practices_affirmed());

        let untouched = slots.at(StageIndex::new(0)).unwrap();
        assert_eq!(untouched.checkbox(), CheckboxState::Pending);
        assert!(!untouched.approved());

        slots.invalidate_approval(target).unwrap();
        assert!(!slots.at(target).unwrap().approved());
        slots.reset_attempt(target).unwrap();
        assert_eq!(
            slots.at(target).unwrap().review_attempt(),
            &ReviewAttempt::default()
        );
        assert!(!slots.at(target).unwrap().practices_affirmed());
    }

    #[test]
    fn the_bulk_commands_move_exactly_the_named_positions() {
        let mut slots = slots();
        for position in 0..3 {
            slots.record_approval(StageIndex::new(position)).unwrap();
            slots
                .record_review_request(
                    StageIndex::new(position),
                    1,
                    crate::orchestration::review_test_fixture::binding(),
                    false,
                )
                .unwrap();
        }
        let targets = StageIndexSet::new([StageIndex::new(0), StageIndex::new(2)]);

        slots.invalidate_approvals(&targets);
        assert_eq!(
            slots.at(StageIndex::new(0)).map(StageSlot::approved),
            Some(false)
        );
        assert_eq!(
            slots.at(StageIndex::new(1)).map(StageSlot::approved),
            Some(true)
        );

        slots.reset_attempts_all();
        for position in 0..3 {
            assert_eq!(
                slots
                    .at(StageIndex::new(position))
                    .unwrap()
                    .review_attempt(),
                &ReviewAttempt::default()
            );
        }
    }

    #[test]
    fn a_bulk_command_naming_a_position_past_the_end_touches_only_what_exists() {
        let mut slots = slots();
        slots.record_approval(StageIndex::new(1)).unwrap();
        slots.invalidate_approvals(&StageIndexSet::new([
            StageIndex::new(1),
            StageIndex::new(9),
        ]));
        assert_eq!(
            slots.at(StageIndex::new(1)).map(StageSlot::approved),
            Some(false)
        );
        assert_eq!(slots.len(), 3, "存在しない位置は生えない");
    }

    #[test]
    fn unfolding_the_list_and_folding_it_back_gives_the_same_value() {
        let slots = slots();
        let unfolded = slots.fold_left(Vec::new(), |mut acc, slot| {
            acc.push(slot.clone());
            acc
        });
        assert_eq!(StageSlots::new(unfolded).unwrap(), slots);
    }

    #[test]
    fn filtering_keeps_the_document_order_and_can_empty_the_list() {
        let slots = slots();
        let skipped = slots.filter(|slot| slot.plan_action() == PlanAction::Skip);
        assert_eq!(skipped.len(), 1);
        assert_eq!(
            skipped.at(0).map(StageSlot::key),
            Some(&key("user-stories", PhaseId::Inception))
        );
        assert_eq!(slots.filter(|_| false), Collection::empty());
        assert_eq!(slots.len(), 3, "元の列は変わらない");
    }

    #[test]
    fn the_shared_traversal_contract_sees_the_same_list() {
        let slots = slots();
        assert_eq!(FirstClassCollection::len(&slots), 3);
        assert!(!FirstClassCollection::is_empty(&slots));
        assert_eq!(
            FirstClassCollection::at(&slots, 1).map(StageSlot::key),
            Some(&key("intent-capture", PhaseId::Ideation))
        );
        assert_eq!(FirstClassCollection::at(&slots, 3), None);
        assert_eq!(
            FirstClassCollection::fold_left(&slots, 0, |count, _| count + 1),
            3
        );
        assert_eq!(FirstClassCollection::filter(&slots, |_| true), slots);
    }
}
