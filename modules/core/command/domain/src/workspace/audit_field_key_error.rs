//! `AuditFieldKeyError` — `AuditFieldKey::parse` の拒否 (材料のみ — 文言はアダプタ層)。

use std::fmt;

/// フィールドキーの拒否 (材料のみ — 文言はアダプタ層)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditFieldKeyError {
    /// 文法外のキー。
    Malformed {
        /// 拒否されたキーの生綴り。
        key: String,
    },
    /// 描き手が所有するキーを呼出側が供給した (`Event`)。
    EmitterOwned {
        /// 拒否されたキーの生綴り。
        key: String,
    },
}

impl fmt::Display for AuditFieldKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditFieldKeyError::Malformed { key } => write!(f, "malformed audit field key: {key}"),
            AuditFieldKeyError::EmitterOwned { key } => {
                write!(f, "emitter-owned audit field key: {key}")
            }
        }
    }
}

impl std::error::Error for AuditFieldKeyError {}
