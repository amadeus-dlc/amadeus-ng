//! セッション監査の全体通番と保存事実。
use super::GlobalSeqNr;
use chrono::{DateTime, Utc};
use core_command_domain::workspace::SessionAuditEvent;
#[derive(Debug, Clone, PartialEq, Eq)]
/// 単一の保存済み監査を全体位置とともに運ぶ。
pub struct SessionJournalEntry {
    global_seq: GlobalSeqNr,
    seq_nr: usize,
    occurred_at: DateTime<Utc>,
    event: SessionAuditEvent,
}
impl SessionJournalEntry {
    /// 検査済みジャーナル行を構築する。
    #[must_use]
    pub const fn new(
        global_seq: GlobalSeqNr,
        seq_nr: usize,
        occurred_at: DateTime<Utc>,
        event: SessionAuditEvent,
    ) -> Self {
        Self {
            global_seq,
            seq_nr,
            occurred_at,
            event,
        }
    }
    /// 全体通番。
    #[must_use]
    pub const fn global_seq(&self) -> GlobalSeqNr {
        self.global_seq
    }
    /// 集約通番。
    #[must_use]
    pub const fn seq_nr(&self) -> usize {
        self.seq_nr
    }
    /// 発生時刻。
    #[must_use]
    pub const fn occurred_at(&self) -> &DateTime<Utc> {
        &self.occurred_at
    }
    /// 保存された事実。
    #[must_use]
    pub const fn event(&self) -> &SessionAuditEvent {
        &self.event
    }
}
