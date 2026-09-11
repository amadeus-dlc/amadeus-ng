//! 最初のフック発火により観測の履歴が始まった事実。
use crate::workspace::{HookHealthEventId, HookHealthId, HookHealthTarget, HookName};
/// 初回heartbeatの対象。発生時刻は他のイベント族と同じく保存封筒が運ぶ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookHealthStarted {
    id: HookHealthEventId,
    aggregate_id: HookHealthId,
    target: HookHealthTarget,
    hook: HookName,
}
impl HookHealthStarted {
    /// 観測対象を含む全情報を受け取る。
    #[must_use]
    pub const fn new(
        id: HookHealthEventId,
        aggregate_id: HookHealthId,
        target: HookHealthTarget,
        hook: HookName,
    ) -> Self {
        Self {
            id,
            aggregate_id,
            target,
            hook,
        }
    }
    /// この事実自身の識別子。
    #[must_use]
    pub const fn id(&self) -> &HookHealthEventId {
        &self.id
    }
    /// 観測履歴を所有する集約。
    #[must_use]
    pub const fn aggregate_id(&self) -> &HookHealthId {
        &self.aggregate_id
    }
    /// 観測が属する領域。
    #[must_use]
    pub const fn target(&self) -> &HookHealthTarget {
        &self.target
    }
    /// 発火したフック。
    #[must_use]
    pub const fn hook(&self) -> &HookName {
        &self.hook
    }
}
