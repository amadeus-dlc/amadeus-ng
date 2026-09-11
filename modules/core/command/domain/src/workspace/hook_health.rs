//! フックの稼働観測を所有する集約。承認や実行進行は所有しない。
use super::{HookHealthError, HookHealthEvent, HookHealthId, HookHealthTarget, HookName};
use chrono::{DateTime, Utc};
/// 観測領域とフックの組ごとに、発火の履歴を保持する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookHealth {
    id: HookHealthId,
    target: HookHealthTarget,
    hook: HookName,
    heartbeat: Option<DateTime<Utc>>,
    observed_at: DateTime<Utc>,
    seq_nr: usize,
    version: usize,
    drop_summary: super::HookDropSummary,
}
impl HookHealth {
    /// 後続の発火を1イベントとして記録する。
    /// # Errors
    /// 通番を増やせない場合。
    pub fn observe_heartbeat(
        &mut self,
        at: DateTime<Utc>,
    ) -> Result<HookHealthEvent, HookHealthError> {
        let sequence = self
            .seq_nr
            .checked_add(1)
            .ok_or(HookHealthError::CounterExhausted)?;
        let event = HookHealthEvent::HeartbeatObserved(super::HookHeartbeatObserved::new(
            super::HookHealthEventId::generate(),
            self.id.clone(),
        ));
        self.apply_event(&event, sequence, at);
        Ok(event)
    }
    /// 監査追記失敗を1件のdrop事実として記録する。
    /// # Errors
    /// 理由が空、または通番を増やせない場合。
    pub fn record_drop(
        &mut self,
        reason: &str,
        at: DateTime<Utc>,
    ) -> Result<HookHealthEvent, HookHealthError> {
        let reason = super::HookDropReason::parse(reason)?;
        let sequence = self
            .seq_nr
            .checked_add(1)
            .ok_or(HookHealthError::CounterExhausted)?;
        self.drop_summary.recorded(reason.clone())?;
        let event = HookHealthEvent::AuditDropped(super::HookAuditDropped::new(
            super::HookHealthEventId::generate(),
            self.id.clone(),
            reason,
        ));
        self.apply_event(&event, sequence, at);
        Ok(event)
    }
    /// 保存済みの差分を、同じイベント適用で再生する。
    /// # Panics
    /// 集約とイベントの不一致、通番欠落、drop回数の矛盾がある壊れた履歴の場合。
    #[must_use]
    pub fn replay(
        mut snapshot: Self,
        events: impl IntoIterator<Item = (HookHealthEvent, usize, DateTime<Utc>)>,
    ) -> Self {
        for (event, sequence, at) in events {
            snapshot.apply_event(&event, sequence, at);
        }
        snapshot
    }

