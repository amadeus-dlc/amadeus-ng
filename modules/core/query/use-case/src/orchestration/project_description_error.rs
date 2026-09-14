//! 依頼原文の正本を解決できなかった — 材料だけを運ぶ拒否。
//!
//! 「読めなかった」だけを語る [`ReadModelReadError`] と違い、こちらは**読めた 2 つの面の
//! 取り合わせが成立しない**ことを語る (正本を名指しているのにサイドカーが無い、など)。
//! どちらも運ぶのは材料だけで、利用者向けの逐語文言は出す側 (プレゼンタ) が組む
//! (`coding-rules/error-handling.md`)。

use core::fmt;
use std::error::Error;

use crate::orchestration::ReadModelReadError;

/// 依頼原文の正本を解決できなかった。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectDescriptionError {
    /// 状態ファイルが無い (record がまだ無い / 状態ファイルが書かれていない)。
    StateFileAbsent,
    /// 状態ファイルを引けなかった。
    StateFileUnreadable(ReadModelReadError),
    /// 正本の名指しが無い legacy record なのに `Project` 欄が無い。
    MissingProjectField,
    /// `Project Description Source` がこの build の知らない綴りを名指している (材料 = その綴り)。
    UnsupportedSource(String),
    /// 状態ファイルがサイドカーを名指しているのに、そのサイドカーが無い。
    SidecarMissing,
    /// サイドカーを引けなかった (通常ファイルでない場合を含む)。
    SidecarUnreadable(ReadModelReadError),
    /// サイドカーが JSON として読めない (材料 = 読取器が返した原因)。
    SidecarNotJson(String),
    /// サイドカーは JSON だが、1 個の文字列ではない。
    SidecarNotAString,
}

impl fmt::Display for ProjectDescriptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectDescriptionError::StateFileAbsent => write!(f, "state file absent"),
            ProjectDescriptionError::StateFileUnreadable(cause) => {
                write!(f, "state file unreadable: {cause}")
            }
            ProjectDescriptionError::MissingProjectField => write!(f, "missing Project field"),
            ProjectDescriptionError::UnsupportedSource(source) => {
                write!(f, "unsupported source: {source}")
            }
            ProjectDescriptionError::SidecarMissing => write!(f, "sidecar missing"),
            ProjectDescriptionError::SidecarUnreadable(cause) => {
                write!(f, "sidecar unreadable: {cause}")
            }
            ProjectDescriptionError::SidecarNotJson(cause) => {
                write!(f, "sidecar is not JSON: {cause}")
            }
            ProjectDescriptionError::SidecarNotAString => {
                write!(f, "sidecar is not a single JSON string")
            }
        }
    }
}

impl Error for ProjectDescriptionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ProjectDescriptionError::StateFileUnreadable(cause)
            | ProjectDescriptionError::SidecarUnreadable(cause) => Some(cause),
            _ => None,
        }
    }
}
