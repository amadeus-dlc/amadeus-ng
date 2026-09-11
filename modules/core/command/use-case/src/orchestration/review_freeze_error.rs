//! 凍結判定と拒否の記録に失敗した原因。
use super::{RepositoryError, SessionAuditCommandError};
use core_command_domain::orchestration::{IntentExecutionId, IntentId};
use core_command_domain::workflow_definition::WorkflowDefinitionId;
/// 判定に要る材料が読めなかった、または拒否の記録に失敗した。
///
/// **判定そのものの結末はここに無い** — 許可も拒否も成功であり、
/// [`ReviewFreezeVerdict`] が運ぶ。
///
/// [`ReviewFreezeVerdict`]: core_command_domain::orchestration::ReviewFreezeVerdict
#[derive(Debug)]
pub enum ReviewFreezeError {
    /// 実行の再構成に失敗した。
    Execution(RepositoryError<IntentExecutionId>),
    /// 計画の再構成に失敗した。
    Intent(RepositoryError<IntentId>),
    /// 定義の再構成に失敗した。
    Definition(RepositoryError<WorkflowDefinitionId>),
    /// 拒否の監査記録に失敗した。
    Audit(SessionAuditCommandError),
    /// 監査項目名の鋳造に失敗した。
    AuditField(core_command_domain::workspace::AuditFieldKeyError),
}
impl core::fmt::Display for ReviewFreezeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Execution(error) => write!(f, "execution repository: {error}"),
            Self::Intent(error) => write!(f, "intent repository: {error}"),
            Self::Definition(error) => write!(f, "definition repository: {error}"),
            Self::Audit(error) => write!(f, "audit: {error}"),
            Self::AuditField(error) => write!(f, "audit field: {error}"),
        }
    }
}
impl std::error::Error for ReviewFreezeError {}
