//! `StageGraph` — コンパイル済み `stage-graph.json` (ルートは**配列**) の読取モデル。
//!
//! **文書順を保持する** (レポート §6.1-6)。`stages_in_scope` と集約側の前進走査は文書順で、
//! `subgraph_for_scope` だけが数値順にソートする — この 2 経路の使い分けを潰さないため、
//! ロード時に数値順へ正規化してはならない。
//!
//! slug → 文書順インデックスの索引は**実装の自由** (レポート §6.2)。upstream の線形
//! スキャンを模倣する必要はない。

use std::collections::{BTreeMap, BTreeSet};

use super::stage_graph_error::StageGraphError;
use super::stage_node::StageNode;
use super::stage_slug::StageSlug;

/// 文書順のノード列 + slug 索引。slug 一意性は構成時に検証される (01 §8.4 #3)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageGraph {
    nodes: Vec<StageNode>,
    index: BTreeMap<StageSlug, usize>,
}

impl StageGraph {
    /// 条件に一致するノードを文書順で保持し、索引を振り直す。
    #[must_use]
    pub fn filter(&self, mut predicate: impl FnMut(&StageNode) -> bool) -> Self {
        let nodes: Vec<StageNode> = self
            .nodes
            .iter()
            .filter(|node| predicate(node))
            .cloned()
            .collect();
        let index = nodes
            .iter()
            .enumerate()
            .map(|(position, node)| (node.slug().clone(), position))
            .collect();
        Self { nodes, index }
    }

    /// ノードを変換し、文書順を保って新しいグラフを検証する。
    ///
    /// # Errors
    /// 変換後のslugが重複した場合はDuplicateSlug。
    pub fn map(
        &self,
        transform: impl FnMut(&StageNode) -> StageNode,
    ) -> Result<Self, StageGraphError> {
        Self::new(self.nodes.iter().map(transform).collect())
    }

    /// 文書順で左から畳み込む。空なら初期値を返す。
    pub fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, &'a StageNode) -> A) -> A {
        self.nodes.iter().fold(initial, fold)
    }

    /// 文書順のノード列からグラフを構成する。
    ///
    /// # Errors
    ///
    /// slug が重複していれば `DuplicateSlug` (最初の出現位置と重複位置を添える)。
    pub fn new(nodes: Vec<StageNode>) -> Result<StageGraph, StageGraphError> {
        let mut index: BTreeMap<StageSlug, usize> = BTreeMap::new();
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
        Ok(StageGraph { nodes, index })
    }

    /// ノードが宣言するプラグイン名の集合 (`plugin` を持つノードだけ)。
    #[must_use]
    pub fn declared_plugins(&self) -> BTreeSet<&str> {
        self.nodes.iter().filter_map(StageNode::plugin).collect()
    }

    /// プラグイン選択を反映した写し (upstream `applyPluginSelection` の意味論)。
    ///
    /// plugin 所属ノードの `enabled` を一度落とし、選択に**無い**プラグインのノードだけ
    /// `false` を立てる。非プラグインノードは触らない。slug 集合は変わらないので索引は
    /// そのまま写す (不変条件は構築時に検査済み)。
    #[must_use]
    pub fn with_plugin_selection(&self, enabled_plugins: &BTreeSet<String>) -> StageGraph {
        let nodes = self
            .nodes
            .iter()
            .map(|node| match node.plugin() {
                None => node.clone(),
                Some(plugin) if enabled_plugins.contains(plugin) => node.clone().with_enabled(None),
                Some(_) => node.clone().with_enabled(Some(false)),
            })
            .collect();
        StageGraph {
            nodes,
            index: self.index.clone(),
        }
    }

    /// **文書順**のノード列 (`stage-graph.json` の配列順そのもの)。
    #[must_use]
    pub fn nodes(&self) -> &[StageNode] {
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
    pub fn index_of(&self, slug: &StageSlug) -> Option<usize> {
        self.index.get(slug).copied()
    }

    /// slug に対応するノード。slug は一意なので候補は高々 1 件。グラフに無ければ `None`。
    #[must_use]
    pub fn get(&self, slug: &StageSlug) -> Option<&StageNode> {
        self.index_of(slug).and_then(|i| self.nodes.get(i))
    }

    /// 文書順インデックスのノード (`nodes()` と同じ並び)。範囲外は `None`。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&StageNode> {
        self.nodes.get(index)
    }

    /// `numericStageOrder` 昇順のノード参照列 (文書順は破壊しない)。
    ///
    /// 番号が同値のノードは文書順を保つ (安定ソート)。
    #[must_use]
    pub fn numeric_order(&self) -> Vec<&StageNode> {
        let mut ordered: Vec<&StageNode> = self.nodes.iter().collect();
        // numericStageOrder = 整数比較のみ。sort_by は安定ソートなので、
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

impl core_infrastructure::collections::FirstClassCollection for StageGraph {
    type Item<'a> = &'a StageNode;
    type Filtered = Self;
    fn len(&self) -> usize {
        Self::len(self)
    }
    fn at(&self, index: usize) -> Option<&StageNode> {
        Self::at(self, index)
    }
    fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, &'a StageNode) -> A) -> A {
        Self::fold_left(self, initial, fold)
    }
    fn filter(&self, predicate: impl FnMut(&StageNode) -> bool) -> Self {
        Self::filter(self, predicate)
    }
}

