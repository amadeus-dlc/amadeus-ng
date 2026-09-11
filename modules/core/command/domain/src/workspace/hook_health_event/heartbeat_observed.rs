//! 後続のフック発火を観測した事実。
use crate::workspace::{HookHealthEventId, HookHealthId};
/// 発火時刻は保存封筒で運び、他の観測領域を書き替える材料を持たない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookHeartbeatObserved {
    id: HookHealthEventId,
    aggregate_id: HookHealthId,
}
impl HookHeartbeatObserved {
    /// 自身と所有集約の識別子を受ける。
    #[must_use]
    pub const fn new(id: HookHealthEventId, aggregate_id: HookHealthId) -> Self {
        Self { id, aggregate_id }
    }
    /// 観測事実の識別子。
    #[must_use]
    pub const fn id(&self) -> &HookHealthEventId {
        &self.id
    }
    /// 所有する集約。
    #[must_use]
    pub const fn aggregate_id(&self) -> &HookHealthId {
        &self.aggregate_id
    }
}
