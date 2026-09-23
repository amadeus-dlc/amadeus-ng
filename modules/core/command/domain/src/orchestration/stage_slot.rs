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
    memory_empty_reported: bool,
    summary_confirmed: bool,
}

impl StageSlot {
    /// ジャンプの到達点と、適用前の位置関係から進捗を導く。
    ///
    /// `in_plan` はこの位置が跳躍に使う計画 (この実行の実効計画、または `--scope` で名指した
    /// 別 scope の静的な列) で EXECUTE かどうか — 読み飛ばし・巻き戻しの対象はそれで決まる。
    pub(super) fn apply_jump(
        &mut self,
        event: &super::intent_execution_event::Jumped,
        position: super::StageIndex,
        source: super::StageIndex,
        target: super::StageIndex,
        in_plan: bool,
    ) {
        if self.key.slug() == event.target() {
            self.checkbox = CheckboxState::InProgress;
        } else if event.direction() == super::JumpDirection::Forward
            && ((position > source && position < target)
                || (position == source && source != target))
        {
            let skip_current = position == source && self.checkbox.is_active();
            let skip_between = position != source && in_plan && self.checkbox.is_in_flight();
            if skip_current || skip_between {
                self.checkbox = CheckboxState::Skipped;
            }
        } else if event.direction() == super::JumpDirection::Backward
            && position > target
            && in_plan
            && self.checkbox != CheckboxState::Pending
        {
            self.checkbox = CheckboxState::Pending;
        }
    }

    /// 前段の完了事実により、この位置で次の試行が始まったことを適用する。
    pub(super) const fn apply_predecessor_completion(
        &mut self,
        event: &super::IntentExecutionEvent,
    ) {
        if event.advancing_stage().is_some() {
            self.checkbox = CheckboxState::InProgress;
        }
    }

    pub(super) fn apply_progress(&mut self, event: &super::IntentExecutionEvent) {
        use super::{IntentExecutionEvent, ReportResult, ReportTransition};
        let progress = match event {
            IntentExecutionEvent::TaskSynchronized(value) => {
                Some((value.stage(), CheckboxState::InProgress))
            }
            IntentExecutionEvent::GateOpened(value) => {
                Some((value.stage(), CheckboxState::AwaitingApproval))
            }
            IntentExecutionEvent::StageRevised(value) => {
                Some((value.stage(), CheckboxState::AwaitingApproval))
            }
            IntentExecutionEvent::GateApproved(value) => {
                Some((value.stage(), CheckboxState::Completed))
            }
            IntentExecutionEvent::GateRejected(value) => {
                Some((value.stage(), CheckboxState::Revising))
            }
            IntentExecutionEvent::StageSkipped(value) => {
                Some((value.stage(), CheckboxState::Skipped))
            }
            IntentExecutionEvent::Reported(value) => match value.result() {
                ReportResult::Committed {
                    stage, transition, ..
                } => {
                    let progress = match transition {
                        ReportTransition::GateOpened { .. } | ReportTransition::StageRevised => {
                            CheckboxState::AwaitingApproval
                        }
                        ReportTransition::GateApproved { .. } => CheckboxState::Completed,
                        ReportTransition::GateRejected { .. } => CheckboxState::Revising,
                        ReportTransition::StageSkipped { .. } => CheckboxState::Skipped,
                    };
                    Some((stage, progress))
                }
                _ => None,
            },
            _ => None,
        };
        if let Some((stage, progress)) = progress
            && stage == self.key.slug()
        {
            self.checkbox = progress;
            // **新しい承認**は「この承認について記録済み」を落とす。固定本家 2.7.1 は
            // `MEMORY_EMPTY` を (slug, 承認時刻) ごとに 1 件だけ記録し、再跳躍して承認し直した
            // 位置には改めて記録する (`aidlc-runtime.ts:786-804`)。承認時刻を持たない本集約では、
            // 承認へ倒れたこと自体が「別の承認になった」印である。承認以外の進捗では落とさない —
            // 承認済みでない位置はそもそも記録の対象外なので、落とす意味が無い。
            // amadeus-lint: allow(checkbox-vocabulary) — 集約が所有する MEMORY_EMPTY 記録の前提
            if progress == CheckboxState::Completed {
                self.memory_empty_reported = false;
            }
        }
    }

    /// この承認について `MEMORY_EMPTY` を記録したと印す。
    pub(super) const fn record_memory_empty(&mut self) {
        self.memory_empty_reported = true;
    }

