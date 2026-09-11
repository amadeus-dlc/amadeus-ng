//! 最後に実際に更新した停止counter。未観測とreset済みを区別する。
use super::{ContinuationError, ContinuationRequest, ContinuationSignature};
/// 停止進捗と連続回数を一体で保持する値オブジェクト。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuationGuard {
    signature: Option<ContinuationSignature>,
    count: u64,
    initialized: bool,
}
impl ContinuationGuard {
    /// 完全な保存材料を検査する。
    /// # Errors
    /// 署名・回数・未観測フラグが矛盾する場合。
    pub fn new(
        signature: Option<ContinuationSignature>,
        count: u64,
        initialized: bool,
    ) -> Result<Self, ContinuationError> {
        if (count == 0) != signature.is_none() || !initialized && count != 0 {
            return Err(ContinuationError::InvalidHistory);
        }
        Ok(Self {
            signature,
            count,
            initialized,
        })
    }
    /// 待機はcounterを変えず、resetは観測済みの0、通常停止は本家の再入規則で進める。
    /// # Errors
    /// 回数が上限へ達した場合。
    pub fn after(&self, request: &ContinuationRequest) -> Result<Self, ContinuationError> {
        if request.wait().is_some() || request.is_wait_probe() {
            return Ok(self.clone());
        }
        if request.is_reset() {
            return Self::new(None, 0, true);
        }
        let count = if request.signature() == self.signature.as_ref() {
            self.count
                .checked_add(1)
                .ok_or(ContinuationError::CounterExhausted)?
        } else if !self.initialized && request.is_reentrant() {
            2
        } else {
            1
        };
        Self::new(request.signature().cloned(), count, true)
    }
    /// 保存境界の署名。
    #[must_use]
    pub const fn signature(&self) -> Option<&ContinuationSignature> {
        self.signature.as_ref()
    }
    /// 保存境界の回数。
    #[must_use]
    pub const fn count(&self) -> u64 {
        self.count
    }
    /// counter更新を一度でも観測したか。
    #[must_use]
    pub const fn is_initialized(&self) -> bool {
        self.initialized
    }
}
