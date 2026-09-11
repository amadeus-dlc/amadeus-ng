//! 観測時の提示に対する応答照合の結果。
use crate::orchestration::{
    PlanApprovalEventId, PlanApprovalOperationId, PlanApprovalRuntimeId, PlanChoice, PlanSession,
};
/// 過去の提示への応答を、新しい提示で解釈し直さないための事実。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanResponseObserved {
    id: PlanApprovalEventId,
    aggregate_id: PlanApprovalRuntimeId,
    observation_id: PlanApprovalOperationId,
    occurrence_id: PlanApprovalOperationId,
    session: PlanSession,
    response_sha256: String,
    choice: Option<PlanChoice>,
}
impl PlanResponseObserved {
    /// 観測の識別・対象・照合結果を束ねる。
    #[must_use]
    pub const fn new(
        id: PlanApprovalEventId,
        aggregate_id: PlanApprovalRuntimeId,
        observation_id: PlanApprovalOperationId,
        occurrence_id: PlanApprovalOperationId,
        session: PlanSession,
        response_sha256: String,
        choice: Option<PlanChoice>,
    ) -> Self {
        Self {
            id,
            aggregate_id,
            observation_id,
            occurrence_id,
            session,
            response_sha256,
            choice,
        }
    }
    /// 記録されたid。
    #[must_use]
    pub const fn id(&self) -> &PlanApprovalEventId {
        &self.id
    }
    /// 記録されたaggregate_id。
    #[must_use]
    pub const fn aggregate_id(&self) -> &PlanApprovalRuntimeId {
        &self.aggregate_id
    }
    /// 記録されたobservation_id。
    #[must_use]
    pub const fn observation_id(&self) -> &PlanApprovalOperationId {
        &self.observation_id
    }
    /// 記録されたoccurrence_id。
    #[must_use]
    pub const fn occurrence_id(&self) -> &PlanApprovalOperationId {
        &self.occurrence_id
    }
    /// 記録されたsession。
    #[must_use]
    pub const fn session(&self) -> &PlanSession {
        &self.session
    }
    /// 記録されたresponse_sha256。
    #[must_use]
    pub const fn response_sha256(&self) -> &String {
        &self.response_sha256
    }
    /// 記録されたchoice。
    #[must_use]
    pub const fn choice(&self) -> &Option<PlanChoice> {
        &self.choice
    }
}