    #[allow(
        clippy::panic,
        clippy::expect_used,
        reason = "保存済みの壊れた履歴を回復しない集約再生の裁定"
    )]
    fn apply_event(&mut self, event: &HookHealthEvent, sequence: usize, at: DateTime<Utc>) {
        assert!(
            event.aggregate_id() == &self.id && self.seq_nr.checked_add(1) == Some(sequence),
            "invalid HookHealth history"
        );
        match event {
            HookHealthEvent::Started(_) | HookHealthEvent::FirstDropObserved(_) => {
                panic!("invalid HookHealth history")
            }
            HookHealthEvent::AuditDropped(event) => {
                self.drop_summary = self
                    .drop_summary
                    .recorded(event.reason().clone())
                    .expect("invalid HookHealth history");
            }
            HookHealthEvent::HeartbeatObserved(_) => self.heartbeat = Some(at),
        }
        self.seq_nr = sequence;
        self.observed_at = at;
    }

    /// 保存された全状態を検査して組む完全コンストラクタ。
    /// # Errors
    /// 識別子と対象が不一致、または通番が0の場合。
    #[allow(
        clippy::too_many_arguments,
        reason = "保存済みの全観測を唯一の完全コンストラクタへ集約する"
    )]
    pub fn new(
        id: HookHealthId,
        target: HookHealthTarget,
        hook: HookName,
        heartbeat: Option<DateTime<Utc>>,
        observed_at: DateTime<Utc>,
        seq_nr: usize,
        version: usize,
        drop_summary: super::HookDropSummary,
    ) -> Result<Self, HookHealthError> {
        if id != HookHealthId::for_hook(&target, &hook) {
            return Err(HookHealthError::TargetMismatch);
        }
        if seq_nr == 0 || (heartbeat.is_none() && drop_summary.count() == 0) {
            return Err(HookHealthError::InvalidHistory);
        }
        Ok(Self {
            id,
            target,
            hook,
            heartbeat,
            observed_at,
            seq_nr,
            version,
            drop_summary,
        })
    }
    /// 最初の発火を観測し、集約と誕生事実を返す。
    /// # Errors
    /// 観測対象から有効な履歴を構築できない場合。
    pub fn start(
        target: HookHealthTarget,
        hook: HookName,
        at: DateTime<Utc>,
    ) -> Result<(Self, HookHealthEvent), HookHealthError> {
        let id = HookHealthId::for_hook(&target, &hook);
        let event = HookHealthEvent::Started(super::HookHealthStarted::new(
            super::HookHealthEventId::generate(),
            id.clone(),
            target.clone(),
            hook.clone(),
        ));
        let health = Self::new(
            id,
            target,
            hook,
            Some(at),
            at,
            1,
            0,
            super::HookDropSummary::new(0, None)?,
        )?;
        Ok((health, event))
    }
    /// 初回のdropだけを記録し、未観測のheartbeatを作らない。
    /// # Errors
    /// 理由が空、または観測対象が不正な場合。
    pub fn start_with_drop(
        target: HookHealthTarget,
        hook: HookName,
        reason: &str,
        at: DateTime<Utc>,
    ) -> Result<(Self, HookHealthEvent), HookHealthError> {
        let reason = super::HookDropReason::parse(reason)?;
        let id = HookHealthId::for_hook(&target, &hook);
        let event = HookHealthEvent::FirstDropObserved(super::HookFirstDropObserved::new(
            super::HookHealthEventId::generate(),
            id.clone(),
            target.clone(),
            hook.clone(),
            reason.clone(),
        ));
        let health = Self::new(
            id,
            target,
            hook,
            None,
            at,
            1,
            0,
            super::HookDropSummary::new(1, Some(reason))?,
        )?;
        Ok((health, event))
    }
    /// 最後の観測時刻。heartbeatとdropを区別して保存封筒へ渡す。
    #[must_use]
    pub const fn observed_at(&self) -> DateTime<Utc> {
        self.observed_at
    }

    /// 集約の識別子。
    #[must_use]
    pub const fn id(&self) -> &HookHealthId {
        &self.id
    }
    /// 観測領域。
    #[must_use]
    pub const fn target(&self) -> &HookHealthTarget {
        &self.target
    }
    /// 観測するフック。
    #[must_use]
    pub const fn hook(&self) -> &HookName {
        &self.hook
    }
    /// 最後に記録した発火時刻。壁時計は集約の外で読む。
    #[must_use]
    pub const fn heartbeat(&self) -> Option<DateTime<Utc>> {
        self.heartbeat
    }
    /// この集約の履歴通番。
    #[must_use]
    pub const fn seq_nr(&self) -> usize {
        self.seq_nr
    }
    /// 永続化境界の楽観ロック版。
    #[must_use]
    pub const fn version(&self) -> usize {
        self.version
    }
    /// 永続化境界が返す版を伴う新しい完成値。
    #[must_use]
    pub const fn with_version(mut self, version: usize) -> Self {
        self.version = version;
        self
    }
    /// 累積した監査drop件数。
    #[must_use]
    pub const fn drops(&self) -> usize {
        self.drop_summary.count()
    }
    /// 最新dropの理由。
    #[must_use]
    pub const fn latest_drop(&self) -> Option<&super::HookDropReason> {
        self.drop_summary.latest()
    }
}
