//! `StageGraphView` — コンパイル済み `stage-graph.json` (ルートは**配列**) のビュー。
//!
//! **文書順を保持する** (12 §6.1-6)。`stages_in_scope` の走査は文書順で、
//! `subgraph_for_scope` だけが数値順にソートする — この 2 経路の使い分けを潰さないため、
//! 読込時に数値順へ正規化してはならない。
//!
//! slug → 文書順インデックスの索引は**実装の自由** (12 §6.2)。

use std::collections::BTreeMap;
use std::fmt;

use super::stage_slug_view::StageSlugView;
use super::stage_view::StageView;

/// 文書順のノード列 + slug 索引。slug 一意性は構成時に検証される (01 §8.4 #3)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageGraphView {
    nodes: Vec<StageView>,
    index: BTreeMap<StageSlugView, usize>,
}

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

impl StageGraphView {
    /// 文書順のノード列からグラフを構成する。
    ///
    /// # Errors
    ///
    /// slug が重複していれば `DuplicateSlug` (最初の出現位置と重複位置を添える)。
    pub fn new(nodes: Vec<StageView>) -> Result<StageGraphView, StageGraphError> {
        let mut index: BTreeMap<StageSlugView, usize> = BTreeMap::new();
        for (position, node) in nodes.iter().enumerate() {
            if let Some(&first_index) = index.get(node.slug()) {
                return Err(StageGraphError::DuplicateSlug {
                    slug: node.slug().as_str().to_string(),
                    first_index,
                    duplicate_index: position,
                });
            }
            index.insert(node.slug().clone(), position);
        }
        Ok(StageGraphView { nodes, index })
    }

    /// **文書順**のノード列 (`stage-graph.json` の配列順そのもの)。
    #[must_use]
    pub fn nodes(&self) -> &[StageView] {
        &self.nodes
    }

    /// グラフが持つステージ数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.nodes.len()
    }

    /// ノードを 1 つも持たないか。空グラフは正当 (`new` は空列を拒否しない)。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// 文書順インデックス。
    #[must_use]
    pub fn index_of(&self, slug: &StageSlugView) -> Option<usize> {
        self.index.get(slug).copied()
    }

    /// slug に対応するノード。slug は一意なので候補は高々 1 件。
    #[must_use]
    pub fn get(&self, slug: &StageSlugView) -> Option<&StageView> {
        self.index_of(slug).and_then(|i| self.nodes.get(i))
    }

    /// 文書順インデックスのノード (`nodes()` と同じ並び)。範囲外は `None`。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&StageView> {
        self.nodes.get(index)
    }

    /// `numericStageOrder` 昇順のノード参照列 (文書順は破壊しない)。
    ///
    /// 番号が同値のノードは文書順を保つ (安定ソート)。
    #[must_use]
    pub fn numeric_order(&self) -> Vec<&StageView> {
        let mut ordered: Vec<&StageView> = self.nodes.iter().collect();
        // numericStageOrder = 整数比較のみ。`sort_by` は安定ソートなので、
        // 前置ゼロ違いの同値は upstream 同様に文書順が残る。
        ordered.sort_by(|a, b| a.number().numeric_cmp(b.number()));
        ordered
    }

    /// 全ノードが宣言しているスコープ名の和集合 (辞書順)。
    #[must_use]
    pub fn declared_scopes(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .nodes
            .iter()
            .flat_map(|n| n.scopes().iter().map(String::as_str))
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::{
        ExecutionKindView, PhaseView, StageModeView, StageNumberView, StageViewBuilder,
    };

    fn node(slug: &str, number: &str, phase: PhaseView, scopes: &[&str]) -> StageView {
        StageViewBuilder::new(
            StageSlugView::parse(slug).unwrap(),
            StageNumberView::parse(number).unwrap(),
            slug.to_string(),
            phase,
            ExecutionKindView::Always,
            StageModeView::Inline,
        )
        .with_scopes(scopes.iter().map(|s| (*s).to_string()).collect())
        .build()
    }

    fn slug(s: &str) -> StageSlugView {
        StageSlugView::parse(s).unwrap()
    }

    #[test]
    fn document_order_survives_construction_even_when_it_is_not_numeric_order() {
        let graph = StageGraphView::new(vec![
            node("c", "1.10", PhaseView::Ideation, &[]),
            node("a", "0.1", PhaseView::Initialization, &[]),
            node("b", "1.9", PhaseView::Ideation, &[]),
        ])
        .unwrap();
        let doc: Vec<&str> = graph.nodes().iter().map(|n| n.slug().as_str()).collect();
        assert_eq!(doc, vec!["c", "a", "b"]);
        let numeric: Vec<&str> = graph
            .numeric_order()
            .iter()
            .map(|n| n.slug().as_str())
            .collect();
        assert_eq!(numeric, vec!["a", "b", "c"]);
    }

    #[test]
    fn numeric_order_keeps_document_order_for_numerically_equal_numbers() {
        let graph = StageGraphView::new(vec![
            node("first", "1.1", PhaseView::Ideation, &[]),
            node("second", "1.01", PhaseView::Ideation, &[]),
        ])
        .unwrap();
        let numeric: Vec<&str> = graph
            .numeric_order()
            .iter()
            .map(|n| n.slug().as_str())
            .collect();
        assert_eq!(numeric, vec!["first", "second"]);
    }

    #[test]
    fn slug_lookup_yields_document_order_indices() {
        let graph = StageGraphView::new(vec![
            node("c", "1.10", PhaseView::Ideation, &[]),
            node("a", "0.1", PhaseView::Initialization, &[]),
        ])
        .unwrap();
        assert_eq!(graph.index_of(&slug("c")), Some(0));
        assert_eq!(graph.index_of(&slug("a")), Some(1));
        assert_eq!(graph.index_of(&slug("zzz")), None);
        assert_eq!(graph.get(&slug("a")).unwrap().number().as_str(), "0.1");
        assert_eq!(graph.at(0).unwrap().slug().as_str(), "c");
        assert_eq!(graph.at(9), None);
        assert_eq!(graph.len(), 2);
        assert!(!graph.is_empty());
    }

    #[test]
    fn duplicate_slugs_are_refused_with_both_positions() {
        let err = StageGraphView::new(vec![
            node("a", "0.1", PhaseView::Initialization, &[]),
            node("b", "1.1", PhaseView::Ideation, &[]),
            node("a", "1.2", PhaseView::Ideation, &[]),
        ])
        .unwrap_err();
        assert_eq!(
            err,
            StageGraphError::DuplicateSlug {
                slug: "a".to_string(),
                first_index: 0,
                duplicate_index: 2,
            }
        );
        assert_eq!(
            err.to_string(),
            "duplicate stage slug \"a\" at index 2 (first seen at 0)"
        );
    }

    #[test]
    fn declared_scopes_is_the_sorted_union_over_nodes() {
        let graph = StageGraphView::new(vec![
            node("a", "0.1", PhaseView::Initialization, &[]),
            node("b", "1.1", PhaseView::Ideation, &["mvp", "feature"]),
            node("c", "1.2", PhaseView::Ideation, &["feature"]),
        ])
        .unwrap();
        assert_eq!(graph.declared_scopes(), vec!["feature", "mvp"]);
    }
}