    /// いまの承認について `MEMORY_EMPTY` をまだ記録していない承認済みの位置か。
    ///
    /// 「承認済み」は `[x]` そのものである — 固定本家 2.7.1 の compile は
    /// `entry.completed_at !== null` (いまの対で完了している) で絞るのであって、
    /// 「一度でも承認した」ではない (`aidlc-runtime.ts:376-401`)。`approved` は再跳躍後も
    /// 立ったままなので、ここでは使えない。
    // amadeus-lint: allow(checkbox-vocabulary) — 集約が所有する MEMORY_EMPTY 記録の前提
    pub(super) const fn awaits_memory_empty(&self) -> bool {
        matches!(self.checkbox, CheckboxState::Completed) && !self.memory_empty_reported
    }

    pub(super) fn apply_recomposition(
        &mut self,
        event: &super::intent_execution_event::Recomposed,
    ) {
        if event.added().contains(self.key.slug()) {
            self.plan_action = PlanAction::Execute;
        } else if event.skipped().contains(self.key.slug()) {
            self.plan_action = PlanAction::Skip;
        }
    }

    /// 誕生時の記録 — 未着手・未承認・差戻し 0 回・空の試行・昇格の受領証なし・
    /// 日誌の空記録なし。
    #[must_use]
    pub fn genesis(key: StageKey, plan_action: PlanAction) -> StageSlot {
        Self::new(
            key,
            plan_action,
            CheckboxState::Pending,
            false,
            0,
            ReviewAttempt::default(),
            false,
            false,
            false,
        )
    }

