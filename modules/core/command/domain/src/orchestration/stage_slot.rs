//! `StageSlot` — 1 つの位置の添字と進捗記録 (BR5.5)。

use crate::workflow_definition::PlanAction;
use crate::workspace::CheckboxState;

use super::review_attempt::ReviewAttempt;
use super::review_verdict::ReviewVerdict;
use super::stage_key::StageKey;

/// 実行の 1 位置ぶんの記録 — 添字 (`key`)・実効計画・進捗・承認・受領証を 1 つの値に束ねる。
///
/// 旧実装が `stage_keys` / `overlay` / `checkbox` / `approved` / `revision_count` /
/// `review_attempts` / `practices_affirmed` の **7 並列列**で持っていたものを 1 要素 1 位置へ
/// 統合したものである。列の長さが揃うという不変条件は、この型を [`StageSlots`] に入れることで
/// 構造的に消える。
///
/// コマンドは `&mut self` で戻り値を持たない (CQS)。レビュー会計は [`ReviewAttempt`] へ委譲し、
/// この型は同じ試行に属する 2 つの受領証 (レビューと昇格) を一緒に消すフロアだけを知っている。
///
/// [`StageSlots`]: super::StageSlots
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageSlot {
    key: StageKey,
    plan_action: PlanAction,
    checkbox: CheckboxState,
    approved: bool,
    revision_count: u32,
    review_attempt: ReviewAttempt,
    practices_affirmed: bool,
}

impl StageSlot {
    /// 誕生時の記録 — 未着手・未承認・差戻し 0 回・空の試行・昇格の受領証なし。
    #[must_use]
    pub fn genesis(key: StageKey, plan_action: PlanAction) -> StageSlot {
        StageSlot {
            key,
            plan_action,
            checkbox: CheckboxState::Pending,
            approved: false,
            revision_count: 0,
            review_attempt: ReviewAttempt::default(),
            practices_affirmed: false,
        }
    }

    /// 保存された行から記録を組み直す (**永続化境界からの再構成専用**)。
    ///
    /// 通常の構築は [`StageSlot::genesis`] と、集約の適用が呼ぶコマンドである。
    #[must_use]
    pub const fn new(
        key: StageKey,
        plan_action: PlanAction,
        checkbox: CheckboxState,
        approved: bool,
        revision_count: u32,
        review_attempt: ReviewAttempt,
        practices_affirmed: bool,
    ) -> StageSlot {
        StageSlot {
            key,
            plan_action,
            checkbox,
            approved,
            revision_count,
            review_attempt,
            practices_affirmed,
        }
    }

    /// イベント適用の添字 (slug + phase)。
    #[must_use]
    pub const fn key(&self) -> &StageKey {
        &self.key
    }

    /// 実効計画 — 静的グリッド由来の計画に recompose のオーバレイを重ねた現在値。
    #[must_use]
    pub const fn plan_action(&self) -> PlanAction {
        self.plan_action
    }

    /// Stage Progress 行の状態マーカー。
    #[must_use]
    pub const fn checkbox(&self) -> CheckboxState {
        self.checkbox
    }

    /// このステージのゲートを一度でも通過したか。
    #[must_use]
    pub const fn approved(&self) -> bool {
        self.approved
    }

    /// 差し戻された回数。
    #[must_use]
    pub const fn revision_count(&self) -> u32 {
        self.revision_count
    }

    /// 現在の試行のレビュー会計。
    #[must_use]
    pub const fn review_attempt(&self) -> &ReviewAttempt {
        &self.review_attempt
    }

    /// 現在の試行で practices の昇格を受領済みか。
    #[must_use]
    pub const fn practices_affirmed(&self) -> bool {
        self.practices_affirmed
    }

    /// 状態マーカーを置き換える。
    pub const fn mark(&mut self, checkbox: CheckboxState) {
        self.checkbox = checkbox;
    }

    /// ゲート通過を記録する。
    pub const fn record_approval(&mut self) {
        self.approved = true;
    }

    /// ゲート通過の記録を取り消す (巻き戻し・再合成で通過が無かったことになる位置)。
    pub const fn invalidate_approval(&mut self) {
        self.approved = false;
    }

    /// 差し戻しを 1 回数える (飽和加算 — 溢れても回り込まない)。
    pub const fn bump_revision(&mut self) {
        self.revision_count = self.revision_count.saturating_add(1);
    }

    /// 実効計画を置き換える (recompose のオーバレイ)。
    pub const fn override_plan(&mut self, plan_action: PlanAction) {
        self.plan_action = plan_action;
    }

    /// 現在の試行を空へ戻す (フロア — 開始・差し戻し・ジャンプ)。
    ///
    /// レビューの会計と昇格の受領証は**同じ試行**に属するので一緒に消える。
    pub fn reset_attempt(&mut self) {
        self.review_attempt.reset();
        self.practices_affirmed = false;
    }

    /// レビュー依頼を 1 件数える。
    pub fn record_review_request(&mut self, iteration: u32) {
        self.review_attempt.record_request(iteration);
    }

