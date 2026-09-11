//! フックの稼働観測を所有する集約のイベント族。
use super::{HookHealthEventId, HookHealthId};
mod audit_dropped;
mod first_drop_observed;
pub use audit_dropped::HookAuditDropped;
pub use first_drop_observed::HookFirstDropObserved;
mod heartbeat_observed;
pub use heartbeat_observed::HookHeartbeatObserved;
mod started;
pub use started::HookHealthStarted;
/// 承認・進行の履歴とは独立した観測事実。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookHealthEvent {
    /// 最初の観測がdropだった。heartbeatはまだない。
    FirstDropObserved(HookFirstDropObserved),
    /// 最初の発火を観測した。
    Started(HookHealthStarted),
    /// 後続の発火を観測した。
    HeartbeatObserved(HookHeartbeatObserved),
    /// 監査追記の失敗をdrop履歴へ記録した。
    AuditDropped(HookAuditDropped),
}
impl HookHealthEvent {
    /// イベント自身の識別子。
    #[must_use]
    pub const fn id(&self) -> &HookHealthEventId {
        match self {
            Self::FirstDropObserved(event) => event.id(),
            Self::Started(event) => event.id(),
            Self::HeartbeatObserved(event) => event.id(),
            Self::AuditDropped(event) => event.id(),
        }
    }
    /// 所有する集約の識別子。
    #[must_use]
    pub const fn aggregate_id(&self) -> &HookHealthId {
        match self {
            Self::FirstDropObserved(event) => event.aggregate_id(),
            Self::Started(event) => event.aggregate_id(),
            Self::HeartbeatObserved(event) => event.aggregate_id(),
            Self::AuditDropped(event) => event.aggregate_id(),
        }
    }
}
