//! jump時に観測した宣言成果物。
use crate::workflow_definition::StageSlug;
/// パスとその時点の存在・Review節の観測。状態遷移の判断は含まない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JumpArtifact {
    stage: StageSlug,
    path: String,
    exists: bool,
    review: bool,
}
impl JumpArtifact {
    /// 宣言された成果物の観測を完全に束ねる。
    #[must_use]
    pub const fn new(stage: StageSlug, path: String, exists: bool, review: bool) -> Self {
        Self {
            stage,
            path,
            exists,
            review,
        }
    }
    /// 保存・投影境界へ所属stageを渡す。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }
    /// 保存・投影境界へ公開パスを渡す。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }
    /// 観測時に存在していたか。
    #[must_use]
    pub const fn exists(&self) -> bool {
        self.exists
    }
    /// 観測時にReview節があったか。
    #[must_use]
    pub const fn has_review(&self) -> bool {
        self.review
    }
}
