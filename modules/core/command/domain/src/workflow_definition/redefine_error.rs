//! `RedefineError` — `WorkflowDefinition::redefine` のガードが拒否する形。

use std::fmt;

use super::definition_revision::DefinitionRevision;
use super::lineage_mismatch::LineageMismatch;

/// 改訂を受け付けられない形 (材料のみ — 利用者向け文言はアダプタ層)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedefineError {
    /// 提示された内容版が現在と同じ — 書くべき事実が無い。
    ///
    /// 無言の no-op にはしない (coding-rules/aggregate-commands.md「拒否はガード付き Err」)。
    /// 取込を冪等に見せるかどうかは呼出側 (ユースケース) の判断であり、集約は「変化が無い」
    /// という事実を返すだけである。
    Unchanged {
        /// 現在と一致した内容版。
        revision: DefinitionRevision,
    },
    /// 通番が上限に達した (飽和加算で成功を装わない)。
    SequenceExhausted,
    /// 渡された配布束がこの定義の系譜のものではない (取り違えのガード)。
    Lineage(LineageMismatch),
}

impl fmt::Display for RedefineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RedefineError::Unchanged { revision } => {
                write!(f, "definition unchanged at revision {}", revision.as_str())
            }
            RedefineError::SequenceExhausted => f.write_str("sequence exhausted"),
            RedefineError::Lineage(mismatch) => write!(f, "{mismatch}"),
        }
    }
}

impl std::error::Error for RedefineError {}
