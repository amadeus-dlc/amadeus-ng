//! jump時の外部観測。再投影時にファイルを再読しない。
use super::{JumpArtifact, SourceBaseline};
/// ソース比較基準と宣言成果物の観測を保持する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JumpObservation {
    baseline: SourceBaseline,
    artifacts: Vec<JumpArtifact>,
}
impl JumpObservation {
    /// 観測を完全に束ねる。
    #[must_use]
    pub const fn new(baseline: SourceBaseline, artifacts: Vec<JumpArtifact>) -> Self {
        Self {
            baseline,
            artifacts,
        }
    }
    /// 保存・投影境界へ比較基準を渡す。
    #[must_use]
    pub const fn baseline(&self) -> &SourceBaseline {
        &self.baseline
    }
    /// 保存・投影境界でドメイン要素のまま畳み込む。
    pub fn fold_artifacts<T>(&self, initial: T, fold: impl FnMut(T, &JumpArtifact) -> T) -> T {
        self.artifacts.iter().fold(initial, fold)
    }
}
