//! `CompiledDefinitionIdError` — `CompiledDefinitionId::parse` が拒否する形。

use std::fmt;

/// `CompiledDefinitionId::parse` が拒否する形。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompiledDefinitionIdError {
    /// 前後の空白を除くと空になる。
    Empty,
    /// 制御文字を含む。
    ControlCharacter(char),
}

impl fmt::Display for CompiledDefinitionIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompiledDefinitionIdError::Empty => f.write_str("empty"),
            CompiledDefinitionIdError::ControlCharacter(c) => {
                write!(f, "control character U+{:04X}", u32::from(*c))
            }
        }
    }
}

impl std::error::Error for CompiledDefinitionIdError {}
