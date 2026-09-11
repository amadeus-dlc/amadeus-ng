//! ワークスペース全体の承認コマンドの失敗。
use super::RepositoryError;
use core_command_domain::orchestration::{
    IntentExecutionId, PlanApprovalRuntimeId, PlanRuntimeError,
};
/// ドメインの拒否と保存境界の失敗を区別する。
#[derive(Debug)]
pub enum PlanApprovalCommandError {
    /// 計画・指示・質問の照合失敗。
    Evidence(core_command_domain::orchestration::PlanApprovalError),
    /// 実行集約が質問を記録できない。
    ExecutionCommand(core_command_domain::orchestration::CommandError),
    /// 実行が参照する依頼の読取失敗。
    IntentRepository(RepositoryError<core_command_domain::orchestration::IntentId>),
    /// 集約の不変条件による拒否。
    Domain(PlanRuntimeError),
    /// 読込み・保存・競合の失敗。
    Repository(RepositoryError<PlanApprovalRuntimeId>),
    /// 元の指示発行を所有する実行の読取失敗。
    ExecutionRepository(RepositoryError<IntentExecutionId>),
}
impl std::fmt::Display for PlanApprovalCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Evidence(error) => error.fmt(f),
            Self::ExecutionCommand(error) => error.fmt(f),
            Self::IntentRepository(error) => error.fmt(f),
            Self::Domain(error) => error.fmt(f),
            Self::Repository(error) => error.fmt(f),
            Self::ExecutionRepository(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for PlanApprovalCommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::Evidence(error) => error,
            Self::ExecutionCommand(error) => error,
            Self::IntentRepository(error) => error,
            Self::Domain(error) => error,
            Self::Repository(error) => error,
            Self::ExecutionRepository(error) => error,
        })
    }
}
impl From<PlanRuntimeError> for PlanApprovalCommandError {
    fn from(error: PlanRuntimeError) -> Self {
        Self::Domain(error)
    }
}
impl From<RepositoryError<PlanApprovalRuntimeId>> for PlanApprovalCommandError {
    fn from(error: RepositoryError<PlanApprovalRuntimeId>) -> Self {
        Self::Repository(error)
    }
}

impl From<RepositoryError<IntentExecutionId>> for PlanApprovalCommandError {
    fn from(error: RepositoryError<IntentExecutionId>) -> Self {
        Self::ExecutionRepository(error)
    }
}

impl From<core_command_domain::orchestration::PlanApprovalError> for PlanApprovalCommandError {
    fn from(error: core_command_domain::orchestration::PlanApprovalError) -> Self {
        Self::Evidence(error)
    }
}
impl From<core_command_domain::orchestration::CommandError> for PlanApprovalCommandError {
    fn from(error: core_command_domain::orchestration::CommandError) -> Self {
        Self::ExecutionCommand(error)
    }
}
impl From<RepositoryError<core_command_domain::orchestration::IntentId>>
    for PlanApprovalCommandError
{
    fn from(error: RepositoryError<core_command_domain::orchestration::IntentId>) -> Self {
        Self::IntentRepository(error)
    }
}
