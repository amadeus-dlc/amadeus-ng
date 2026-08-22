//! `WorkflowDefinition` — Published Language (`stage-graph.json` / `scope-grid.json` /
//! `scopes/*.md`) の**読取モデル**。「何を実行しうるか」の静的定義を 1 つの集約にまとめ、
//! orchestration が依存する 5 述語を純関数として提供する (01 §3.1 / 10 §3)。
//!
//! # 観測可能契約 (レポート §6.1 — 逸脱台帳行き)
//!
//! - **未知スコープの非対称**: `subgraph_for_scope` だけが `Err(UnknownScope)`。
//!   `next_in_scope_stage` / `first_in_scope_stage_of_phase` / `stages_in_scope` は
//!   同じ未知スコープに対して `None` / 空を返す。
//! - **`.md` あり × グリッド列なし** = zero-EXECUTE な**正当**スコープ (エラーにしない)。
//! - **グリッド列あり × `.md` なし** = ランタイムから不可視 (有効スコープの権威は `.md`)。
//! - **グリッドに slug が無い** = `None`。`SKIP` に畳まない (3 値契約)。
//! - **文書順の保持**: `next_in_scope_stage` は文書順で前進走査し、`subgraph_for_scope`
//!   だけが数値順にソートする。2 経路の使い分けを潰さない。
//!
//! `enabled: false` のノードは**除外しない**。意味論が未確定 (レポート §7) のため、
//! 読取モデルは `StageNode::is_enabled()` を露出するだけで判断は呼出側に委ねる。

use std::collections::BTreeMap;

use super::phase::PhaseId;
use super::scope_grid::ScopeGrid;
use super::scope_metadata::ScopeMetadata;
use super::stage_graph::StageGraph;
use super::stage_node::StageNode;
use super::stage_slug::StageSlug;
use crate::orchestration::plan_action::PlanAction;
use crate::workspace::checkbox::CheckboxState;

/// `validScopes()` に無いスコープ名。
///
/// upstream の逐語文言 `Unknown scope: "<scope>". Valid scopes: <csv>` を組み立てるのに
/// 必要な材料をそのまま保持する (文言化は文言カタログ側の責務)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownScope {
    scope: String,
    /// 有効スコープ名 (辞書順)。
    valid_scopes: Vec<String>,
}

impl UnknownScope {
    /// 拒否されたスコープ名と、拒否時点の有効スコープ一覧 (辞書順) を束ねる。
    /// どちらも生値のまま保持する。
    #[must_use]
    pub fn new(scope: impl Into<String>, valid_scopes: Vec<String>) -> UnknownScope {
        UnknownScope {
            scope: scope.into(),
            valid_scopes,
        }
    }

    /// 拒否されたスコープ名。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// 有効スコープ名 (辞書順)。
    #[must_use]
    pub fn valid_scopes(&self) -> &[String] {
        &self.valid_scopes
    }
}

/// ワークフロー定義の読取モデル (以後 immutable)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowDefinition {
    graph: StageGraph,
    grid: ScopeGrid,
    scopes: BTreeMap<String, ScopeMetadata>,
}

impl WorkflowDefinition {
    /// 3 入力をそのまま束ねる。
    ///
    /// グリッド列と `.md` の**不一致は検証しない** — 双方向の不一致がどちらも正当な
    /// 観測可能契約だからである (zero-EXECUTE スコープ / ランタイム不可視スコープ)。
    #[must_use]
    pub const fn new(
        graph: StageGraph,
        grid: ScopeGrid,
        scopes: BTreeMap<String, ScopeMetadata>,
    ) -> WorkflowDefinition {
        WorkflowDefinition {
            graph,
            grid,
            scopes,
        }
    }

    /// `stage-graph.json` 由来のステージグラフ (文書順を保持したまま)。
    #[must_use]
    pub const fn graph(&self) -> &StageGraph {
        &self.graph
    }

    /// `scope-grid.json` 由来の静的 EXECUTE / SKIP グリッド。recompose サフィックスは含まない。
    #[must_use]
    pub const fn grid(&self) -> &ScopeGrid {
        &self.grid
    }

    /// スコープ `.md` 由来のメタデータ (スコープ名の辞書順)。有効スコープの権威。
    #[must_use]
    pub const fn scopes(&self) -> &BTreeMap<String, ScopeMetadata> {
        &self.scopes
    }

