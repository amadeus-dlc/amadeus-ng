//! `WorkflowDefinitionIdError` — `WorkflowDefinitionId::parse` が拒否する形。

use std::fmt;

/// `WorkflowDefinitionId::parse` が拒否する形。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowDefinitionIdError {
    /// 前後の空白を除くと空になる。
    Empty,
    /// 制御文字を含む。
    ControlCharacter(char),
}

impl fmt::Display for WorkflowDefinitionIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkflowDefinitionIdError::Empty => f.write_str("empty"),
            WorkflowDefinitionIdError::ControlCharacter(c) => {
                write!(f, "control character U+{:04X}", u32::from(*c))
            }
        }
    }
}

impl std::error::Error for WorkflowDefinitionIdError {}