    /// レビュー判定を 1 件閉じる。
    pub fn record_review_verdict(&mut self, iteration: u32, verdict: ReviewVerdict) {
        self.review_attempt.record_verdict(iteration, verdict);
    }

    /// practices の昇格を受領済みにする。
    pub const fn affirm_practices(&mut self) {
        self.practices_affirmed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::StageSlot;
    use crate::orchestration::{
        ReviewAttempt, ReviewClosure, ReviewClosures, ReviewVerdict, StageKey,
    };
    use crate::workflow_definition::{PhaseId, PlanAction, StageSlug};
    use crate::workspace::CheckboxState;

    fn key() -> StageKey {
        StageKey::new(
            StageSlug::parse("intent-capture").unwrap(),
            PhaseId::Ideation,
        )
    }

    fn genesis() -> StageSlot {
        StageSlot::genesis(key(), PlanAction::Execute)
    }

    #[test]
    fn a_newborn_slot_is_pending_unapproved_and_has_no_receipt() {
        let slot = genesis();
        assert_eq!(slot.key(), &key());
        assert_eq!(slot.plan_action(), PlanAction::Execute);
        assert_eq!(slot.checkbox(), CheckboxState::Pending);
        assert!(!slot.approved());
        assert_eq!(slot.revision_count(), 0);
        assert_eq!(slot.review_attempt(), &ReviewAttempt::default());
        assert!(!slot.practices_affirmed());
    }

    #[test]
    fn the_full_constructor_carries_every_attribute_for_the_persistence_boundary() {
        let slot = StageSlot::new(
            key(),
            PlanAction::Skip,
            CheckboxState::Revising,
            true,
            3,
            ReviewAttempt::restored(
                1,
                vec![2],
                ReviewClosures::new(vec![ReviewClosure::new(1, ReviewVerdict::NotReady)]),
            ),
            true,
        );
        assert_eq!(slot.plan_action(), PlanAction::Skip);
        assert_eq!(slot.checkbox(), CheckboxState::Revising);
        assert!(slot.approved());
        assert_eq!(slot.revision_count(), 3);
        assert_eq!(slot.review_attempt().request_count(), 1);
        assert!(slot.review_attempt().is_pending(2));
        assert!(slot.practices_affirmed());
    }

    #[test]
    fn marking_moves_the_checkbox_and_leaves_the_rest_alone() {
        let mut slot = genesis();
        slot.mark(CheckboxState::InProgress);
        assert_eq!(slot.checkbox(), CheckboxState::InProgress);
        slot.mark(CheckboxState::Completed);
        assert_eq!(slot.checkbox(), CheckboxState::Completed);
        assert!(!slot.approved());
        assert_eq!(slot.revision_count(), 0);
    }

    #[test]
    fn approval_is_recorded_and_can_be_invalidated_again() {
        let mut slot = genesis();
        slot.record_approval();
        assert!(slot.approved());
        slot.invalidate_approval();
        assert!(!slot.approved());
    }

    #[test]
    fn revisions_are_counted_with_saturating_addition() {
        let mut slot = StageSlot::new(
            key(),
            PlanAction::Execute,
            CheckboxState::Revising,
            false,
            u32::MAX,
            ReviewAttempt::default(),
            false,
        );
        slot.bump_revision();
        assert_eq!(slot.revision_count(), u32::MAX, "飽和加算で溢れない");

        let mut slot = genesis();
        slot.bump_revision();
        slot.bump_revision();
        assert_eq!(slot.revision_count(), 2);
    }

    #[test]
    fn the_effective_plan_can_be_overridden_by_a_recompose() {
        let mut slot = genesis();
        slot.override_plan(PlanAction::Skip);
        assert_eq!(slot.plan_action(), PlanAction::Skip);
        assert_eq!(
            slot.key(),
            &key(),
            "オーバレイは静的計画の添字帳を書き換えない"
        );
    }

    #[test]
    fn review_requests_and_verdicts_are_delegated_to_the_attempt() {
        let mut slot = genesis();
        slot.record_review_request(1);
        assert_eq!(slot.review_attempt().request_count(), 1);
        assert!(slot.review_attempt().is_pending(1));

        slot.record_review_verdict(1, ReviewVerdict::Ready);
        assert!(!slot.review_attempt().is_pending(1));
        assert_eq!(slot.review_attempt().closed().len(), 1);
    }

    #[test]
    fn resetting_the_attempt_clears_both_receipts_of_the_current_try() {
        let mut slot = genesis();
        slot.record_review_request(1);
        slot.record_review_verdict(1, ReviewVerdict::Ready);
        slot.affirm_practices();
        assert!(slot.practices_affirmed());

        slot.reset_attempt();
        assert_eq!(slot.review_attempt(), &ReviewAttempt::default());
        assert!(
            !slot.practices_affirmed(),
            "フロアは昇格の受領証も一緒に消す"
        );
    }

    #[test]
    fn two_slots_with_the_same_record_are_the_same_value() {
        assert_eq!(genesis(), genesis());
        let mut moved = genesis();
        moved.mark(CheckboxState::InProgress);
        assert_ne!(moved, genesis());
    }
}
