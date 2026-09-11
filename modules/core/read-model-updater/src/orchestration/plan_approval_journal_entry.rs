//! ワークスペース全体の承認イベントの読取レコード。
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::PlanApprovalEvent;
/// ストアの封筒の値とイベントを保持する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanApprovalJournalEntry {
    sequence: usize,
    at: DateTime<Utc>,
    event: PlanApprovalEvent,
}
impl PlanApprovalJournalEntry {
    /// 検査済みの行を束ねる。
    #[must_use]
    pub const fn new(sequence: usize, at: DateTime<Utc>, event: PlanApprovalEvent) -> Self {
        Self {
            sequence,
            at,
            event,
        }
    }
    /// 集約内の通番。
    #[must_use]
    pub const fn sequence(&self) -> usize {
        self.sequence
    }
    /// 元の発生時刻。
    #[must_use]
    pub const fn occurred_at(&self) -> DateTime<Utc> {
        self.at
    }
    /// 記録された事実。
    #[must_use]
    pub const fn event(&self) -> &PlanApprovalEvent {
        &self.event
    }
}
