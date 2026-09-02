//! `ScopeGrid` — コンパイル済み `scope-grid.json` の読取モデル。
//!
//! オンディスク形は `{ <scope>: { stages: { <slug>: "EXECUTE"|"SKIP" } } }` で、中間の
//! `"stages"` キーはレガシー `mapping[scope].stages` 互換のために省略できない
//! (レポート §6.1-2)。ただしそれは**ワイヤ構造**の話であり、ドメイン側は
//! 「scope 名 → (slug → `PlanAction`)」の 2 段写像だけを持つ。
//!
//! **3 値契約** (レポート §6.1-12): グリッド列に slug が無い場合は `None` を返す。
//! `SKIP` に畳まないこと — 「このグリッドがコンパイルしていないステージ」と
//! 「SKIP と宣言されたステージ」は別物で、畳み込みは呼出側の責務。

use std::collections::{BTreeMap, BTreeSet};

use super::plan_action::PlanAction;
use super::stage_graph::StageGraph;
use super::stage_slug::StageSlug;

/// scope 名 → (stage slug → `PlanAction`)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScopeGrid {
    columns: BTreeMap<String, BTreeMap<StageSlug, PlanAction>>,
}

impl ScopeGrid {
    /// 読み取った列写像をそのまま保持する。コンポーザが書いた列も**逐語**で受け、
    /// 転置で上書きしたり欠損セルを補完したりしない (再コンパイルは読取側の責務ではない)。
    #[must_use]
    pub const fn new(columns: BTreeMap<String, BTreeMap<StageSlug, PlanAction>>) -> ScopeGrid {
        ScopeGrid { columns }
    }

    /// `scope-grid.json` が読めないときの導出フォールバック (レポート §6.1-9)。
    ///
    /// 転置の述語は `phase == initialization || node.scopes.contains(scope)` —
    /// **initialization の 3 ステージは frontmatter に関係なく全列で EXECUTE** になる
    /// (レポート §3.2 の特例)。列に現れるスコープ名はノードが宣言したものの和集合で、
    /// 全ステージが明示的に EXECUTE か SKIP を持つ (`validateGrid` の要求と同形)。
    #[must_use]
    pub fn from_graph(graph: &StageGraph) -> ScopeGrid {
        let names: BTreeSet<&str> = graph.declared_scopes().into_iter().collect();
        let mut columns: BTreeMap<String, BTreeMap<StageSlug, PlanAction>> = BTreeMap::new();
        for name in names {
            let mut column: BTreeMap<StageSlug, PlanAction> = BTreeMap::new();
            for node in graph.nodes() {
                let action = if node.phase().is_always_in_plan() || node.declares_scope(name) {
                    PlanAction::Execute
                } else {
                    PlanAction::Skip
                };
                column.insert(node.slug().clone(), action);
            }
            columns.insert(name.to_string(), column);
        }
        ScopeGrid { columns }
    }

    /// 列を 1 つ足した (同名があれば差し替えた) 写し (scope 登記 —
    /// `CompiledDefinition::register_scope`)。
    #[must_use]
    pub fn with_column(
        mut self,
        scope: String,
        column: BTreeMap<StageSlug, PlanAction>,
    ) -> ScopeGrid {
        self.columns.insert(scope, column);
        self
    }

    /// グリッドが列を持つスコープ名 (辞書順)。
    ///
    /// **`validScopes()` ではない** — 有効スコープの権威はスコープ `.md` の存在であって
    /// グリッドではない (レポート §4.6)。`WorkflowDefinition::valid_scopes` を使うこと。
    #[must_use]
    pub fn scope_names(&self) -> Vec<&str> {
        self.columns.keys().map(String::as_str).collect()
    }

    /// グリッドが列を持つスコープか。**有効スコープの判定ではない** —
    /// 権威はスコープ `.md` の存在 (`WorkflowDefinition::is_valid_scope`)。
    #[must_use]
    pub fn contains_scope(&self, scope: &str) -> bool {
        self.columns.contains_key(scope)
    }

    /// スコープ 1 列 (slug → `PlanAction`)。列そのものが無ければ `None` — 空列とは区別する。
    #[must_use]
    pub fn column(&self, scope: &str) -> Option<&BTreeMap<StageSlug, PlanAction>> {
        self.columns.get(scope)
    }

    /// 全列。スコープ名の辞書順、列内は slug の辞書順。
    #[must_use]
    pub const fn columns(&self) -> &BTreeMap<String, BTreeMap<StageSlug, PlanAction>> {
        &self.columns
    }

    /// 3 値の静的プラン参照。列が無い / 列に slug が無い場合はどちらも `None`。
    #[must_use]
    pub fn action(&self, scope: &str, slug: &StageSlug) -> Option<PlanAction> {
        self.columns.get(scope)?.get(slug).copied()
    }

