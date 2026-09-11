//! Pipelineの試行境界と、その後に受領したlink。
use chrono::{DateTime, Utc};
/// 試行と受領を時系列順に保持する値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineRecord {
    /// 単独試行が完了したstage。
    Closed(String),
    /// あるステージ（Noneは通常実行全体）の試行が始まった。
    Boundary {
        /// 対象ステージ。
        stage: Option<String>,

        /// 独立実行の境界か。
        single: bool,

        /// 境界の発生時刻。
        at: DateTime<Utc>,
    },
    /// 実行集約がlinkを受理した。
    Completed(super::PipelineLinkCompleted),
}
