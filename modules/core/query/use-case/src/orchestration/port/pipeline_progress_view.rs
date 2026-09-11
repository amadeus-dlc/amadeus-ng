//! Pipeline参照投影の読取り値。
/// RMUが確定した完了link列。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineProgressView {
    completed: String,
}
impl PipelineProgressView {
    /// 投影されたJSON列を束ねる。
    #[must_use]
    pub const fn new(completed: String) -> Self {
        Self { completed }
    }
    /// 完了linkのJSON列。
    #[must_use]
    pub fn completed(&self) -> &str {
        &self.completed
    }
}