    /// 指定スコープ列で EXECUTE の slug 集合 (辞書順)。列が無ければ空。
    #[must_use]
    pub fn execute_slugs(&self, scope: &str) -> Vec<&StageSlug> {
        match self.columns.get(scope) {
            None => Vec::new(),
            Some(column) => column
                .iter()
                .filter(|(_, action)| **action == PlanAction::Execute)
                .map(|(slug, _)| slug)
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    // テストは固定長フィクスチャの添字参照を許容 (clippy.toml に相当設定が無いため file 単位で
    // allow)。
    #![allow(clippy::indexing_slicing)]

    use super::*;
    use crate::workflow_definition::{
        ExecutionKind, PhaseId, StageMode, StageNode, StageNodeBuilder, StageNumber,
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

    fn sample_graph() -> StageGraph {
        StageGraph::new(vec![
            node("bootstrap", "0.1", PhaseId::Initialization, &[]),
            node(
                "intent-capture",
                "1.1",
                PhaseId::Ideation,
                &["feature", "mvp"],
            ),
            node("threat-model", "2.1", PhaseId::Inception, &["feature"]),
            node("ops-runbook", "4.1", PhaseId::Operation, &[]),
        ])
        .unwrap()
    }

    #[test]
    fn transposition_puts_initialization_in_every_column() {
        let grid = ScopeGrid::from_graph(&sample_graph());
        assert_eq!(grid.scope_names(), vec!["feature", "mvp"]);
        for scope in ["feature", "mvp"] {
            assert_eq!(
                grid.action(scope, &slug("bootstrap")),
                Some(PlanAction::Execute),
                "initialization は {scope} 列でも EXECUTE"
            );
        }
        assert_eq!(
            grid.action("mvp", &slug("threat-model")),
            Some(PlanAction::Skip)
        );
        assert_eq!(
            grid.action("feature", &slug("threat-model")),
            Some(PlanAction::Execute)
        );
        // 宣言のないステージも明示的に SKIP が入る (欠損させない)
        assert_eq!(
            grid.action("feature", &slug("ops-runbook")),
            Some(PlanAction::Skip)
        );
    }

    #[test]
    fn missing_column_and_missing_slug_are_both_none_not_skip() {
        let grid = ScopeGrid::from_graph(&sample_graph());
        assert_eq!(grid.action("no-such-scope", &slug("bootstrap")), None);
        assert_eq!(grid.action("feature", &slug("no-such-stage")), None);
        assert!(!grid.contains_scope("no-such-scope"));
        assert!(grid.execute_slugs("no-such-scope").is_empty());
    }

    #[test]
    fn execute_slugs_lists_only_the_execute_cells() {
        let grid = ScopeGrid::from_graph(&sample_graph());
        let slugs: Vec<&str> = grid
            .execute_slugs("mvp")
            .into_iter()
            .map(StageSlug::as_str)
            .collect();
        assert_eq!(slugs, vec!["bootstrap", "intent-capture"]);
    }

    #[test]
    fn an_explicit_grid_is_taken_verbatim() {
        // コンポーザが書いた列は逐語で読む (レポート §6.1-20) — 転置で上書きしない
        let mut column = BTreeMap::new();
        column.insert(slug("bootstrap"), PlanAction::Skip);
        let mut columns = BTreeMap::new();
        columns.insert("composed".to_string(), column);
        let grid = ScopeGrid::new(columns);
        assert_eq!(
            grid.action("composed", &slug("bootstrap")),
            Some(PlanAction::Skip)
        );
        assert_eq!(grid.scope_names(), vec!["composed"]);
        assert_eq!(grid.columns().len(), 1);
    }

    proptest! {
        /// 転置の述語が全セルで成立する: EXECUTE ⟺ (initialization ∨ scope を宣言)。
        #[test]
        fn transposition_predicate_holds_for_every_cell(
            specs in proptest::collection::vec((0u32..5, proptest::collection::vec(0usize..3, 0..3)), 1..10),
        ) {
            let pool = ["alpha", "beta", "gamma"];
            let nodes: Vec<StageNode> = specs
                .iter()
                .enumerate()
                .map(|(i, (phase_index, scope_indices))| {
                    let phase = PhaseId::from_index(*phase_index).unwrap();
                    let scopes: Vec<&str> = scope_indices.iter().map(|&j| pool[j]).collect();
                    node(&format!("s{i}"), &format!("{phase_index}.{i}"), phase, &scopes)
                })
                .collect();
            let graph = StageGraph::new(nodes).unwrap();
            let grid = ScopeGrid::from_graph(&graph);

            // 列は「いずれかのノードが宣言したスコープ」の和集合と一致する
            prop_assert_eq!(grid.scope_names(), graph.declared_scopes());

            for scope in grid.scope_names() {
                for n in graph.nodes() {
                    let expected = if n.phase() == PhaseId::Initialization || n.declares_scope(scope) {
                        PlanAction::Execute
                    } else {
                        PlanAction::Skip
                    };
                    prop_assert_eq!(grid.action(scope, n.slug()), Some(expected));
                }
                // 全ステージが明示的な値を持つ (欠損セルなし)
                prop_assert_eq!(
                    grid.column(scope).map(BTreeMap::len),
                    Some(graph.len())
                );
            }
        }

        /// `execute_slugs` は列の EXECUTE セルとちょうど一致する。
        #[test]
        fn execute_slugs_matches_the_execute_cells(
            specs in proptest::collection::vec(proptest::collection::vec(0usize..2, 0..2), 1..8),
        ) {
            let pool = ["alpha", "beta"];
            let nodes: Vec<StageNode> = specs
                .iter()
                .enumerate()
                .map(|(i, scope_indices)| {
                    let scopes: Vec<&str> = scope_indices.iter().map(|&j| pool[j]).collect();
                    node(&format!("s{i}"), &format!("1.{i}"), PhaseId::Ideation, &scopes)
                })
                .collect();
            let graph = StageGraph::new(nodes).unwrap();
            let grid = ScopeGrid::from_graph(&graph);
            for scope in grid.scope_names() {
                let listed = grid.execute_slugs(scope);
                for s in &listed {
                    prop_assert_eq!(grid.action(scope, s), Some(PlanAction::Execute));
                }
                let expected = graph
                    .nodes()
                    .iter()
                    .filter(|n| grid.action(scope, n.slug()) == Some(PlanAction::Execute))
                    .count();
                prop_assert_eq!(listed.len(), expected);
            }
        }
    }
}
