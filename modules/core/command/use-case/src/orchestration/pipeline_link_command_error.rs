//! Pipeline受領の更新経路の失敗。
use super::RepositoryError;
use core_command_domain::{
    orchestration::{IntentExecutionId, IntentId, PipelineLinkError},
    workflow_definition::WorkflowDefinitionId,
};
/// 失敗した境界と原因を保持する。
#[derive(Debug)]
pub enum PipelineLinkCommandError {
    /// 実行の取得・保存失敗。
    Execution(RepositoryError<IntentExecutionId>),
    /// intentの取得失敗。
    Intent(RepositoryError<IntentId>),
    /// 定義の取得失敗。
    Definition(RepositoryError<WorkflowDefinitionId>),
    /// 集約による受領拒否。
    Rejected(PipelineLinkError),
}
impl std::fmt::Display for PipelineLinkCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Execution(e) => write!(f, "execution: {e}"),
            Self::Intent(e) => write!(f, "intent: {e}"),
            Self::Definition(e) => write!(f, "definition: {e}"),
            Self::Rejected(e) => write!(f, "{e}"),
        }
    }
}
impl std::error::Error for PipelineLinkCommandError {}
