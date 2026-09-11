//! 操作IDで読み返す計画回答の独立したビュー。
/// 投影済みの結果を運び、ドメインの判断を持たない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanAnswerView {
    id: String,
    status: String,
    emitted: Option<String>,
    stage: String,
    error: Option<String>,
    as_of: u64,
}
impl PlanAnswerView {
    /// 投影行の値を束ねる。
    #[must_use]
    pub const fn new(
        id: String,
        status: String,
        emitted: Option<String>,
        stage: String,
        error: Option<String>,
        as_of: u64,
    ) -> Self {
        Self {
            id,
            status,
            emitted,
            stage,
            error,
            as_of,
        }
    }
    /// 回答操作の識別子。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// 投影済みの配送状態。
    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }
    /// 監査記録済みの場合の公開イベント名。
    #[must_use]
    pub fn emitted(&self) -> Option<&str> {
        self.emitted.as_deref()
    }
    /// 対象ステージ。
    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }
    /// 拒否の逐語文言。
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    /// 投影元の通番。
    #[must_use]
    pub const fn as_of(&self) -> u64 {
        self.as_of
    }
}