#[cfg(test)]
mod tests {
    // テストは固定長フィクスチャの添字参照を許容 (clippy.toml に相当設定が無いため file 単位で
    // allow)。
    #![allow(clippy::indexing_slicing)]

    use super::*;
    use crate::workflow_definition::{
        ExecutionKind, PhaseId, StageMode, StageNodeBuilder, StageNumber,
    };
    use proptest::prelude::*;

    fn node(slug: &str, number: &str, phase: PhaseId, scopes: &[&str]) -> StageNode {
        StageNodeBuilder::new(
            StageSlug::parse(slug).unwrap(),
            StageNumber::parse(number).unwrap(),
            slug.to_string(),
            phase,
            ExecutionKind::Always,
            StageMode::Inline,
        )
        .scopes(scopes.iter().map(|s| (*s).to_string()).collect())
        .build()
    }

    fn slug(s: &str) -> StageSlug {
        StageSlug::parse(s).unwrap()
    }

    #[test]
    fn collection_operations_rebuild_indices_and_reject_mapping_collisions() {
        let graph = StageGraph::new(vec![
            node("b", "1.2", PhaseId::Inception, &[]),
            node("a", "1.1", PhaseId::Inception, &[]),
        ])
        .unwrap();
        let selected = graph.filter(|node| node.slug() == &slug("a"));
        assert_eq!(selected.index_of(&slug("a")), Some(0));
        assert_eq!(selected.at(0).unwrap().slug(), &slug("a"));
        assert_eq!(
            graph.fold_left(String::new(), |acc, node| acc + node.slug().as_str()),
            "ba"
        );
        let mapped = graph
            .map(|node| node.clone().with_enabled(Some(false)))
            .unwrap();
        assert_eq!(mapped.at(0).unwrap().enabled(), Some(false));
        assert_eq!(graph.at(0).unwrap().enabled(), None);
        assert!(
            graph
                .map(|_| node("same", "1.1", PhaseId::Inception, &[]))
                .is_err()
        );
        let empty = graph.filter(|_| false);
        assert!(empty.is_empty());
        let count = |acc, _: &StageNode| acc + 1;
        assert_eq!(empty.fold_left(5, count), 5);
        assert_eq!(graph.fold_left(5, count), 7);
        assert!(empty.map(Clone::clone).unwrap().is_empty());
    }

    #[test]
    fn document_order_survives_construction_even_when_it_is_not_numeric_order() {
        let graph = StageGraph::new(vec![
            node("c", "1.10", PhaseId::Ideation, &[]),
            node("a", "0.1", PhaseId::Initialization, &[]),
            node("b", "1.9", PhaseId::Ideation, &[]),
        ])
        .unwrap();
        // 文書順は入力のまま
        let doc: Vec<&str> = graph.nodes().iter().map(|n| n.slug().as_str()).collect();
        assert_eq!(doc, vec!["c", "a", "b"]);
        // 数値順は "1.10" > "1.9" (辞書順ではない)
        let numeric: Vec<&str> = graph
            .numeric_order()
            .iter()
            .map(|n| n.slug().as_str())
            .collect();
        assert_eq!(numeric, vec!["a", "b", "c"]);
    }

