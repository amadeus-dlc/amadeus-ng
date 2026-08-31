//! `ScopeGridView` — コンパイル済み `scope-grid.json` のビュー。
//!
//! オンディスク形は `{ <scope>: { stages: { <slug>: "EXECUTE"|"SKIP" } } }` で、中間の
//! `"stages"` キーはレガシー `mapping[scope].stages` 互換のために省略できない (12 §6.1-2)。
//! ただしそれは**ワイヤ構造**の話であり、このビューは「scope 名 → (slug → `PlanActionView`)」
//! の 2 段写像だけを持つ。
//!
//! **3 値契約** (12 §6.1-12): グリッド列に slug が無い場合は `None` を返す。`SKIP` に
//! 畳まないこと — 「このグリッドがコンパイルしていないステージ」と「SKIP と宣言された
//! ステージ」は別物で、畳み込みは呼出側の責務。

use std::collections::{BTreeMap, BTreeSet};

use super::plan_action_view::PlanActionView;
use super::stage_graph_view::StageGraphView;
use super::stage_slug_view::StageSlugView;

/// scope 名 → (stage slug → `PlanActionView`)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScopeGridView {
    columns: BTreeMap<String, BTreeMap<StageSlugView, PlanActionView>>,
}

impl ScopeGridView {
    /// 読み取った列写像をそのまま保持する。コンポーザが書いた列も**逐語**で受け、転置で
    /// 上書きしたり欠損セルを補完したりしない (再コンパイルは読取側の責務ではない)。
    ///
    /// 構造体リテラルが現れるのはここ 1 か所で、[`ScopeGridView::from_graph`] も
    /// これへ委譲する (`coding-rules/factory-naming.md`)。
    #[must_use]
    pub const fn new(
        columns: BTreeMap<String, BTreeMap<StageSlugView, PlanActionView>>,
    ) -> ScopeGridView {
        ScopeGridView { columns }
    }

    /// `scope-grid.json` が読めないときの導出フォールバック (12 §6.1-9)。
    ///
    /// 転置の述語は `phase == initialization || node.scopes.contains(scope)` —
    /// **initialization のステージは frontmatter に関係なく全列で EXECUTE** になる
    /// (12 §3.2 の特例)。列に現れるスコープ名はノードが宣言したものの和集合で、全ステージが
    /// 明示的に EXECUTE か SKIP を持つ (`validateGrid` の要求と同形)。
    #[must_use]
    pub fn from_graph(graph: &StageGraphView) -> ScopeGridView {
        let names: BTreeSet<&str> = graph.declared_scopes().into_iter().collect();
        let mut columns: BTreeMap<String, BTreeMap<StageSlugView, PlanActionView>> =
            BTreeMap::new();
        for name in names {
            let mut column: BTreeMap<StageSlugView, PlanActionView> = BTreeMap::new();
            for node in graph.nodes() {
                let action = if node.phase().is_always_in_plan() || node.declares_scope(name) {
                    PlanActionView::Execute
                } else {
                    PlanActionView::Skip
                };
                column.insert(node.slug().clone(), action);
            }
            columns.insert(name.to_string(), column);
        }
        ScopeGridView::new(columns)
    }

    /// グリッドが列を持つスコープ名 (辞書順)。
    ///
    /// **`validScopes()` ではない** — 有効スコープの権威はスコープ `.md` の存在であって
    /// グリッドではない (12 §4.6)。[`super::DefinitionView::valid_scopes`] を使うこと。
    #[must_use]
    pub fn scope_names(&self) -> Vec<&str> {
        self.columns.keys().map(String::as_str).collect()
    }

    /// グリッドが列を持つスコープか。**有効スコープの判定ではない**。
    #[must_use]
    pub fn contains_scope(&self, scope: &str) -> bool {
        self.columns.contains_key(scope)
    }

