//! 提示の特定の発行回に一致した、人間応答の観測。
use super::{PlanApprovalOperationId, PlanChoice};
/// 承認集約に属する応答。意味上の選択は観測時に確定する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanHumanResponse {
    id: PlanApprovalOperationId,
    occurrence_id: PlanApprovalOperationId,
    choice: PlanChoice,
    response_sha256: String,
}
impl PlanHumanResponse {
    /// 識別済みの応答を束ねる。
    /// # Errors
    /// 応答の指紋が不正な場合。
    pub fn new(
        id: PlanApprovalOperationId,
        occurrence_id: PlanApprovalOperationId,
        choice: PlanChoice,
        response_sha256: String,
    ) -> Result<Self, super::PlanApprovalError> {
        if response_sha256.len() != 64
            || !response_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(super::PlanApprovalError::new(
                "invalid recorded human response digest",
            ));
        }
        Ok(Self {
            id,
            occurrence_id,
            choice,
            response_sha256,
        })
    }
    /// 元の観測を識別する操作。
    #[must_use]
    pub const fn id(&self) -> &PlanApprovalOperationId {
        &self.id
    }
    /// 観測時に提示されていた発行回。
    #[must_use]
    pub const fn occurrence_id(&self) -> &PlanApprovalOperationId {
        &self.occurrence_id
    }
    /// 提示された選択肢へ照合した意味。
    #[must_use]
    pub const fn choice(&self) -> PlanChoice {
        self.choice
    }
    /// JS trimに対応する処理をした応答原文のSHA-256。
    #[must_use]
    pub fn response_sha256(&self) -> &str {
        &self.response_sha256
    }
}
