//! `RecompileError` — `CompiledDefinition::recompile` のガードが拒否する形。

use std::fmt;

use super::definition_revision::DefinitionRevision;

/// 再コンパイルを受け付けられない形 (材料のみ — 利用者向け文言はアダプタ層)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecompileError {
    /// 提示された内容が現在と同じ — 書くべき事実が無い (無言の no-op にしない)。
    Unchanged {
        /// 現在の内容版 (提示された内容からも同じ値が導出される)。
        revision: DefinitionRevision,
    },
}

impl fmt::Display for RecompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecompileError::Unchanged { revision } => {
                write!(
                    f,
                    "compiled definition unchanged at revision {}",
                    revision.as_str()
                )
            }
        }
    }
}

impl std::error::Error for RecompileError {}
