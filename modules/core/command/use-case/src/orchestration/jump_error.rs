//! `JumpError` — `JumpUseCase` の失敗。

use std::fmt;

use core_command_domain::orchestration::{CommandError, IntentExecutionId, IntentId};

use super::port::RepositoryError;

/// [`super::JumpUseCase`] の失敗（材料のみ — 逐語文言は出す側が組む）。
///
/// 対象の解決と移動可否は集約が判断し、媒体の失敗はRepositoryが所有する。
/// この型は失敗を原因連鎖を保って伝播する。
// `Clone` / `PartialEq` は実装しない — `Corrupt` の `source` (原因連鎖) が比較・複製不能で
// ある (裁定 6 で受容済み)。テストは `matches!` で判定する。
#[derive(Debug)]
pub enum JumpError {
    /// 実行の再構成・永続化の失敗（ポートからそのまま伝播）。
    Repository(RepositoryError<IntentExecutionId>),
    /// intent の取得の失敗（ポートからそのまま伝播）。
    IntentRepository(RepositoryError<IntentId>),
    /// 集約がコマンドを拒否した（そのまま伝播 — 対象不正 / 実行状態）。
    Command(CommandError),
    /// 別 scope の列を引く定義の取得の失敗（ポートからそのまま伝播）。
    DefinitionRepository(
        RepositoryError<core_command_domain::workflow_definition::WorkflowDefinitionId>,
    ),
    /// `--scope` が定義の scope grid に無い（本家 `Unknown scope: <name>` 逐語）。
    UnknownScope(String),
}

impl fmt::Display for JumpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JumpError::Repository(error) => write!(f, "repository: {error}"),
            JumpError::IntentRepository(error) => write!(f, "intent repository: {error}"),
            JumpError::Command(error) => write!(f, "command: {error}"),
            JumpError::DefinitionRepository(error) => write!(f, "definition repository: {error}"),
            JumpError::UnknownScope(name) => write!(f, "Unknown scope: {name}"),
        }
    }
}

impl std::error::Error for JumpError {
    /// 内包した失敗へ連鎖する。
    ///
    /// **封筒は連鎖を切ってはならない。** `RepositoryError::Corrupt` は「壊れていた」としか
    /// `Display` に書かず、実材料は `Error::source` の連鎖に載せる（裁定 6）。ここで `None` を
    /// 返すと、その材料はこの型で行き止まりになり、診断には分類だけが残る。
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            JumpError::Repository(error) => Some(error),
            JumpError::IntentRepository(error) => Some(error),
            JumpError::Command(error) => Some(error),
            JumpError::DefinitionRepository(error) => Some(error),
            JumpError::UnknownScope(_) => None,
        }
    }
}

impl From<RepositoryError<IntentExecutionId>> for JumpError {
    fn from(error: RepositoryError<IntentExecutionId>) -> JumpError {
        JumpError::Repository(error)
    }
}

impl From<RepositoryError<IntentId>> for JumpError {
    fn from(error: RepositoryError<IntentId>) -> JumpError {
        JumpError::IntentRepository(error)
    }
}

impl From<CommandError> for JumpError {
    fn from(error: CommandError) -> JumpError {
        JumpError::Command(error)
    }
}
