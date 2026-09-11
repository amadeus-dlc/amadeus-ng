//!成果物保存監査を所有する集約。実行進行・承認は所有しない。
use super::{
    ArtifactAuditEvent, ArtifactAuditEventId, ArtifactAuditId, ArtifactAuditRecord, ArtifactSaved,
    ArtifactWriteObservation, HookHealthError, HookHealthTarget,
};
use chrono::{DateTime, Utc};
#[derive(Debug, Clone, PartialEq, Eq)]
/// 完成した成果物監査のドメイン型。
pub struct ArtifactAudit {
    id: ArtifactAuditId,
    target: HookHealthTarget,
    seq_nr: usize,
    version: usize,
    record: ArtifactAuditRecord,
}
impl ArtifactAudit {
    /// 観測材料に対応する集約IDを導く。
    #[must_use]
    pub fn id_for(observation: &ArtifactWriteObservation) -> ArtifactAuditId {
        ArtifactAuditId::for_target(observation.target())
    }
    /// 検査済みの値を返す。
    /// # Errors
    /// 集約の不変条件または履歴通番が不正な場合。
    pub fn new(
        id: ArtifactAuditId,
        target: HookHealthTarget,
        seq_nr: usize,
        version: usize,
        record: ArtifactAuditRecord,
    ) -> Result<Self, HookHealthError> {
        if id != ArtifactAuditId::for_target(&target) || seq_nr == 0 {
            return Err(HookHealthError::InvalidHistory);
        }
        Ok(Self {
            id,
            target,
            seq_nr,
            version,
            record,
        })
    }
    /// 検査済みの値を返す。
    /// # Errors
    /// 集約の不変条件または履歴通番が不正な場合。
    pub fn start(
        observation: ArtifactWriteObservation,
        at: DateTime<Utc>,
    ) -> Result<(Self, ArtifactAuditEvent), HookHealthError> {
        let id = ArtifactAuditId::for_target(observation.target());
        let a = Self::new(
            id.clone(),
            observation.target().clone(),
            1,
            0,
            ArtifactAuditRecord::new(
                observation.file().into(),
                observation.tool().into(),
                observation.context().into(),
                observation.created(),
                at,
            ),
        )?;
        Ok((
            a,
            ArtifactAuditEvent::Saved(ArtifactSaved::new(
                ArtifactAuditEventId::generate(),
                id,
                observation,
            )),
        ))
    }
    /// 検査済みの値を返す。
    /// # Errors
    /// 集約の不変条件または履歴通番が不正な場合。
    pub fn record(
        &mut self,
        observation: ArtifactWriteObservation,
        at: DateTime<Utc>,
    ) -> Result<ArtifactAuditEvent, HookHealthError> {
        if observation.target() != &self.target {
            return Err(HookHealthError::TargetMismatch);
        }
        let seq = self
            .seq_nr
            .checked_add(1)
            .ok_or(HookHealthError::CounterExhausted)?;
        let event = ArtifactAuditEvent::Saved(ArtifactSaved::new(
            ArtifactAuditEventId::generate(),
            self.id.clone(),
            observation,
        ));
        self.apply_event(&event, seq, at);
        Ok(event)
    }

    /// 保存された事実を適用する。コマンドの再実行やイベント生成は行わない。
    /// # Panics
    /// 集約ID・記録対象・連続通番が保存済みの履歴と整合しない場合。
    pub fn apply_event(&mut self, event: &ArtifactAuditEvent, seq: usize, at: DateTime<Utc>) {
        assert_eq!(
            event.aggregate_id(),
            &self.id,
            "artifact audit history aggregate"
        );
        assert_eq!(
            Some(seq),
            self.seq_nr.checked_add(1),
            "artifact audit history sequence"
        );
        let observation = event.observation();
        assert_eq!(
            observation.target(),
            &self.target,
            "artifact audit history target"
        );
        self.record = ArtifactAuditRecord::new(
            observation.file().into(),
            observation.tool().into(),
            observation.context().into(),
            observation.created(),
            at,
        );
        self.seq_nr = seq;
    }

    /// スナップショットと、それ以降の保存事実から再構成する。
    /// # Panics
    /// 保存履歴の集約ID・対象・通番が不正な場合。
    #[must_use]
    pub fn replay(
        mut base: Self,
        events: impl IntoIterator<Item = (ArtifactAuditEvent, usize, DateTime<Utc>)>,
    ) -> Self {
        for (event, seq, at) in events {
            base.apply_event(&event, seq, at);
        }
        base
    }
    #[must_use]
    /// 検査済みの値を返す。
    pub const fn id(&self) -> &ArtifactAuditId {
        &self.id
    }
    #[must_use]
    /// 検査済みの値を返す。
    pub const fn target(&self) -> &HookHealthTarget {
        &self.target
    }
    #[must_use]
    /// 検査済みの値を返す。
    pub const fn seq_nr(&self) -> usize {
        self.seq_nr
    }
    #[must_use]
    /// 検査済みの値を返す。
    pub const fn version(&self) -> usize {
        self.version
    }
    #[must_use]
    /// 検査済みの値を返す。
    pub fn last_file(&self) -> &str {
        self.record.file()
    }
    #[must_use]
    /// 検査済みの値を返す。
    pub fn last_tool(&self) -> &str {
        self.record.tool()
    }
    #[must_use]
    /// 検査済みの値を返す。
    pub fn last_context(&self) -> &str {
        self.record.context()
    }
    #[must_use]
    /// 検査済みの値を返す。
    pub const fn last_created(&self) -> bool {
        self.record.created()
    }
    #[must_use]
    /// 検査済みの値を返す。
    pub const fn last_at(&self) -> DateTime<Utc> {
        self.record.at()
    }
    #[must_use]
    /// 検査済みの値を返す。
    pub const fn with_version(mut self, v: usize) -> Self {
        self.version = v;
        self
    }
}
