//! 作業に対する診断で得られた成功数と失敗数。
/// 診断の判定済み件数。個々の検査や修復手順は保持しない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthCheckResult {
    passed: u64,
    failed: u64,
}
impl HealthCheckResult {
    /// 非負の診断件数を束ねる完全コンストラクタ。
    #[must_use]
    pub const fn new(passed: u64, failed: u64) -> Self {
        Self { passed, failed }
    }
    /// 成功した検査数。
    #[must_use]
    pub const fn passed(&self) -> u64 {
        self.passed
    }
    /// 失敗した検査数。
    #[must_use]
    pub const fn failed(&self) -> u64 {
        self.failed
    }
}
