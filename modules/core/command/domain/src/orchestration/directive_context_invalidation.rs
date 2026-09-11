//! 文脈失効を要求した時点の所有者と対象。
use super::IntentId;
/// PreCompactで観測した境界。失効可否は実行集約が判断する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectiveContextInvalidation {
    intent_id: IntentId,
    project_sha256: String,
    state_sha256: String,
    session: String,
}
impl DirectiveContextInvalidation {
    /// 観測した全境界を束ねる。
    #[must_use]
    pub const fn new(
        intent_id: IntentId,
        project_sha256: String,
        state_sha256: String,
        session: String,
    ) -> Self {
        Self {
            intent_id,
            project_sha256,
            state_sha256,
            session,
        }
    }
    /// 観測した依頼。
    #[must_use]
    pub const fn intent_id(&self) -> &IntentId {
        &self.intent_id
    }
    /// 観測したプロジェクト。
    #[must_use]
    pub fn project_sha256(&self) -> &str {
        &self.project_sha256
    }
    /// 観測した状態本文。
    #[must_use]
    pub fn state_sha256(&self) -> &str {
        &self.state_sha256
    }
    /// フックから受け取ったセッション。
    #[must_use]
    pub fn session(&self) -> &str {
        &self.session
    }
}
