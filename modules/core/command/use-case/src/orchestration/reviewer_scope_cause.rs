//! `ReviewerScopeCause` — 読み取り範囲の拒否を記録できなかった理由。
use super::SessionAuditCommandError;
use core_command_domain::workspace::AuditFieldKeyError;

/// 記録に失敗した理由。
#[derive(Debug)]
pub enum ReviewerScopeCause {
    /// 拒否の監査記録に失敗した。
    Audit(SessionAuditCommandError),
    /// 監査項目名の鋳造に失敗した。
    AuditField(AuditFieldKeyError),
}
impl core::fmt::Display for ReviewerScopeCause {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Audit(error) => write!(f, "audit: {error}"),
            Self::AuditField(error) => write!(f, "audit field: {error}"),
        }
    }
}
