//! `StageGraphError` — `StageGraph` の構築が拒否する形。

/// `StageGraph::new` が拒否する構成違反。
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
