//! 成果物の再利用受領を拒否した理由。公開文言は出力側で組む。
/// 受領の前提を満たさなかった材料。
#[derive(Debug)]
pub enum ArtifactReuseError {
    /// ステージが空。
    StageRequired,
    /// 成果物の指定が空。
    ArtifactsRequired,
    /// 決定が閉集合（`keep` / `modify` / `redo`）の外。
    InvalidDecision {
        /// 指定された決定。
        given: String,
    },
    /// 受領が名指した段が定義グラフに無い。
    UnknownStage {
        /// 指定された段の slug。
        slug: String,
    },
    /// 実行集約による拒否。
    Command(super::CommandError),
}
impl std::fmt::Display for ArtifactReuseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "artifact reuse: {self:?}")
    }
}
impl std::error::Error for ArtifactReuseError {}
