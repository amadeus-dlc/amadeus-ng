//! `AuditShardWriteError` — 監査シャードへの追記 ([`super::append_audit_shard`]) の失敗。

use std::io;

/// 監査シャードへの追記の失敗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditShardWriteError {
    /// I/O の失敗（分類だけを運ぶ — 文言はアダプタ層）。
    Io {
        /// OS 由来の分類。
        kind: io::ErrorKind,
    },
}

impl core::fmt::Display for AuditShardWriteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AuditShardWriteError::Io { kind } => write!(f, "io: {kind:?}"),
        }
    }
}

impl std::error::Error for AuditShardWriteError {}
