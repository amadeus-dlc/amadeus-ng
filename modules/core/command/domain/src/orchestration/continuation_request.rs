//! 停止要求の進捗・再入観測・上限。
use super::{ContinuationAttemptId, ContinuationError, ContinuationSignature};
#[derive(Debug, Clone, PartialEq, Eq)]
/// 停止要求の進捗・再入観測・上限。
pub struct ContinuationRequest {
    id: ContinuationAttemptId,
    signature: Option<ContinuationSignature>,
    reentrant: bool,
    limit: u64,
    wait: Option<super::ContinuationWait>,
    probe_only: bool,
    observed_guard: Option<super::ContinuationGuard>,
}
impl ContinuationRequest {
    /// 今回の公開counter読取りを外部IO観測として伴う。
    #[must_use]
    pub fn with_observed_guard(mut self, guard: super::ContinuationGuard) -> Self {
        self.observed_guard = Some(guard);
        self
    }
    /// 保存境界の外部IO観測。未指定時は集約が所有する直前値を使う。
    #[must_use]
    pub const fn observed_guard(&self) -> Option<&super::ContinuationGuard> {
        self.observed_guard.as_ref()
    }
    /// counterを変更せず、正当な待機だけを照会する。
    /// # Errors
    /// counterのresetと待機照会を同時に要求する場合。
    pub fn with_wait_probe(mut self) -> Result<Self, ContinuationError> {
        if self.is_reset() {
            return Err(ContinuationError::InvalidHistory);
        }
        self.probe_only = true;
        Ok(self)
    }
    /// counter更新を伴わない待機照会か。
    #[must_use]
    pub const fn is_wait_probe(&self) -> bool {
        self.probe_only
    }

    /// 実行が判断した、人間等を待つ理由を伴う要求。
    /// # Errors
    /// counterのreset要求へ待機理由を付けようとした場合。
    pub fn with_wait(
        mut self,
        wait: Option<super::ContinuationWait>,
    ) -> Result<Self, ContinuationError> {
        if wait.is_some() && self.is_reset() {
            return Err(ContinuationError::InvalidHistory);
        }
        self.wait = wait;
        Ok(self)
    }
    /// 確定した待機理由。
    #[must_use]
    pub const fn wait(&self) -> Option<super::ContinuationWait> {
        self.wait
    }
    /// 実行が所有する自律モードに従い、正の環境指定だけで上限を置き換える。
    #[must_use]
    pub fn for_mode(mut self, mode: super::AutonomyMode, raw: Option<&str>) -> Self {
        if self.is_reset() {
            return self;
        }
        let fallback = if mode.is_autonomous() { 8 } else { 2 };
        let override_limit = raw.and_then(|raw| {
            let value = core_infrastructure::ecmascript::trim(raw);
            if value.starts_with('-') {
                return None;
            }
            let value = value.strip_prefix('+').unwrap_or(value);
            let digits = value
                .chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>();
            let number = digits.parse::<f64>().ok()?;
            if !number.is_finite() || number <= 0.0 {
                return None;
            }
            // JSの有限な巨大上限は、表現可能な停止回数より大きい上限として保持する。
            Some(digits.parse::<u64>().unwrap_or(u64::MAX))
        });
        self.limit = override_limit.unwrap_or(fallback);
        self
    }
    /// 検査済み進捗と正の上限を一体で構築する。
    /// # Errors
    /// 上限が0の場合。
    pub fn new(
        id: ContinuationAttemptId,
        signature: Option<ContinuationSignature>,
        reentrant: bool,
        limit: u64,
    ) -> Result<Self, ContinuationError> {
        if limit == 0 {
            return Err(ContinuationError::InvalidLimit);
        }
        if signature.is_none() && (reentrant || limit != 1) {
            return Err(ContinuationError::InvalidHistory);
        }
        Ok(Self {
            id,
            signature,
            reentrant,
            limit,
            wait: None,
            probe_only: false,
            observed_guard: None,
        })
    }
    /// 保存境界の要求ID。
    #[must_use]
    pub const fn id(&self) -> &ContinuationAttemptId {
        &self.id
    }
    /// 保存境界の進捗署名。
    #[must_use]
    pub const fn signature(&self) -> Option<&ContinuationSignature> {
        self.signature.as_ref()
    }
    /// 進捗署名を持たず、以前の反復を解除する要求か。
    #[must_use]
    pub const fn is_reset(&self) -> bool {
        self.signature.is_none()
    }
    /// 再入観測。
    #[must_use]
    pub const fn is_reentrant(&self) -> bool {
        self.reentrant
    }
    /// 正の停止制御上限。
    #[must_use]
    pub const fn limit(&self) -> u64 {
        self.limit
    }
}