    #[test]
    fn numeric_order_keeps_document_order_for_numerically_equal_numbers() {
        // 前置ゼロ違い ("1.01" と "1.1") は numericStageOrder で同値 — upstream の
        // 安定ソートと同じく文書順が残る (生表現の辞書順 "1.01" < "1.1" に並べ替えない)
        let graph = StageGraph::new(vec![
            node("first", "1.1", PhaseId::Ideation, &[]),
            node("second", "1.01", PhaseId::Ideation, &[]),
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
        let graph = StageGraph::new(vec![
            node("c", "1.10", PhaseId::Ideation, &[]),
            node("a", "0.1", PhaseId::Initialization, &[]),
        ])
        .unwrap();
        assert_eq!(graph.index_of(&slug("c")), Some(0));
        assert_eq!(graph.index_of(&slug("a")), Some(1));
        assert_eq!(graph.index_of(&slug("zzz")), None);
        assert_eq!(graph.get(&slug("a")).unwrap().number().as_str(), "0.1");
        assert_eq!(graph.at(0).unwrap().slug().as_str(), "c");
        assert_eq!(graph.len(), 2);
        assert!(!graph.is_empty());
    }

    #[test]
    fn duplicate_slugs_are_refused_with_both_positions() {
        let err = StageGraph::new(vec![
            node("a", "0.1", PhaseId::Initialization, &[]),
            node("b", "1.1", PhaseId::Ideation, &[]),
            node("a", "1.2", PhaseId::Ideation, &[]),
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
    }

    #[test]
    fn declared_scopes_is_the_sorted_union_over_nodes() {
        let graph = StageGraph::new(vec![
            node("a", "0.1", PhaseId::Initialization, &[]),
            node("b", "1.1", PhaseId::Ideation, &["mvp", "feature"]),
            node("c", "1.2", PhaseId::Ideation, &["feature"]),
        ])
        .unwrap();
        assert_eq!(graph.declared_scopes(), vec!["feature", "mvp"]);
    }

    proptest! {
        /// 一意な slug の列は必ず構成でき、索引は文書順の位置と一致する。
        #[test]
        fn indices_agree_with_document_positions(count in 1usize..12) {
            let nodes: Vec<StageNode> = (0..count)
                .map(|i| node(&format!("s{i}"), &format!("{}.{}", i % 5, i), PhaseId::Ideation, &[]))
                .collect();
            let graph = StageGraph::new(nodes).unwrap();
            for i in 0..count {
                let s = slug(&format!("s{i}"));
                prop_assert_eq!(graph.index_of(&s), Some(i));
                prop_assert_eq!(graph.at(i).unwrap().slug(), &s);
            }
        }

        /// `numeric_order` は文書順を破壊せず、必ず番号昇順で同じ多重集合を返す。
        #[test]
        fn numeric_order_is_a_sorted_permutation_of_document_order(
            numbers in proptest::collection::vec((0u32..5, 0u32..30), 1..12),
        ) {
            let nodes: Vec<StageNode> = numbers
                .iter()
                .enumerate()
                .map(|(i, (p, s))| node(&format!("s{i}"), &format!("{p}.{s}"), PhaseId::Ideation, &[]))
                .collect();
            let graph = StageGraph::new(nodes).unwrap();
            let ordered = graph.numeric_order();
            prop_assert_eq!(ordered.len(), graph.len());
            for w in ordered.windows(2) {
                prop_assert!(w[0].number() <= w[1].number());
            }
            let mut slugs: Vec<&str> = ordered.iter().map(|n| n.slug().as_str()).collect();
            slugs.sort_unstable();
            let mut doc: Vec<&str> = graph.nodes().iter().map(|n| n.slug().as_str()).collect();
            doc.sort_unstable();
            prop_assert_eq!(slugs, doc);
            // 文書順そのものは不変
            let after: Vec<&str> = graph.nodes().iter().map(|n| n.slug().as_str()).collect();
            prop_assert_eq!(after.len(), numbers.len());
        }
    }
}
