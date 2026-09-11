//! heartbeatに先行して監査失敗を観測した事実。
use crate::workspace::{
    HookDropReason, HookHealthEventId, HookHealthId, HookHealthTarget, HookName,
};
/// 最初のdropが属する対象と理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookFirstDropObserved {
    id: HookHealthEventId,
    aggregate_id: HookHealthId,
    target: HookHealthTarget,
    hook: HookName,
    reason: HookDropReason,
}
impl HookFirstDropObserved {
    /// 誕生事実の全材料を受け取る。
    #[must_use]
    pub const fn new(
        id: HookHealthEventId,
        aggregate_id: HookHealthId,
        target: HookHealthTarget,
        hook: HookName,
        reason: HookDropReason,
    ) -> Self {
        Self {
            id,
            aggregate_id,
            target,
            hook,
            reason,
        }
    }
    /// 事実自身の識別子。
    #[must_use]
    pub const fn id(&self) -> &HookHealthEventId {
        &self.id
    }
    /// 所有する集約。
    #[must_use]
    pub const fn aggregate_id(&self) -> &HookHealthId {
        &self.aggregate_id
    }
    /// 観測領域。
    #[must_use]
    pub const fn target(&self) -> &HookHealthTarget {
        &self.target
    }
    /// フック名。
    #[must_use]
    pub const fn hook(&self) -> &HookName {
        &self.hook
    }
    /// 失敗理由。
    #[must_use]
    pub const fn reason(&self) -> &HookDropReason {
        &self.reason
    }
}
