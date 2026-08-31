//! `CreateIntentError` — `CreateIntentUseCase` の失敗。

use std::fmt;

use core_command_domain::orchestration::{IntentError, IntentExecutionId, IntentId};
use core_command_domain::workflow_definition::WorkflowDefinitionId;

use super::port::RepositoryError;

/// `CreateIntentUseCase` の失敗（材料のみ — 逐語文言は出す側が組む）。
///
/// 4 変種はいずれも**そのまま伝播させるための封筒**である。ユースケースはポートや集約の
/// 拒否を握り潰さないし言い換えもしない（`coding-rules/error-handling.md`）。この動詞は
/// 3 つのポートを順に叩くので、どのポートで倒れたかが変種で分かる必要がある — 失敗の
/// 位置は復旧手順を変えるからである（定義が読めないのはワークスペースの問題、intent の
/// 重複は再実行の問題、実行の書込失敗は intent だけが着地した中途半端な状態）。
// `Clone` / `PartialEq` は実装しない — `Corrupt` の `source`（原因連鎖）が比較・複製不能で
// ある（裁定 6 で受容済み）。テストは `matches!` で判定する。
#[derive(Debug)]
pub enum CreateIntentError {
    /// 定義の取得の失敗（ポートからそのまま伝播）。
    DefinitionRepository(RepositoryError<WorkflowDefinitionId>),
    /// 集約 `Intent` の genesis が拒否した（そのまま伝播 — 未知スコープなど）。
    Intent(IntentError),
    /// intent の永続化の失敗（ポートからそのまま伝播）。
    IntentRepository(RepositoryError<IntentId>),
    /// 実行の永続化の失敗（ポートからそのまま伝播）。
    ///
    /// この変種だけは **intent が既に着地したあと**の失敗である。合成ルートは同じ
    /// `intent_id` で再試行できない（intent の genesis が `Conflict` になる）ので、
    /// 出す側は「intent は作られたが実行が始まっていない」ことを言う必要がある。
    ExecutionRepository(RepositoryError<IntentExecutionId>),
}

impl fmt::Display for CreateIntentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CreateIntentError::DefinitionRepository(error) => {
                write!(f, "definition repository: {error}")
            }
            CreateIntentError::Intent(error) => write!(f, "intent: {error}"),
            CreateIntentError::IntentRepository(error) => write!(f, "intent repository: {error}"),
            CreateIntentError::ExecutionRepository(error) => {
                write!(f, "execution repository: {error}")
            }
        }
    }
}

impl std::error::Error for CreateIntentError {}

impl From<RepositoryError<WorkflowDefinitionId>> for CreateIntentError {
    fn from(error: RepositoryError<WorkflowDefinitionId>) -> CreateIntentError {
        CreateIntentError::DefinitionRepository(error)
    }
}

impl From<IntentError> for CreateIntentError {
    fn from(error: IntentError) -> CreateIntentError {
        CreateIntentError::Intent(error)
    }
}

impl From<RepositoryError<IntentId>> for CreateIntentError {
    fn from(error: RepositoryError<IntentId>) -> CreateIntentError {
        CreateIntentError::IntentRepository(error)
    }
}

impl From<RepositoryError<IntentExecutionId>> for CreateIntentError {
    fn from(error: RepositoryError<IntentExecutionId>) -> CreateIntentError {
        CreateIntentError::ExecutionRepository(error)
    }
}
