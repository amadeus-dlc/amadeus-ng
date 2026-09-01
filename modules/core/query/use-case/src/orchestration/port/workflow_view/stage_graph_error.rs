//! `StageGraphError` — [`StageGraphView::new`] が拒否した構成違反。
//!
//! 運ぶのは**材料だけ** (重複した slug と両方の文書順位置) で、利用者向けの逐語文言は
//! 出す側が組む (`coding-rules/error-handling.md`)。
//!
//! [`StageGraphView::new`]: super::StageGraphView::new

use std::fmt;

/// `StageGraphView::new` が拒否する構成違反。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageGraphError {
    /// slug がグラフ内で重複している。
    DuplicateSlug {
        /// 重複した slug の生値。
        slug: String,
        /// 最初の出現の文書順インデックス。
        first_index: usize,
        /// 重複として拒否された出現の文書順インデックス。
        duplicate_index: usize,
    },
}

impl fmt::Display for StageGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StageGraphError::DuplicateSlug {
                slug,
                first_index,
                duplicate_index,
            } => write!(
                f,
                "duplicate stage slug {slug:?} at index {duplicate_index} (first seen at {first_index})"
            ),
        }
    }
}

impl std::error::Error for StageGraphError {}
