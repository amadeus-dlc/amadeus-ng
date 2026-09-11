//! 監査追記が失敗し、理由をdrop履歴へ記録した事実。
use crate::workspace::{HookDropReason, HookHealthEventId, HookHealthId};
/// 監査のdrop理由を所有集約へ運ぶイベント。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookAuditDropped {
    id: HookHealthEventId,
    aggregate_id: HookHealthId,
    reason: HookDropReason,
}
impl HookAuditDropped {
    /// 全フィールドを受け取る完全コンストラクタ。
    /// イベント自身の識別子。
    #[must_use]
    pub const fn new(
        id: HookHealthEventId,
        aggregate_id: HookHealthId,
        reason: HookDropReason,
    ) -> Self {
        Self {
            id,
            aggregate_id,
            reason,
        }
    }
    /// 所有する集約の識別子。
    #[must_use]
    pub const fn id(&self) -> &HookHealthEventId {
        &self.id
    }
    /// 所有する集約の識別子。
    #[must_use]
    pub const fn aggregate_id(&self) -> &HookHealthId {
        &self.aggregate_id
    }
    /// drop理由。
    #[must_use]
    pub const fn reason(&self) -> &HookDropReason {
        &self.reason
    }
}
