//! `IntentExecutionEventIdError` — `IntentExecutionEventId::parse` が拒否する形。

use std::fmt;

/// `IntentExecutionEventId::parse` が拒否する形 (材料のみ — 利用者向け文言はアダプタ層)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentExecutionEventIdError {
    /// UUIDv7 の正準表記 (小文字 `8-4-4-4-12`・version `7`・RFC variant) でない。
    NotCanonicalUuidV7,
}

impl fmt::Display for IntentExecutionEventIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntentExecutionEventIdError::NotCanonicalUuidV7 => {
                f.write_str("not a canonical UUIDv7 (expected lowercase 8-4-4-4-12)")
            }
        }
    }
}

impl std::error::Error for IntentExecutionEventIdError {}
