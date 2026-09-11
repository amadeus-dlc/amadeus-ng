//! 記録先ごとのセッション監査を所有する。workflow進行・承認は変更しない。
use super::{
    EventType, HookHealthTarget, SessionAuditError, SessionAuditEvent, SessionAuditEventId,
    SessionAuditId, SessionAuditObservation, SessionAuditRecord,
};
use chrono::{DateTime, Utc};
#[derive(Debug, Clone, PartialEq, Eq)]
/// 記録先に帰属するセッション監査の集約。
pub struct SessionAudit {
    id: SessionAuditId,
    target: HookHealthTarget,
    last_record: SessionAuditRecord,
    observation_id: super::SessionAuditObservationId,
    seq_nr: usize,
    version: usize,
    occurred_at: DateTime<Utc>,
}
impl SessionAudit {
    /// 対象を固定した完全snapshotを構築する。
    /// # Errors
    /// ID/対象/通番が不整合の場合。
    pub fn new(
        id: SessionAuditId,
        target: HookHealthTarget,
        last_record: SessionAuditRecord,
        observation_id: super::SessionAuditObservationId,
        seq_nr: usize,
        version: usize,
        occurred_at: DateTime<Utc>,
    ) -> Result<Self, SessionAuditError> {
        if id != SessionAuditId::for_target(&target) || seq_nr == 0 {
            return Err(SessionAuditError::InvalidHistory);
        }
        Ok(Self {
            id,
            target,
            last_record,
            observation_id,
            seq_nr,
            version,
            occurred_at,
        })
    }
    /// 観測対象から集約IDを導く。
    #[must_use]
    pub fn id_for(observation: &SessionAuditObservation) -> SessionAuditId {
        SessionAuditId::for_target(observation.target())
    }
    fn applies(observation: &SessionAuditObservation) -> bool {
        observation.record().kind() != EventType::SubagentCompleted
            || observation.workflow_status() == "Running"
    }
    /// 初回の適用を判断する。無視する完了通知では集約を作らない。
    /// # Errors
    /// 材料の不整合。
    pub fn start(
        observation: &SessionAuditObservation,
        at: DateTime<Utc>,
    ) -> Result<Option<(Self, SessionAuditEvent)>, SessionAuditError> {
        if !Self::applies(observation) {
            return Ok(None);
        }
        let id = Self::id_for(observation);
        let event = SessionAuditEvent::new(
            SessionAuditEventId::generate(),
            observation.id().clone(),
            id.clone(),
            observation.target().clone(),
            observation.record().clone(),
        )?;
        let aggregate = Self::new(
            id,
            event.target().clone(),
            event.record().clone(),
            event.observation_id().clone(),
            1,
            0,
            at,
        )?;
        Ok(Some((aggregate, event)))
    }
    /// 適用可能な通知を単一イベントにする。
    /// # Errors
    /// 別対象または通番上限。
    pub fn record(
        &mut self,
        observation: &SessionAuditObservation,
        at: DateTime<Utc>,
    ) -> Result<Option<SessionAuditEvent>, SessionAuditError> {
        if observation.target() != &self.target {
            return Err(SessionAuditError::TargetMismatch);
        }
        if !Self::applies(observation) {
            return Ok(None);
        }
        let seq = self
            .seq_nr
            .checked_add(1)
            .ok_or(SessionAuditError::CounterExhausted)?;
        let event = SessionAuditEvent::new(
            SessionAuditEventId::generate(),
            observation.id().clone(),
            self.id.clone(),
            self.target.clone(),
            observation.record().clone(),
        )?;
        self.apply_event(&event, seq, at);
        Ok(Some(event))
    }
    /// 保存された事実を直接適用する。
    /// # Panics
    /// 壊れた対象/集約ID/通番を含む履歴。
    pub fn apply_event(&mut self, event: &SessionAuditEvent, seq: usize, at: DateTime<Utc>) {
        assert!(
            event.aggregate_id() == &self.id
                && event.target() == &self.target
                && self.seq_nr.checked_add(1) == Some(seq),
            "invalid SessionAudit history"
        );
        self.observation_id = event.observation_id().clone();
        self.last_record = event.record().clone();
        self.seq_nr = seq;
        self.occurred_at = at;
    }
    /// snapshotより後の保存事実だけを畳む。
    /// # Panics
    /// 保存履歴の不整合。
    #[must_use]
    pub fn replay(
        mut snapshot: Self,
        events: impl IntoIterator<Item = (SessionAuditEvent, usize, DateTime<Utc>)>,
    ) -> Self {
        for (event, seq, at) in events {
            snapshot.apply_event(&event, seq, at);
        }
        snapshot
    }
    /// Repositoryの楽観版を引き継ぐ。
    #[must_use]
    pub const fn with_version(mut self, version: usize) -> Self {
        self.version = version;
        self
    }
    /// 最後に保存した通知ID。
    #[must_use]
    pub const fn observation_id(&self) -> &super::SessionAuditObservationId {
        &self.observation_id
    }
    /// 保存境界のID。
    #[must_use]
    pub const fn id(&self) -> &SessionAuditId {
        &self.id
    }
    /// 保存境界の対象。
    #[must_use]
    pub const fn target(&self) -> &HookHealthTarget {
        &self.target
    }
    /// 最後の保存事実。
    #[must_use]
    pub const fn last_record(&self) -> &SessionAuditRecord {
        &self.last_record
    }
    /// 保存通番。
    #[must_use]
    pub const fn seq_nr(&self) -> usize {
        self.seq_nr
    }
    /// 楽観版。
    #[must_use]
    pub const fn version(&self) -> usize {
        self.version
    }
    /// 発生時刻。
    #[must_use]
    pub const fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
}
