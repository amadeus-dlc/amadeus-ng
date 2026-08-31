//! `DefineWorkflowError` — `DefineWorkflowUseCase` の失敗。

use std::fmt;

use core_command_domain::workflow_definition::{RedefineError, WorkflowDefinitionId};

use super::port::{DefinitionArtifactsError, RepositoryError};

/// `DefineWorkflowUseCase` の失敗 (材料のみ — 逐語文言は出す側が組む)。
///
/// 3 変種はいずれも**そのまま伝播させるための封筒**である。ユースケースはポートや集約の
/// 拒否を握り潰さないし言い換えもしない (`coding-rules/error-handling.md`)。失敗の位置は
/// 復旧手順を変えるので変種で分かる必要がある — 配布物が読めないのはハーネス配置の問題、
/// ストアの失敗は永続化の問題である。
///
/// **「内容が変わっていない」はここに現れない。** それは失敗ではなく取込が冪等であること
/// の帰結であり、ユースケースが `Ok` へ畳む ([`DefineWorkflowUseCase`] の doc を参照)。
///
/// `Clone` / `PartialEq` は実装しない — `Corrupt` の `source` (原因連鎖) が比較・複製不能で
/// ある (裁定 6 で受容済み)。テストは `matches!` で判定する。
///
/// [`DefineWorkflowUseCase`]: super::define_workflow_use_case::DefineWorkflowUseCase
#[derive(Debug)]
pub enum DefineWorkflowError {
    /// ハーネス配布物の取込の失敗 (ポートからそのまま伝播)。
    Artifacts(DefinitionArtifactsError),
    /// 定義の取得ないし永続化の失敗 (ポートからそのまま伝播)。
    DefinitionRepository(RepositoryError<WorkflowDefinitionId>),
    /// 集約が改訂を拒否した (そのまま伝播 — 通番の枯渇)。
    Redefine(RedefineError),
}

impl fmt::Display for DefineWorkflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DefineWorkflowError::Artifacts(error) => write!(f, "definition artifacts: {error}"),
            DefineWorkflowError::DefinitionRepository(error) => {
                write!(f, "definition repository: {error}")
            }
            DefineWorkflowError::Redefine(error) => write!(f, "redefine: {error}"),
        }
    }
}

impl std::error::Error for DefineWorkflowError {}

impl From<DefinitionArtifactsError> for DefineWorkflowError {
    fn from(error: DefinitionArtifactsError) -> DefineWorkflowError {
        DefineWorkflowError::Artifacts(error)
    }
}

impl From<RepositoryError<WorkflowDefinitionId>> for DefineWorkflowError {
    fn from(error: RepositoryError<WorkflowDefinitionId>) -> DefineWorkflowError {
        DefineWorkflowError::DefinitionRepository(error)
    }
}

impl From<RedefineError> for DefineWorkflowError {
    fn from(error: RedefineError) -> DefineWorkflowError {
        DefineWorkflowError::Redefine(error)
    }
}
