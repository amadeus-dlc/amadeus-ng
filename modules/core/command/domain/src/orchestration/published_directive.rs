//! ハーネスへ実際に発行する作業指示。
use crate::workflow_definition::StageSlug;
/// 発行記録に必要な指示の意味。全文や集約の写しは運ばない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishedDirective {
    /// 文脈が失効し、作業を許可しない指示。
    Error {
        /// 失効した段階。
        stage: StageSlug,
    },
    /// 段階の実行。
    RunStage {
        /// 対象段階。
        stage: StageSlug,
        /// 作業単位。stage-levelはNone。
        unit: Option<String>,
    },
    /// 規則の配送。
    LoadSteering {
        /// 対象段階。
        stage: StageSlug,
        /// 現在の部。
        part: u32,
        /// 全部数。
        parts: u32,
        /// そのまま渡す継続トークン。
        token: String,
    },
}
impl PublishedDirective {
    /// 指示の対象。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        match self {
            Self::RunStage { stage, .. }
            | Self::LoadSteering { stage, .. }
            | Self::Error { stage } => stage,
        }
    }
}
