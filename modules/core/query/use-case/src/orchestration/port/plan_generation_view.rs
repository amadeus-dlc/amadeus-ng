//! 操作IDで読み返す実装開始の独立ビュー。
/// 開始判断は持たず、RMUが確定した値だけを運ぶ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanGenerationView {
    id: String,
    status: String,
    unit: Option<String>,
    error: Option<String>,
    as_of: u64,
}
impl PlanGenerationView {
    /// 投影行の値を束ねる。
    #[must_use]
    pub const fn new(
        id: String,
        status: String,
        unit: Option<String>,
        error: Option<String>,
        as_of: u64,
    ) -> Self {
        Self {
            id,
            status,
            unit,
            error,
            as_of,
        }
    }
    /// 指定した操作ID。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// 投影済みの開始状態。
    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }
    /// 対象Unit。stage-levelはNone。
    #[must_use]
    pub fn unit(&self) -> Option<&str> {
        self.unit.as_deref()
    }
    /// 失効時の逐語理由。
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
