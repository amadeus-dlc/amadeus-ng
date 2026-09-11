//! 要求IDに結び付けたcounter公開のIO観測。
use super::ContinuationAttemptId;
/// 公開判断の代用品ではなく、実際に行った一回の書込みの成否。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuationPublicationObservation {
    attempt: ContinuationAttemptId,
    succeeded: bool,
}
impl ContinuationPublicationObservation {
    /// 操作の対象と観測を同時に構築する。
    #[must_use]
    pub const fn new(attempt: ContinuationAttemptId, succeeded: bool) -> Self {
        Self { attempt, succeeded }
    }
    /// 観測した要求。
    #[must_use]
    pub const fn attempt(&self) -> &ContinuationAttemptId {
        &self.attempt
    }
    /// 書込みが成功したか。
    #[must_use]
    pub const fn succeeded(&self) -> bool {
        self.succeeded
    }
}
