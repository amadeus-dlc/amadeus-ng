//! 共有集約から投影する計画回答の結果行。
/// 操作IDが主キーで、Queryはこの結果をそのまま読む。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanAnswerRow {
    id: String,
    status: String,
    emitted: Option<String>,
    stage: String,
    error: Option<String>,
}
impl PlanAnswerRow {
    /// 投影した値を束ねる。
    #[must_use]
    pub const fn new(
        id: String,
        status: String,
        emitted: Option<String>,
        stage: String,
        error: Option<String>,
    ) -> Self {
        Self {
            id,
            status,
            emitted,
            stage,
            error,
        }
    }
    /// 投影済みのid。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// 投影済みのstatus。
    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }
    /// 投影済みのemitted。
    #[must_use]
    pub fn emitted(&self) -> Option<&str> {
        self.emitted.as_deref()
    }
    /// 投影済みのstage。
    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }
    /// 投影済みのerror。
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}
