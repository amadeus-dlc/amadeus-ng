//! ArtifactSavedジャーナル行のRMU内部レコード。
use super::GlobalSeqNr;
use chrono::{DateTime, Utc};
use core_command_domain::workspace::ArtifactAuditEvent;
/// 全体通番と集約内事実を一体で運ぶ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactJournalEntry {
    global_seq: GlobalSeqNr,
    seq_nr: usize,
    occurred_at: DateTime<Utc>,
    event: ArtifactAuditEvent,
}
impl ArtifactJournalEntry {
    /// 読取材料を組む。
    #[must_use]
    pub const fn new(
        global_seq: GlobalSeqNr,
        seq_nr: usize,
        occurred_at: DateTime<Utc>,
        event: ArtifactAuditEvent,
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
    /// 集約内通番。
    #[must_use]
    pub const fn seq_nr(&self) -> usize {
        self.seq_nr
    }
    /// 発生時刻。
    #[must_use]
    pub const fn occurred_at(&self) -> &DateTime<Utc> {
        &self.occurred_at
    }
    /// 保存事実。
    #[must_use]
    pub const fn event(&self) -> &ArtifactAuditEvent {
        &self.event
    }
}
