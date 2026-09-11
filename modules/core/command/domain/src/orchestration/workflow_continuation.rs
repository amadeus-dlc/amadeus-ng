//! 実行ごとの進捗署名と、進捗なしの停止要求回数を所有する集約。
use super::{
    ContinuationError, ContinuationRequest, WorkflowContinuationEvent, WorkflowContinuationEventId,
    WorkflowContinuationId,
};
use chrono::{DateTime, Utc};
#[derive(Debug, Clone, PartialEq, Eq)]
/// 実行ごとの進捗署名と、進捗なしの停止要求回数を所有する集約。
pub struct WorkflowContinuation {
    id: WorkflowContinuationId,
    last_request: ContinuationRequest,
    counter: super::ContinuationCounter,
    seq_nr: usize,
    version: usize,
    occurred_at: DateTime<Utc>,
}
impl WorkflowContinuation {
    /// 保存された完全状態の構築口。
    /// # Errors
    /// 集約識別子や通番が矛盾する場合。
    pub fn new(
        id: WorkflowContinuationId,
        last_request: ContinuationRequest,
        counter: super::ContinuationCounter,
        seq_nr: usize,
        version: usize,
        occurred_at: DateTime<Utc>,
    ) -> Result<Self, ContinuationError> {
        let count = counter.selected().count();
        let expected = counter.before().after(&last_request)?;
        let beyond_history = u64::try_from(seq_nr)
            .ok()
            .and_then(|sequence| sequence.checked_add(1))
            .is_some_and(|maximum| count > maximum);
        if seq_nr == 0
            || counter.selected() != &expected
            || last_request
                .observed_guard()
                .is_some_and(|observed| observed != counter.before())
            || seq_nr == 1
                && last_request.wait().is_none()
                && !last_request.is_wait_probe()
                && counter.published().is_some()
            || (last_request.wait().is_some() || last_request.is_wait_probe())
                && counter.published() != Some(true)
            || beyond_history && last_request.observed_guard().is_none()
        {
            return Err(ContinuationError::InvalidHistory);
        }
        Ok(Self {
            id,
            last_request,
            counter,
            seq_nr,
            version,
            occurred_at,
        })
    }
    /// 初回停止要求を判断し、誕生の事実と集約を返す。
    /// # Errors
    /// 入力と保存事実が矛盾する場合。
    pub fn start(
        id: WorkflowContinuationId,
        request: ContinuationRequest,
        at: DateTime<Utc>,
    ) -> Result<(Self, WorkflowContinuationEvent), ContinuationError> {
        let before = request
            .observed_guard()
            .cloned()
            .unwrap_or(super::ContinuationGuard::new(None, 0, false)?);
        let guard = before.after(&request)?;
        let published = (request.wait().is_some() || request.is_wait_probe()).then_some(true);
        let count = guard.count();
        let blocked = request.wait().is_none()
            && !request.is_wait_probe()
            && !request.is_reset()
            && count < request.limit();
        let event = WorkflowContinuationEvent::new(
            WorkflowContinuationEventId::generate(),
            id.clone(),
            request,
            count,
            blocked,
        )?;
        Ok((
            Self::new(
                id,
                event.request().clone(),
                super::ContinuationCounter::new(before, guard, published),
                1,
                0,
                at,
            )?,
            event,
        ))
    }
    /// 進捗が変われば再入中でも1へ戻し、同じ進捗なら回数を増やす。
    /// # Errors
    /// 通番または反復回数が上限の場合。
    pub fn consider(
        &mut self,
        request: ContinuationRequest,
        at: DateTime<Utc>,
    ) -> Result<WorkflowContinuationEvent, ContinuationError> {
        if self.counter.published().is_none() {
            return Err(ContinuationError::InvalidHistory);
        }
        let seq = self
            .seq_nr
            .checked_add(1)
            .ok_or(ContinuationError::CounterExhausted)?;
        let guard = request
            .observed_guard()
            .unwrap_or(self.counter.effective())
            .after(&request)?;
        let count = guard.count();
        let blocked = request.wait().is_none()
            && !request.is_wait_probe()
            && !request.is_reset()
            && count < request.limit();
        let event = WorkflowContinuationEvent::new(
            WorkflowContinuationEventId::generate(),
            self.id.clone(),
            request,
            count,
            blocked,
        )?;
        self.apply_event(&event, seq, at);
        Ok(event)
    }
    /// 要求IDに結び付いた外部IO観測を、未確定操作と照合する。
    /// # Errors
    /// 古い要求・二重確定・通番上限の場合。
    pub fn record_publication(
        &mut self,
        observation: &super::ContinuationPublicationObservation,
        at: DateTime<Utc>,
    ) -> Result<WorkflowContinuationEvent, ContinuationError> {
        self.settle_publication(observation.attempt(), observation.succeeded(), at)
    }
    /// 要求IDで未確定の公開操作を照合し、公開成否を単一イベントにする。
    /// # Errors
    /// 古い要求・二重確定・通番上限の場合。
    fn settle_publication(
        &mut self,
        attempt: &super::ContinuationAttemptId,
        succeeded: bool,
        at: DateTime<Utc>,
    ) -> Result<WorkflowContinuationEvent, ContinuationError> {
        if self.counter.published().is_some() || self.last_request.id() != attempt {
            return Err(ContinuationError::InvalidHistory);
        }
        let sequence = self
            .seq_nr
            .checked_add(1)
            .ok_or(ContinuationError::CounterExhausted)?;
        let count = self.counter.selected().count();
        let event = WorkflowContinuationEvent::new(
            WorkflowContinuationEventId::generate(),
            self.id.clone(),
            self.last_request.clone(),
            count,
            !self.last_request.is_reset() && count < self.last_request.limit(),
        )?
        .with_publication(succeeded);
        self.apply_event(&event, sequence, at);
        Ok(event)
    }
    /// 確定済みの履歴を適用する。新しい事実は生成しない。
    /// # Panics
    /// 別集約・通番欠落・反復回数の矛盾を含む壊れた履歴の場合。
    #[allow(clippy::panic, reason = "集約の壊れた履歴は回復せず停止する裁定")]
    pub fn apply_event(
        &mut self,
        event: &WorkflowContinuationEvent,
        sequence: usize,
        at: DateTime<Utc>,
    ) {
        assert!(
            event.aggregate_id() == &self.id && self.seq_nr.checked_add(1) == Some(sequence),
            "invalid WorkflowContinuation history"
        );
        if let Some(published) = event.publication() {
            assert!(
                self.counter.published().is_none()
                    && event.request() == &self.last_request
                    && event.count() == self.counter.selected().count(),
                "invalid WorkflowContinuation history"
            );
            self.counter = super::ContinuationCounter::new(
                self.counter.before().clone(),
                self.counter.selected().clone(),
                Some(published),
            );
        } else {
            assert!(
                self.counter.published().is_some(),
                "invalid WorkflowContinuation history"
            );
            let before = event
                .request()
                .observed_guard()
                .unwrap_or(self.counter.effective())
                .clone();
            let selected = match before.after(event.request()) {
                Ok(guard) if guard.count() == event.count() => guard,
                _ => panic!("invalid WorkflowContinuation history"),
            };
            let published = (event.request().wait().is_some() || event.request().is_wait_probe())
                .then_some(true);
            self.counter = super::ContinuationCounter::new(before, selected, published);
        }
        self.last_request = event.request().clone();
        self.seq_nr = sequence;
        self.occurred_at = at;
    }
    /// 最新スナップショット以降の事実だけを適用する。
    /// # Panics
    /// apply_eventが拒む壊れた履歴の場合。
    #[must_use]
    pub fn replay(
        mut snapshot: Self,
        events: impl IntoIterator<Item = (WorkflowContinuationEvent, usize, DateTime<Utc>)>,
    ) -> Self {
        for (event, sequence, at) in events {
            snapshot.apply_event(&event, sequence, at);
        }
        snapshot
    }
    /// Repositoryの楽観版を持つ新しい値。
    #[must_use]
    pub const fn with_version(mut self, version: usize) -> Self {
        self.version = version;
        self
    }
    /// 保存境界の識別子。
    #[must_use]
    pub const fn id(&self) -> &WorkflowContinuationId {
        &self.id
    }
    /// 保存境界の最後の要求。
    #[must_use]
    pub const fn last_request(&self) -> &ContinuationRequest {
        &self.last_request
    }
    /// 確定した停止要求の反復回数。
    #[must_use]
    pub const fn count(&self) -> u64 {
        self.counter.selected().count()
    }
    /// 最後に反復回数を更新した進捗。待機だけの要求では変化しない。
    #[must_use]
    pub const fn guard_signature(&self) -> Option<&super::ContinuationSignature> {
        self.counter.effective().signature()
    }
    /// 保存境界のcounter状態。
    #[must_use]
    pub const fn guard(&self) -> &super::ContinuationGuard {
        self.counter.effective()
    }
    /// 保存境界の公開状態。
    #[must_use]
    pub const fn counter(&self) -> &super::ContinuationCounter {
        &self.counter
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
    /// 封筒へ渡す発生時刻。
    #[must_use]
    pub const fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
}
