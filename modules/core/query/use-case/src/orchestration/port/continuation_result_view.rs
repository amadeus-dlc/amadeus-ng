//! 停止要求IDに対応する投影済みの結果。
/// 判断を再計算しないQuery DTO。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuationResultView {
    id: String,
    blocked: bool,
    count: u64,
    limit: u64,
    wait: Option<String>,
    published: Option<bool>,
    settled: bool,
}
impl ContinuationResultView {
    /// 停止回数を変更せず待機する理由。
    #[must_use]
    pub fn wait(&self) -> Option<&str> {
        self.wait.as_deref()
    }
    /// 1行の全情報を構築する。
    #[must_use]
    pub const fn new(
        id: String,
        blocked: bool,
        count: u64,
        limit: u64,
        wait: Option<String>,
        published: Option<bool>,
        settled: bool,
    ) -> Self {
        Self {
            id,
            blocked,
            count,
            limit,
            wait,
            published,
            settled,
        }
    }
    /// RMUが実行したcounter公開の成否。待機だけならNone。
    #[must_use]
    pub const fn published(&self) -> Option<bool> {
        self.published
    }
    /// 公開成否の事実が集約で確定したか。
    #[must_use]
    pub const fn settled(&self) -> bool {
        self.settled
    }
    /// 要求ID。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// 投影済みの停止判断。
    #[must_use]
    pub const fn blocked(&self) -> bool {
        self.blocked
    }
    /// 投影済みの反復回数。
    #[must_use]
    pub const fn count(&self) -> u64 {
        self.count
    }
    /// 実行と環境から確定済みの上限。
    #[must_use]
    pub const fn limit(&self) -> u64 {
        self.limit
    }
}
