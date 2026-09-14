//! 自己診断を 1 回実施した事実 — 評価済みの行を運ぶ。

use super::{
    DoctorChecks, HookHealthTarget, WorkspaceDoctorError, WorkspaceDoctorEventId, WorkspaceDoctorId,
};

/// 1 回の診断の記録。行は集約の判断の写しであり、投影はこれを再実装せず集約を再生して読む。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDoctorEvent {
    id: WorkspaceDoctorEventId,
    aggregate_id: WorkspaceDoctorId,
    target: HookHealthTarget,
    checks: DoctorChecks,
}

impl WorkspaceDoctorEvent {
    /// 保存境界の全材料を検査する。
    ///
    /// # Errors
    ///
    /// 集約 ID と対象が対応しない場合。
    pub fn new(
        id: WorkspaceDoctorEventId,
        aggregate_id: WorkspaceDoctorId,
        target: HookHealthTarget,
        checks: DoctorChecks,
    ) -> Result<Self, WorkspaceDoctorError> {
        if aggregate_id != WorkspaceDoctorId::for_target(&target) {
            return Err(WorkspaceDoctorError::TargetMismatch);
        }
        Ok(Self {
            id,
            aggregate_id,
            target,
            checks,
        })
    }

    /// 事実自身の識別子。
    #[must_use]
    pub const fn id(&self) -> &WorkspaceDoctorEventId {
        &self.id
    }

    /// 所有する集約。
    #[must_use]
    pub const fn aggregate_id(&self) -> &WorkspaceDoctorId {
        &self.aggregate_id
    }

    /// 診断の対象。
    #[must_use]
    pub const fn target(&self) -> &HookHealthTarget {
        &self.target
    }

    /// 評価済みの行。
    #[must_use]
    pub const fn checks(&self) -> &DoctorChecks {
        &self.checks
    }
}
