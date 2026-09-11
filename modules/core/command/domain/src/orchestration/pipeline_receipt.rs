//! 完了したpipeline linkの受領内容。
/// 保存と監査へ渡す、受理時点に確定した事実。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineReceipt {
    stage: String,
    link: String,
    repo: Option<String>,
    single: bool,
    position: usize,
    total: usize,
    handoff: Option<super::PipelineHandoff>,
}
impl PipelineReceipt {
    /// 有効な位置と全材料から構築する。
    /// # Errors
    /// 位置が宣言範囲にない場合。
    pub fn new(
        stage: String,
        link: String,
        repo: Option<String>,
        single: bool,
        position: usize,
        total: usize,
        handoff: Option<super::PipelineHandoff>,
    ) -> Result<Self, super::PipelineLinkError> {
        if stage.is_empty() || link.is_empty() || position == 0 || position > total {
            return Err(super::PipelineLinkError::InvalidHandoff);
        }
        Ok(Self {
            stage,
            link,
            repo,
            single,
            position,
            total,
            handoff,
        })
    }
    /// 保存境界のステージ。
    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }
    /// 保存境界のlink。
    #[must_use]
    pub fn link(&self) -> &str {
        &self.link
    }
    /// 保存境界のrepo。
    #[must_use]
    pub fn repo(&self) -> Option<&str> {
        self.repo.as_deref()
    }
    /// 独立実行か。
    #[must_use]
    pub const fn is_single(&self) -> bool {
        self.single
    }
    /// 宣言中の位置（1始まり）。
    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }
    /// 宣言されたlink数。
    #[must_use]
    pub const fn total(&self) -> usize {
        self.total
    }
    /// 受理した引継ぎファイル。
    #[must_use]
    pub const fn handoff(&self) -> Option<&super::PipelineHandoff> {
        self.handoff.as_ref()
    }
}
