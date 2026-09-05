//! Intent に基づくレビュー方針解決の拒否材料。
use std::fmt;

/// 計画と定義からレビュー方針を決められない理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentReviewError {
    /// 定義が対象ステージを持たない。
    UnknownStage,
    /// 履歴に記録された override が既知の値でない。
    InvalidOverride(String),
    /// 渡された定義が intent の参照先と異なる。
    DefinitionMismatch,
}
impl fmt::Display for IntentReviewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownStage => f.write_str("unknown definition stage"),
            Self::InvalidOverride(value) => write!(f, "invalid review override: {value}"),
            Self::DefinitionMismatch => f.write_str("definition mismatch"),
        }
    }
}
impl std::error::Error for IntentReviewError {}