    /// スコープ 1 列 (slug → `PlanActionView`)。列そのものが無ければ `None` — 空列とは区別する。
    #[must_use]
    pub fn column(&self, scope: &str) -> Option<&BTreeMap<StageSlugView, PlanActionView>> {
        self.columns.get(scope)
    }

    /// 全列。スコープ名の辞書順、列内は slug の辞書順。
    #[must_use]
    pub const fn columns(&self) -> &BTreeMap<String, BTreeMap<StageSlugView, PlanActionView>> {
        &self.columns
    }

    /// 3 値の静的プラン参照。列が無い / 列に slug が無い場合はどちらも `None`。
    #[must_use]
    pub fn action(&self, scope: &str, slug: &StageSlugView) -> Option<PlanActionView> {
        self.columns.get(scope)?.get(slug).copied()
    }

    /// 指定スコープ列で EXECUTE の slug 集合 (辞書順)。列が無ければ空。
    #[must_use]
    pub fn execute_slugs(&self, scope: &str) -> Vec<&StageSlugView> {
        match self.columns.get(scope) {
            None => Vec::new(),
            Some(column) => column
                .iter()
                .filter(|(_, action)| **action == PlanActionView::Execute)
                .map(|(slug, _)| slug)
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::{
        ExecutionKindView, PhaseView, StageModeView, StageNumberView, StageView, StageViewBuilder,
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

    fn sample_graph() -> StageGraphView {
        StageGraphView::new(vec![
            node("bootstrap", "0.1", PhaseView::Initialization, &[]),
            node("intent-capture", "1.1", PhaseView::Ideation, &["feature"]),
            node("code-generation", "3.1", PhaseView::Construction, &["mvp"]),
        ])
        .unwrap()
    }

    #[test]
    fn the_transpose_puts_initialization_in_every_column() {
        let grid = ScopeGridView::from_graph(&sample_graph());
        assert_eq!(grid.scope_names(), vec!["feature", "mvp"]);
        for scope in ["feature", "mvp"] {
            assert_eq!(
                grid.action(scope, &slug("bootstrap")),
                Some(PlanActionView::Execute),
                "{scope}"
            );
        }
        assert_eq!(
            grid.action("feature", &slug("code-generation")),
            Some(PlanActionView::Skip)
        );
        assert_eq!(
            grid.action("mvp", &slug("code-generation")),
            Some(PlanActionView::Execute)
        );
    }

    #[test]
    fn a_missing_column_or_cell_is_the_third_value_not_skip() {
        let grid = ScopeGridView::from_graph(&sample_graph());
        assert_eq!(grid.action("ghost", &slug("bootstrap")), None);
        assert_eq!(grid.action("feature", &slug("nowhere")), None);
        assert!(!grid.contains_scope("ghost"));
        assert!(grid.contains_scope("feature"));
        assert_eq!(grid.column("ghost"), None);
        assert_eq!(grid.column("feature").map(BTreeMap::len), Some(3));
    }

    #[test]
    fn execute_slugs_lists_only_the_execute_cells_in_slug_order() {
        let grid = ScopeGridView::from_graph(&sample_graph());
        let executed: Vec<&str> = grid
            .execute_slugs("mvp")
            .iter()
            .map(|s| s.as_str())
            .collect();
        assert_eq!(executed, vec!["bootstrap", "code-generation"]);
        assert!(grid.execute_slugs("ghost").is_empty());
    }

    #[test]
    fn a_grid_read_from_disk_is_kept_verbatim_without_transposing_over_it() {
        let mut column: BTreeMap<StageSlugView, PlanActionView> = BTreeMap::new();
        column.insert(slug("bootstrap"), PlanActionView::Skip);
        let mut columns = BTreeMap::new();
        columns.insert("feature".to_string(), column);
        let grid = ScopeGridView::new(columns);
        // 転置導出なら initialization は EXECUTE になるが、読んだ列は逐語で保つ。
        assert_eq!(
            grid.action("feature", &slug("bootstrap")),
            Some(PlanActionView::Skip)
        );
        assert_eq!(grid.columns().len(), 1);
    }
}
