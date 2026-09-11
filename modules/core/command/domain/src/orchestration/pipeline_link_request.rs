//! 完了したpipeline linkを記録する要求。
/// 構文境界の材料。状態依存の受理判断は実行集約が行う。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineLinkRequest {
    stage: String,
    link: String,
    repo: Option<String>,
    single: bool,
    handoff: super::PipelineHandoffInput,
}
impl PipelineLinkRequest {
    /// 入力を損失なく束ねる。
    #[must_use]
    pub const fn new(
        stage: String,
        link: String,
        repo: Option<String>,
        single: bool,
        handoff: super::PipelineHandoffInput,
    ) -> Self {
        Self {
            stage,
            link,
            repo,
            single,
            handoff,
        }
    }
    /// 指定されたステージ。
    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }
    /// 完了したlink名。
    #[must_use]
    pub fn link(&self) -> &str {
        &self.link
    }
    /// repoの生指定。空と省略を区別する。
    #[must_use]
    pub fn repo(&self) -> Option<&str> {
        self.repo.as_deref()
    }
    /// 独立実行の受領か。
    #[must_use]
    pub const fn is_single(&self) -> bool {
        self.single
    }
    pub(super) const fn handoff(&self) -> &super::PipelineHandoffInput {
        &self.handoff
    }
}