    /// スコープ `.md` のメタデータ。`.md` が無ければ `None` (= 無効スコープ)。
    #[must_use]
    pub fn scope_metadata(&self, scope: &str) -> Option<&ScopeMetadata> {
        self.scopes.get(scope)
    }

    /// `validScopes()` — 権威はスコープ `.md` の存在 (グリッドではない)。辞書順。
    #[must_use]
    pub fn valid_scopes(&self) -> Vec<&str> {
        self.scopes.keys().map(String::as_str).collect()
    }

    /// `valid_scopes()` に含まれるか。権威は `.md` の存在であってグリッド列の有無ではない。
    #[must_use]
    pub fn is_valid_scope(&self, scope: &str) -> bool {
        self.scopes.contains_key(scope)
    }

    /// `effectivePlanAction` — **state ファイルの per-stage サフィックス (recompose 由来) が
    /// 静的グリッドに勝つ**。両方に無ければ `None` (`SKIP` に畳まない — 3 値契約)。
    ///
    /// スコープの有効性は問わない (upstream もグリッド列だけを見る)。未知スコープでは
    /// グリッド側が常に `None` になるため、結果はサフィックスのみで決まる。
    #[must_use]
    pub fn effective_plan_action(
        &self,
        suffixes: &BTreeMap<StageSlug, PlanAction>,
        scope: &str,
        slug: &StageSlug,
    ) -> Option<PlanAction> {
        suffixes
            .get(slug)
            .copied()
            .or_else(|| self.grid.action(scope, slug))
    }

    /// `subgraphForScope` — 静的グリッドの EXECUTE セルを抽出し、**数値順**で返す。
    ///
    /// ランタイムでは topo ソートしない (compile のエッジ局所不変条件により数値順が
    /// 有効な topo 順であることが保証されている — レポート §4.6)。
    ///
    /// # Errors
    ///
    /// スコープ `.md` が存在しなければ `UnknownScope` (有効スコープ一覧を添える)。
    /// **未知スコープで `Err` を返すのはこの述語だけ**である (非対称契約)。
    pub fn subgraph_for_scope(&self, scope: &str) -> Result<Vec<&StageNode>, UnknownScope> {
        if !self.is_valid_scope(scope) {
            return Err(UnknownScope::new(
                scope,
                self.valid_scopes()
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
            ));
        }
        // 列が無い有効スコープは zero-EXECUTE (エラーではない)。
        Ok(self
            .graph
            .numeric_order()
            .into_iter()
            .filter(|node| self.grid.action(scope, node.slug()) == Some(PlanAction::Execute))
            .collect())
    }

    /// `nextInScopeStage` — **文書順**に `after` の次から前進走査し、checkbox が
    /// completed / skipped のノードを読み飛ばして、実効プランが EXECUTE の最初のノードを返す。
    ///
    /// 未知スコープは `None` (`subgraph_for_scope` との非対称)。`after` が文書順に存在
    /// しない場合は先頭から走査する (upstream の `findIndex` → `-1` の帰結)。
    /// checkbox に載っていない slug は未着手として扱う。
    #[must_use]
    pub fn next_in_scope_stage(
        &self,
        after: &StageSlug,
        scope: &str,
        checkboxes: &BTreeMap<StageSlug, CheckboxState>,
        suffixes: &BTreeMap<StageSlug, PlanAction>,
    ) -> Option<&StageNode> {
        if !self.is_valid_scope(scope) {
            return None;
        }
        let start = self.graph.index_of(after).map_or(0, |i| i + 1);
        self.graph.nodes()[start..].iter().find(|node| {
            if checkboxes
                .get(node.slug())
                .is_some_and(|cb| cb.is_finished())
            {
                return false;
            }
            self.effective_plan_action(suffixes, scope, node.slug()) == Some(PlanAction::Execute)
        })
    }

    /// `firstInScopeStageOfPhase` — `subgraph_for_scope` の**数値順**の並びで最初に
    /// 当該フェーズに属するノード。walking skeleton のアンカーの導出元 (ハードコードではない)。
    ///
    /// 未知スコープは `None`。
    #[must_use]
    pub fn first_in_scope_stage_of_phase(&self, phase: PhaseId, scope: &str) -> Option<&StageNode> {
        self.subgraph_for_scope(scope)
            .ok()?
            .into_iter()
            .find(|node| node.phase() == phase)
    }