    /// 保存された行から記録を組み直す (**永続化境界からの再構成専用**)。
    ///
    /// 通常の構築は [`StageSlot::genesis`] と、集約の適用が呼ぶコマンドである。
    #[must_use]
    #[allow(
        clippy::too_many_arguments,
        reason = "保存された 1 位置ぶんの完全な記録を唯一の再構成口へ渡す"
    )]
    pub const fn new(
        key: StageKey,
        plan_action: PlanAction,
        checkbox: CheckboxState,
        approved: bool,
        revision_count: u32,
        review_attempt: ReviewAttempt,
        practices_affirmed: bool,
        memory_empty_reported: bool,
        summary_confirmed: bool,
    ) -> StageSlot {
        StageSlot {
            key,
            plan_action,
            checkbox,
            approved,
            revision_count,
            review_attempt,
            practices_affirmed,
            memory_empty_reported,
            summary_confirmed,
        }
    }

    /// 現在の試行で、人間が内容確認（Consolidated Summary Confirmation）を返したか。
    #[must_use]
    pub const fn summary_confirmed(&self) -> bool {
        self.summary_confirmed
    }

    /// 内容確認への人間の選択を記録する（`SUMMARY_CONFIRMATION_RECORDED`）。
    ///
    /// 確認済みになるのは `Looks correct` のときだけで、`Request changes` は先の確認も
    /// 取り消す — 2.8.2 は最新の受領の `Details` が `Looks correct` でなければ
    /// `SUMMARY_RECEIPT_MISSING` で拒否し、否定の回答では承認用の記録を消す
    /// （`aidlc-log.ts` の `positive` 分岐）。
    pub const fn record_summary_choice(&mut self, choice: super::SummaryChoice) {
        self.summary_confirmed = matches!(choice, super::SummaryChoice::LooksCorrect);
    }

    /// この承認について `MEMORY_EMPTY` を既に記録したか (**永続化境界の読取専用**)。
    #[must_use]
    pub const fn memory_empty_reported(&self) -> bool {
        self.memory_empty_reported
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

    /// 現在の試行を空へ戻す (フロア — 開始・差し戻し・ジャンプ)。
    ///
    /// レビューの会計と昇格・内容確認の受領証は**同じ試行**に属するので一緒に消える。
    pub fn reset_attempt(&mut self) {
        self.review_attempt.reset();
        self.practices_affirmed = false;
        self.summary_confirmed = false;
    }

    /// レビュー依頼を 1 件数える。
    pub fn record_review_request(
        &mut self,
        iteration: u32,
        binding: super::ReviewBinding,
        retry: bool,
    ) {
        self.review_attempt
            .record_request(iteration, binding, retry);
    }

    /// レビュー判定を 1 件閉じる。
    pub fn record_review_verdict(
        &mut self,
        iteration: u32,
        verdict: ReviewVerdict,
        completion: super::ReviewCompletion,
    ) {
        self.review_attempt
            .record_verdict(iteration, verdict, completion);
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

    fn gate_opened() -> crate::orchestration::IntentExecutionEvent {
        crate::orchestration::IntentExecutionEvent::GateOpened(
            crate::orchestration::intent_execution_event::GateOpened::new(
                crate::orchestration::IntentExecutionEventId::generate(),
                crate::orchestration::IntentExecutionId::parse(
                    "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000",
                )
                .unwrap(),
                key().slug().clone(),
                crate::orchestration::ArtifactPaths::empty(),
            ),
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
                crate::orchestration::ReviewHistory::default(),
            ),
            true,
            true,
            false,
        );
        assert!(slot.memory_empty_reported());
        assert_eq!(slot.plan_action(), PlanAction::Skip);
        assert_eq!(slot.checkbox(), CheckboxState::Revising);
        assert!(slot.approved());
        assert_eq!(slot.revision_count(), 3);
        assert_eq!(slot.review_attempt().request_count(), 1);
        assert!(slot.review_attempt().is_pending(2));
        assert!(slot.practices_affirmed());
    }

    #[test]
    fn a_gate_event_changes_only_the_named_stage_progress() {
        use crate::orchestration::intent_execution_event::GateOpened;
        use crate::orchestration::{
            ArtifactPaths, IntentExecutionEvent, IntentExecutionEventId, IntentExecutionId,
        };
        let execution = IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap();
        let event = IntentExecutionEvent::GateOpened(GateOpened::new(
            IntentExecutionEventId::generate(),
            execution.clone(),
            key().slug().clone(),
            ArtifactPaths::empty(),
        ));
        let mut slot = genesis();
        slot.apply_progress(&event);
        assert_eq!(slot.checkbox(), CheckboxState::AwaitingApproval);
        assert!(!slot.approved());
        let before = slot.clone();
        let unrelated = IntentExecutionEvent::GateOpened(GateOpened::new(
            IntentExecutionEventId::generate(),
            execution,
            StageSlug::parse("other-stage").unwrap(),
            ArtifactPaths::empty(),
        ));
        slot.apply_progress(&unrelated);
        assert_eq!(slot, before);
    }

    #[test]
    fn progress_events_move_the_checkbox_and_leave_other_receipts_alone() {
        let mut slot = genesis();
        slot.apply_progress(&gate_opened());
        assert_eq!(slot.checkbox(), CheckboxState::AwaitingApproval);
        let event = crate::orchestration::IntentExecutionEvent::GateApproved(
            crate::orchestration::intent_execution_event::GateApproved::new(
                crate::orchestration::IntentExecutionEventId::generate(),
                crate::orchestration::IntentExecutionId::parse(
                    "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000",
                )
                .unwrap(),
                key().slug().clone(),
                None,
            ),
        );
        slot.apply_progress(&event);
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
            false,
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
    fn a_recorded_recomposition_changes_only_its_named_slot() {
        use crate::orchestration::intent_execution_event::Recomposed;
        use crate::orchestration::{IntentExecutionEventId, IntentExecutionId, StageSlugSet};
        let execution = IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap();
        let changed = Recomposed::new(
            IntentExecutionEventId::generate(),
            execution.clone(),
            StageSlugSet::new([key().slug().clone()]),
            StageSlugSet::empty(),
        );
        let mut slot = genesis();
        slot.apply_recomposition(&changed);
        assert_eq!(slot.plan_action(), PlanAction::Skip);
        let unrelated = Recomposed::new(
            IntentExecutionEventId::generate(),
            execution.clone(),
            StageSlugSet::empty(),
            StageSlugSet::new([StageSlug::parse("other-stage").unwrap()]),
        );
        let before = slot.clone();
        slot.apply_recomposition(&unrelated);
        assert_eq!(slot, before);
        let restored = Recomposed::new(
            IntentExecutionEventId::generate(),
            execution,
            StageSlugSet::empty(),
            StageSlugSet::new([key().slug().clone()]),
        );
        slot.apply_recomposition(&restored);
        assert_eq!(slot, genesis());
    }

    #[test]
    fn the_effective_plan_can_be_overridden_by_a_recompose() {
        let mut slot = genesis();
        slot.apply_recomposition(
            &crate::orchestration::intent_execution_event::Recomposed::new(
                crate::orchestration::IntentExecutionEventId::generate(),
                crate::orchestration::IntentExecutionId::parse(
                    "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000",
                )
                .unwrap(),
                crate::orchestration::StageSlugSet::new([key().slug().clone()]),
                crate::orchestration::StageSlugSet::empty(),
            ),
        );
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
        slot.record_review_request(
            1,
            crate::orchestration::review_test_fixture::binding(),
            false,
        );
        assert_eq!(slot.review_attempt().request_count(), 1);
        assert!(slot.review_attempt().is_pending(1));

        slot.record_review_verdict(
            1,
            ReviewVerdict::Ready,
            crate::orchestration::review_test_fixture::completion(),
        );
        assert!(!slot.review_attempt().is_pending(1));
        assert_eq!(slot.review_attempt().closed().len(), 1);
    }

    #[test]
    fn resetting_the_attempt_clears_both_receipts_of_the_current_try() {
        let mut slot = genesis();
        slot.record_review_request(
            1,
            crate::orchestration::review_test_fixture::binding(),
            false,
        );
        slot.record_review_verdict(
            1,
            ReviewVerdict::Ready,
            crate::orchestration::review_test_fixture::completion(),
        );
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
        moved.apply_progress(&gate_opened());
        assert_ne!(moved, genesis());
    }
}
