//! 質問と人間応答の受領状態。監査ファイルを権限の正本にしない。
use super::{
    DecisionRecorded, IntentExecutionEventId, IntentExecutionId, PendingDecisions, PromptObserved,
};
/// 集約が保持する対話の受領状態。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionState {
    plan_answers: super::PlanAppliedOperations,
    approval_observations: super::PlanAppliedOperations,
    latest_human: Option<PromptObserved>,
    consumed_human: Option<IntentExecutionEventId>,
    pending: PendingDecisions,
    summary_prompts: super::PendingSummaryDecisions,
}
impl Default for InteractionState {
    fn default() -> Self {
        Self::new(
            Default::default(),
            Default::default(),
            None,
            None,
            Default::default(),
            Default::default(),
        )
    }
}
impl InteractionState {
    /// 元の実行へ保存した計画回答の監査操作。
    #[must_use]
    pub const fn plan_answers(&self) -> &super::PlanAppliedOperations {
        &self.plan_answers
    }
    pub(crate) fn record_plan_answer(&mut self, id: super::PlanApprovalOperationId) {
        self.plan_answers.insert(id);
    }
    /// 保存済みの保護された応答操作。最新の応答を越えて保持する。
    #[must_use]
    pub const fn approval_observations(&self) -> &super::PlanAppliedOperations {
        &self.approval_observations
    }

    /// 永続化された値から完全な状態を復元する。
    #[must_use]
    pub const fn new(
        plan_answers: super::PlanAppliedOperations,
        approval_observations: super::PlanAppliedOperations,
        latest_human: Option<PromptObserved>,
        consumed_human: Option<IntentExecutionEventId>,
        pending: PendingDecisions,
        summary_prompts: super::PendingSummaryDecisions,
    ) -> Self {
        Self {
            plan_answers,
            approval_observations,
            latest_human,
            consumed_human,
            pending,
            summary_prompts,
        }
    }
    /// 最新の人間応答の事実。
    #[must_use]
    pub const fn latest_human(&self) -> Option<&PromptObserved> {
        self.latest_human.as_ref()
    }
    /// 既に回答へ使った人間応答の識別子。
    #[must_use]
    pub const fn consumed_human(&self) -> Option<&IntentExecutionEventId> {
        self.consumed_human.as_ref()
    }
    /// 未回答の提示。
    #[must_use]
    pub const fn pending(&self) -> &PendingDecisions {
        &self.pending
    }
    /// 未回答の内容確認の提示。
    #[must_use]
    pub const fn summary_prompts(&self) -> &super::PendingSummaryDecisions {
        &self.summary_prompts
    }
    pub(crate) fn summary_prompt(&self, stage: &str, file: &str) -> Option<&DecisionRecorded> {
        self.summary_prompts.get(stage, file)
    }
    pub(crate) fn human_after(&self, question: &DecisionRecorded) -> bool {
        self.latest_human
            .as_ref()
            .is_some_and(|event| Some(event.id()) != question.human_before())
    }
    pub(crate) fn consume_summary(&mut self, stage: &str, file: &str) {
        self.consume(stage);
        self.summary_prompts.clear(stage, file);
    }

    pub(crate) fn belongs_to(&self, id: &IntentExecutionId) -> bool {
        self.latest_human
            .as_ref()
            .is_none_or(|event| event.aggregate_id() == id && !event.unattended())
            && self.pending.belongs_to(id)
            && self.summary_prompts.belongs_to(id)
    }
    pub(crate) fn observe(&mut self, event: &PromptObserved) {
        if !event.unattended() {
            if let Some(id) = event.approval_observation_id() {
                self.approval_observations.insert(id.clone());
            }
            self.latest_human = Some(event.clone());
        }
    }
    pub(crate) fn record(&mut self, event: &DecisionRecorded) {
        self.pending.record(event);
        self.summary_prompts.record(event);
    }
    pub(crate) fn has_fresh_human(&self) -> bool {
        self.latest_human
            .as_ref()
            .is_some_and(|event| Some(event.id()) != self.consumed_human.as_ref())
    }
    pub(crate) fn consume(&mut self, stage: &str) {
        self.consumed_human = self.latest_human.as_ref().map(|event| event.id().clone());
        self.pending.clear(stage);
    }
    pub(crate) fn has_question(&self, stage: &str) -> bool {
        self.pending.contains(stage)
    }
    pub(crate) fn clear_question(&mut self, stage: &str) {
        self.pending.clear(stage);
    }
}
