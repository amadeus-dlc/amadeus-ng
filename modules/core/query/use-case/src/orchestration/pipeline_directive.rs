//! 委譲順序と完了した引継ぎの公開表現。
/// pipelineの指示。現在状態の判断は持たない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineDirective {
    links: Vec<String>,
    completed: Vec<String>,
}
impl PipelineDirective {
    /// 指示順と記録済みの完了を束ねる。
    #[must_use]
    pub const fn new(links: Vec<String>, completed: Vec<String>) -> Self {
        Self { links, completed }
    }
    /// 委譲する順序。
    #[must_use]
    pub fn links(&self) -> &[String] {
        &self.links
    }
    /// 完了した引継ぎ。
    #[must_use]
    pub fn completed(&self) -> &[String] {
        &self.completed
    }
}
