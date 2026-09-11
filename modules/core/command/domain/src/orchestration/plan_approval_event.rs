//! ワークスペース共有承認集約のイベント族。
use super::{PlanApprovalEventId, PlanApprovalRuntimeId};
mod plan_answer_aborted;
mod plan_answer_recorded;
mod plan_generation_certified;
mod plan_generation_requested;
mod plan_generation_revoked;
pub use plan_answer_aborted::PlanAnswerAborted;
pub use plan_generation_certified::PlanGenerationCertified;
pub use plan_generation_requested::PlanGenerationRequested;
pub use plan_generation_revoked::PlanGenerationRevoked;
mod plan_answer_completed;
pub use plan_answer_completed::PlanAnswerCompleted;
mod plan_challenge_issued;
mod plan_invalidation_prepared;
mod plan_invalidation_resolved;
mod plan_response_observed;
mod plan_response_prepared;
mod plan_runtime_created;
pub use plan_answer_recorded::PlanAnswerRecorded;
pub use plan_challenge_issued::PlanChallengeIssued;
pub use plan_invalidation_prepared::PlanInvalidationPrepared;
pub use plan_invalidation_resolved::PlanInvalidationResolved;
pub use plan_response_observed::PlanResponseObserved;
pub use plan_response_prepared::PlanResponsePrepared;
pub use plan_runtime_created::PlanRuntimeCreated;
/// コマンドごとに1件だけ生成する事実。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanApprovalEvent {
    /// 共有承認集約が作成された。
    Created(PlanRuntimeCreated),
    /// 実装開始を要求した。開始済みの場合も同じIDで結果を記録する。
    GenerationRequested(Box<PlanGenerationRequested>),
    /// 公開後のソース一致を確認した。
    GenerationCertified(PlanGenerationCertified),
    /// 公開後のソース変化により失効した。
    GenerationRevoked(PlanGenerationRevoked),
    /// 計画承認の選択肢が提示された。
    ChallengeIssued(Box<PlanChallengeIssued>),
    /// 人間の応答を、観測時の発行回へ照合した。
    ResponseObserved(PlanResponseObserved),
    /// 人間応答を観測時の発行回へ固定した。
    ResponsePrepared(PlanResponsePrepared),
    /// 実際の提示・応答と一致する計画回答を受領した。
    AnswerRecorded(Box<PlanAnswerRecorded>),
    /// 元の実行への監査記録を確認した。
    AnswerCompleted(PlanAnswerCompleted),
    /// 認証中に変化したソースへの受領を取り消した。
    AnswerAborted(PlanAnswerAborted),
    /// 通常指示の発行前に、共有承認の判断を保留した。
    InvalidationPrepared(PlanInvalidationPrepared),
    /// 元の指示の確定結果に従い、保留を解除した。
    InvalidationResolved(PlanInvalidationResolved),
}
impl PlanApprovalEvent {
    /// イベント自身の識別子。
    #[must_use]
    pub const fn id(&self) -> &PlanApprovalEventId {
        match self {
            Self::Created(e) => e.id(),
            Self::GenerationRequested(e) => e.id(),
            Self::GenerationCertified(e) => e.id(),
            Self::GenerationRevoked(e) => e.id(),
            Self::ChallengeIssued(e) => e.id(),
            Self::ResponseObserved(e) => e.id(),
            Self::ResponsePrepared(e) => e.id(),
            Self::AnswerRecorded(e) => e.id(),
            Self::AnswerCompleted(e) => e.id(),
            Self::AnswerAborted(e) => e.id(),
            Self::InvalidationPrepared(e) => e.id(),
            Self::InvalidationResolved(e) => e.id(),
        }
    }
    /// この事実が所属する共有集約。
    #[must_use]
    pub const fn aggregate_id(&self) -> &PlanApprovalRuntimeId {
        match self {
            Self::Created(e) => e.aggregate_id(),
            Self::GenerationRequested(e) => e.aggregate_id(),
            Self::GenerationCertified(e) => e.aggregate_id(),
            Self::GenerationRevoked(e) => e.aggregate_id(),
            Self::ChallengeIssued(e) => e.aggregate_id(),
            Self::ResponseObserved(e) => e.aggregate_id(),
            Self::ResponsePrepared(e) => e.aggregate_id(),
            Self::AnswerRecorded(e) => e.aggregate_id(),
            Self::AnswerCompleted(e) => e.aggregate_id(),
            Self::AnswerAborted(e) => e.aggregate_id(),
            Self::InvalidationPrepared(e) => e.aggregate_id(),
            Self::InvalidationResolved(e) => e.aggregate_id(),
        }
    }
}