    /// `stagesInScope` — **全ステージ**について `(slug, phase, action)` を**文書順**で返す。
    ///
    /// `action` は静的グリッドの 3 値 (recompose サフィックスは合成しない)。
    /// 未知スコープは空 (`subgraph_for_scope` との非対称)。
    #[must_use]
    pub fn stages_in_scope(&self, scope: &str) -> Vec<(&StageSlug, PhaseId, Option<PlanAction>)> {
        if !self.is_valid_scope(scope) {
            return Vec::new();
        }
        self.graph
            .nodes()
            .iter()
            .map(|node| {
                (
                    node.slug(),
                    node.phase(),
                    self.grid.action(scope, node.slug()),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_definition::execution_kind::ExecutionKind;
    use crate::workflow_definition::stage_mode::StageMode;
    use crate::workflow_definition::stage_node::StageNodeBuilder;
    use crate::workflow_definition::stage_number::StageNumber;
    use proptest::prelude::*;

    /// grid 列がある 2 スコープ + `.md` だけがある 1 スコープ + `.md` が無い 1 スコープ。
    const REGISTERED: [&str; 3] = ["alpha", "beta", "delta"];
    const POOL: [&str; 3] = ["alpha", "beta", "gamma"];

    fn slug(s: &str) -> StageSlug {
        StageSlug::parse(s).unwrap()
    }

    fn node(name: &str, number: &str, phase: PhaseId, scopes: &[&str]) -> StageNode {
        StageNodeBuilder::new(
            slug(name),
            StageNumber::parse(number).unwrap(),
            name.to_string(),
            phase,
            ExecutionKind::Always,
            StageMode::Inline,
        )
        .scopes(scopes.iter().map(|s| (*s).to_string()).collect())
        .build()
    }

    fn registry(names: &[&str]) -> BTreeMap<String, ScopeMetadata> {
        names
            .iter()
            .map(|n| ((*n).to_string(), ScopeMetadata::new(n).unwrap()))
            .collect()
    }

    /// 文書順 = 数値順の小さな出荷グラフ相当。
    fn sample() -> WorkflowDefinition {
        let graph = StageGraph::new(vec![
            node("bootstrap", "0.1", PhaseId::Initialization, &[]),
            node(
                "intent-capture",
                "1.1",
                PhaseId::Ideation,
                &["alpha", "beta"],
            ),
            node("requirements", "1.2", PhaseId::Ideation, &["alpha"]),
            node("threat-model", "2.1", PhaseId::Inception, &["alpha"]),
            node(
                "code-generation",
                "3.1",
                PhaseId::Construction,
                &["alpha", "beta"],
            ),
            node("ops-runbook", "4.1", PhaseId::Operation, &["gamma"]),
        ])
        .unwrap();
        let grid = ScopeGrid::derive_from_graph(&graph);
        WorkflowDefinition::new(graph, grid, registry(&REGISTERED))
    }

    // ---- ユビキタス言語の例示 ----

    #[test]
    fn valid_scopes_are_authored_by_the_md_files_not_by_the_grid() {
        let wd = sample();
        // gamma はグリッド列を持つが `.md` が無い → ランタイムから不可視
        assert!(wd.grid().contains_scope("gamma"));
        assert_eq!(wd.valid_scopes(), vec!["alpha", "beta", "delta"]);
        assert!(!wd.is_valid_scope("gamma"));
    }

    #[test]
    fn a_scope_with_no_grid_column_is_a_legitimate_zero_execute_scope() {
        let wd = sample();
        assert!(!wd.grid().contains_scope("delta"));
        assert_eq!(wd.subgraph_for_scope("delta"), Ok(Vec::new()));
        assert_eq!(
            wd.first_in_scope_stage_of_phase(PhaseId::Ideation, "delta"),
            None
        );
        // stages_in_scope は全ステージを返すが action は 3 値の None
        let listed = wd.stages_in_scope("delta");
        assert_eq!(listed.len(), 6);
        assert!(listed.iter().all(|(_, _, action)| action.is_none()));
    }

    #[test]
    fn unknown_scopes_are_asymmetric_error_here_none_everywhere_else() {
        let wd = sample();
        let err = wd.subgraph_for_scope("gamma").unwrap_err();
        assert_eq!(err.scope(), "gamma");
        assert_eq!(err.valid_scopes(), ["alpha", "beta", "delta"]);
        assert_eq!(
            wd.next_in_scope_stage(
                &slug("bootstrap"),
                "gamma",
                &BTreeMap::new(),
                &BTreeMap::new()
            ),
            None
        );
        assert_eq!(
            wd.first_in_scope_stage_of_phase(PhaseId::Operation, "gamma"),
            None
        );
        assert!(wd.stages_in_scope("gamma").is_empty());
    }

    #[test]
    fn subgraph_extracts_execute_cells_in_numeric_order_including_initialization() {
        let wd = sample();
        let beta: Vec<&str> = wd
            .subgraph_for_scope("beta")
            .unwrap()
            .iter()
            .map(|n| n.slug().as_str())
            .collect();
        // initialization は宣言せずとも全列 EXECUTE (転置の特例)
        assert_eq!(beta, vec!["bootstrap", "intent-capture", "code-generation"]);
    }

    #[test]
    fn next_in_scope_stage_walks_document_order_and_skips_finished_checkboxes() {
        let wd = sample();
        let mut checkboxes = BTreeMap::new();
        checkboxes.insert(slug("bootstrap"), CheckboxState::Completed);
        checkboxes.insert(slug("intent-capture"), CheckboxState::Skipped);
        let next = wd
            .next_in_scope_stage(&slug("bootstrap"), "alpha", &checkboxes, &BTreeMap::new())
            .unwrap();
        assert_eq!(next.slug().as_str(), "requirements");
    }

    #[test]
    fn recompose_suffixes_beat_the_static_grid_in_next_in_scope_stage() {
        let wd = sample();
        let mut suffixes = BTreeMap::new();
        // grid では beta 列で SKIP の requirements を EXECUTE へ反転
        suffixes.insert(slug("requirements"), PlanAction::Execute);
        assert_eq!(
            wd.grid().action("beta", &slug("requirements")),
            Some(PlanAction::Skip)
        );
        assert_eq!(
            wd.effective_plan_action(&suffixes, "beta", &slug("requirements")),
            Some(PlanAction::Execute)
        );
        let next = wd
            .next_in_scope_stage(&slug("intent-capture"), "beta", &BTreeMap::new(), &suffixes)
            .unwrap();
        assert_eq!(next.slug().as_str(), "requirements");
    }

    #[test]
    fn effective_plan_action_is_three_valued() {
        let wd = sample();
        let suffixes = BTreeMap::new();
        assert_eq!(
            wd.effective_plan_action(&suffixes, "alpha", &slug("threat-model")),
            Some(PlanAction::Execute)
        );
        assert_eq!(
            wd.effective_plan_action(&suffixes, "beta", &slug("threat-model")),
            Some(PlanAction::Skip)
        );
        // グリッド列にない slug は None (SKIP に畳まない)
        assert_eq!(
            wd.effective_plan_action(&suffixes, "alpha", &slug("no-such-stage")),
            None
        );
        // 列そのものが無い有効スコープも None
        assert_eq!(
            wd.effective_plan_action(&suffixes, "delta", &slug("bootstrap")),
            None
        );
    }

    #[test]
    fn skeleton_anchor_is_derived_from_the_scope_subgraph() {
        let wd = sample();
        let anchor = wd
            .first_in_scope_stage_of_phase(PhaseId::Construction, "beta")
            .unwrap();
        assert_eq!(anchor.slug().as_str(), "code-generation");
        // beta には Inception の EXECUTE が無いのでアンカーも無い
        assert_eq!(
            wd.first_in_scope_stage_of_phase(PhaseId::Inception, "beta"),
            None
        );
    }

    #[test]
    fn document_order_and_numeric_order_are_two_distinct_paths() {
        // 文書順が数値順と一致しない手編集グラフでも両経路の使い分けが残る
        let graph = StageGraph::new(vec![
            node("late", "1.10", PhaseId::Ideation, &["alpha"]),
            node("boot", "0.1", PhaseId::Initialization, &[]),
            node("early", "1.9", PhaseId::Ideation, &["alpha"]),
        ])
        .unwrap();
        let grid = ScopeGrid::derive_from_graph(&graph);
        let wd = WorkflowDefinition::new(graph, grid, registry(&["alpha"]));

        let numeric: Vec<&str> = wd
            .subgraph_for_scope("alpha")
            .unwrap()
            .iter()
            .map(|n| n.slug().as_str())
            .collect();
        assert_eq!(numeric, vec!["boot", "early", "late"]);

        // next は文書順 — "late" (文書順 0) の次は "boot"
        let next = wd
            .next_in_scope_stage(&slug("late"), "alpha", &BTreeMap::new(), &BTreeMap::new())
            .unwrap();
        assert_eq!(next.slug().as_str(), "boot");

        // stages_in_scope も文書順
        let listed: Vec<&str> = wd
            .stages_in_scope("alpha")
            .iter()
            .map(|(s, _, _)| s.as_str())
            .collect();
        assert_eq!(listed, vec!["late", "boot", "early"]);
    }

    // ---- PBT: ランダム合成グラフ + グリッド ----

    type NodeSpec = (u32, u32, Vec<usize>);

    fn arb_specs() -> impl Strategy<Value = Vec<NodeSpec>> {
        proptest::collection::vec(
            (
                0u32..5,
                0u32..40,
                proptest::collection::vec(0usize..POOL.len(), 0..3),
            ),
            1..10,
        )
    }

    fn build(specs: &[NodeSpec]) -> WorkflowDefinition {
        let nodes: Vec<StageNode> = specs
            .iter()
            .enumerate()
            .map(|(i, (phase_index, seq, scope_indices))| {
                let phase = PhaseId::from_index(*phase_index).unwrap_or(PhaseId::Ideation);
                let scopes: Vec<&str> = scope_indices.iter().map(|&j| POOL[j]).collect();
                node(
                    &format!("s{i}"),
                    &format!("{phase_index}.{seq}"),
                    phase,
                    &scopes,
                )
            })
            .collect();
        let graph = StageGraph::new(nodes).unwrap();
        let grid = ScopeGrid::derive_from_graph(&graph);
        WorkflowDefinition::new(graph, grid, registry(&REGISTERED))
    }

    fn checkbox_map(wd: &WorkflowDefinition, markers: &[u8]) -> BTreeMap<StageSlug, CheckboxState> {
        wd.graph()
            .nodes()
            .iter()
            .zip(markers.iter())
            .map(|(n, m)| {
                let state = match m % 4 {
                    0 => CheckboxState::Pending,
                    1 => CheckboxState::InProgress,
                    2 => CheckboxState::Completed,
                    _ => CheckboxState::Skipped,
                };
                (n.slug().clone(), state)
            })
            .collect()
    }

    fn suffix_map(wd: &WorkflowDefinition, bits: &[u8]) -> BTreeMap<StageSlug, PlanAction> {
        wd.graph()
            .nodes()
            .iter()
            .zip(bits.iter())
            .filter(|(_, b)| **b % 3 != 0)
            .map(|(n, b)| {
                let action = if b % 3 == 1 {
                    PlanAction::Execute
                } else {
                    PlanAction::Skip
                };
                (n.slug().clone(), action)
            })
            .collect()
    }

    proptest! {
        /// `subgraph_for_scope` の結果はすべてグリッド EXECUTE かつ数値順、
        /// かつ EXECUTE セルを 1 つも取りこぼさない。
        #[test]
        fn subgraph_is_exactly_the_execute_cells_in_numeric_order(specs in arb_specs()) {
            let wd = build(&specs);
            for scope in wd.valid_scopes() {
                let sub = wd.subgraph_for_scope(scope).unwrap();
                for n in &sub {
                    prop_assert_eq!(
                        wd.grid().action(scope, n.slug()),
                        Some(PlanAction::Execute)
                    );
                }
                for w in sub.windows(2) {
                    prop_assert!(
                        w[0].number().numeric_cmp(w[1].number()) != std::cmp::Ordering::Greater
                    );
                }
                let expected = wd
                    .graph()
                    .nodes()
                    .iter()
                    .filter(|n| wd.grid().action(scope, n.slug()) == Some(PlanAction::Execute))
                    .count();
                prop_assert_eq!(sub.len(), expected);
            }
        }

        /// `next_in_scope_stage` は文書順で `after` より後の、checkbox 未完了かつ
        /// 実効 EXECUTE な**最初の**ノードを返す (最小性まで含めて特徴づける)。
        #[test]
        fn next_in_scope_stage_is_the_first_qualifying_node_in_document_order(
            specs in arb_specs(),
            markers in proptest::collection::vec(any::<u8>(), 10),
            bits in proptest::collection::vec(any::<u8>(), 10),
            after_index in 0usize..10,
            scope_index in 0usize..REGISTERED.len(),
        ) {
            let wd = build(&specs);
            let checkboxes = checkbox_map(&wd, &markers);
            let suffixes = suffix_map(&wd, &bits);
            let scope = REGISTERED[scope_index];
            let after_index = after_index % wd.graph().len();
            let after = wd.graph().at(after_index).unwrap().slug().clone();

            let qualifies = |i: usize| {
                let n = wd.graph().at(i).unwrap();
                let finished = matches!(
                    checkboxes.get(n.slug()),
                    Some(CheckboxState::Completed | CheckboxState::Skipped)
                );
                !finished
                    && wd.effective_plan_action(&suffixes, scope, n.slug())
                        == Some(PlanAction::Execute)
            };

            let result = wd.next_in_scope_stage(&after, scope, &checkboxes, &suffixes);
            match result {
                Some(found) => {
                    let found_index = wd.graph().index_of(found.slug()).unwrap();
                    prop_assert!(found_index > after_index, "文書順で after より後");
                    prop_assert!(qualifies(found_index));
                    for i in (after_index + 1)..found_index {
                        prop_assert!(!qualifies(i), "最小性: {i} が先に該当してはならない");
                    }
                }
                None => {
                    for i in (after_index + 1)..wd.graph().len() {
                        prop_assert!(!qualifies(i), "該当なしのはずが {i} が該当");
                    }
                }
            }
        }

        /// 未知スコープの非対称契約: `subgraph_for_scope` だけが `Err`。
        #[test]
        fn unknown_scope_is_error_only_for_subgraph(
            specs in arb_specs(),
            name in "[a-z]{1,10}",
        ) {
            let wd = build(&specs);
            prop_assume!(!wd.is_valid_scope(&name));
            let err = wd.subgraph_for_scope(&name).unwrap_err();
            prop_assert_eq!(err.scope(), name.as_str());
            prop_assert_eq!(err.valid_scopes(), ["alpha", "beta", "delta"]);
            let after = wd.graph().at(0).unwrap().slug().clone();
            prop_assert!(
                wd.next_in_scope_stage(&after, &name, &BTreeMap::new(), &BTreeMap::new())
                    .is_none()
            );
            for phase in PhaseId::ALL {
                prop_assert!(wd.first_in_scope_stage_of_phase(phase, &name).is_none());
            }
            prop_assert!(wd.stages_in_scope(&name).is_empty());
        }

        /// `effective_plan_action`: サフィックスがグリッドに勝ち、両方に無ければ `None`。
        #[test]
        fn suffixes_beat_the_grid_and_absence_is_none(
            specs in arb_specs(),
            bits in proptest::collection::vec(any::<u8>(), 10),
            scope_index in 0usize..REGISTERED.len(),
        ) {
            let wd = build(&specs);
            let suffixes = suffix_map(&wd, &bits);
            let scope = REGISTERED[scope_index];
            for n in wd.graph().nodes() {
                let effective = wd.effective_plan_action(&suffixes, scope, n.slug());
                match suffixes.get(n.slug()) {
                    Some(&overridden) => prop_assert_eq!(effective, Some(overridden)),
                    None => prop_assert_eq!(effective, wd.grid().action(scope, n.slug())),
                }
            }
            // グラフに無い slug は、サフィックスにもグリッドにも無いので必ず None
            prop_assert_eq!(
                wd.effective_plan_action(&suffixes, scope, &slug("not-in-graph")),
                None
            );
        }

        /// `stages_in_scope` は全ステージを文書順で返し、`action` は静的グリッドの 3 値。
        #[test]
        fn stages_in_scope_lists_every_stage_in_document_order(
            specs in arb_specs(),
            scope_index in 0usize..REGISTERED.len(),
        ) {
            let wd = build(&specs);
            let scope = REGISTERED[scope_index];
            let listed = wd.stages_in_scope(scope);
            prop_assert_eq!(listed.len(), wd.graph().len());
            for (i, (s, phase, action)) in listed.iter().enumerate() {
                let n = wd.graph().at(i).unwrap();
                prop_assert_eq!(*s, n.slug());
                prop_assert_eq!(*phase, n.phase());
                prop_assert_eq!(*action, wd.grid().action(scope, n.slug()));
            }
        }

        /// `first_in_scope_stage_of_phase` は subgraph の数値順で最初の該当フェーズ。
        #[test]
        fn first_in_scope_stage_of_phase_agrees_with_the_subgraph(
            specs in arb_specs(),
            scope_index in 0usize..REGISTERED.len(),
        ) {
            let wd = build(&specs);
            let scope = REGISTERED[scope_index];
            let sub = wd.subgraph_for_scope(scope).unwrap();
            for phase in PhaseId::ALL {
                let expected = sub.iter().find(|n| n.phase() == phase).map(|n| n.slug());
                let actual = wd
                    .first_in_scope_stage_of_phase(phase, scope)
                    .map(StageNode::slug);
                prop_assert_eq!(actual, expected);
            }
        }
    }
}
