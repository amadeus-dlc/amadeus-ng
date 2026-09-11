//! SessionStartへ渡すワークフローの表示値。状態の読取りや判断は行わない。
/// SessionStartへ渡すワークフローの表示値。状態の読取りや判断は行わない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionWorkflowFields {
    scope: String,
    phase: String,
    stage: String,
    status: String,
    agent: String,
    last: String,
    next: String,
}
impl SessionWorkflowFields {
    /// 判断済みの全表示材料から構築する。
    #[must_use]
    pub const fn new(
        scope: String,
        phase: String,
        stage: String,
        status: String,
        agent: String,
        last: String,
        next: String,
    ) -> Self {
        Self {
            scope,
            phase,
            stage,
            status,
            agent,
            last,
            next,
        }
    }
    /// スコープ。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }
    /// フェーズ。
    #[must_use]
    pub fn phase(&self) -> &str {
        &self.phase
    }
    /// 現在工程。
    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }
    /// 状態。
    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }
    /// 担当。
    #[must_use]
    pub fn agent(&self) -> &str {
        &self.agent
    }
    /// 最後の完了工程。
    #[must_use]
    pub fn last(&self) -> &str {
        &self.last
    }
    /// 次の操作。
    #[must_use]
    pub fn next(&self) -> &str {
        &self.next
    }
}
