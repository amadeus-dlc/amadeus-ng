//! 開始操作の事実から投影する結果行。
/// 操作IDを主キーとする、判断済みの開始状態。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanGenerationRow {
    id: String,
    status: String,
    unit: Option<String>,
    error: Option<String>,
}
impl PlanGenerationRow {
    /// 投影核が決定した値を束ねる。
    #[must_use]
    pub const fn new(
        id: String,
        status: String,
        unit: Option<String>,
        error: Option<String>,
    ) -> Self {
        Self {
            id,
            status,
            unit,
            error,
        }
    }
    /// 元の操作ID。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// 開始の状態。
    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }
    /// 対象Unit。
    #[must_use]
    pub fn unit(&self) -> Option<&str> {
        self.unit.as_deref()
    }
    /// 失効時の理由。
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}
