//! `WorkflowDefinition` — Published Language (`stage-graph.json` / `scope-grid.json` /
//! `scopes/*.md`) の**読取モデル**を内包するエンティティ。「何を実行しうるか」の静的定義を
//! 1 つの集約にまとめ、orchestration が依存する 6 述語を純関数として提供する
//! (01 §3.1 / 10 §3)。
//!
//! 識別子 `WorkflowDefinitionId` と内容版 `DefinitionRevision` を持つ (ADR-008)。id は
//! 内容が変わっても不変の系譜 ID、revision は 3 入力の内容ダイジェストであり、どちらも
//! Repository 実装が付与する (ドメインは計算しない)。
//!
//! # 観測可能契約 (レポート §6.1 — 逸脱台帳行き)
//!
//! - **未知スコープの非対称**: `subgraph_for_scope` だけが `Err(UnknownScope)`。
//!   `first_in_scope_stage_of_phase` / `stages_in_scope` は同じ未知スコープに対して
//!   `None` / 空を返す。
//! - **`.md` あり × グリッド列なし** = zero-EXECUTE な**正当**スコープ (エラーにしない)。
//! - **グリッド列あり × `.md` なし** = ランタイムから不可視 (有効スコープの権威は `.md`)。
//! - **グリッドに slug が無い** = `None`。`SKIP` に畳まない (3 値契約)。
//! - **文書順の保持**: `stages_in_scope` は文書順で全ステージを返し、`subgraph_for_scope`
//!   だけが数値順にソートする。2 経路の使い分けを潰さない。
//!
//! `enabled: false` のノードは**除外しない**。意味論が未確定 (レポート §7) のため、
//! 読取モデルは `StageNode::is_enabled()` を露出するだけで判断は呼出側に委ねる。
//!
//! # 集約への畳み込み (FR8.4)
//!
//! かつてここにあった `effective_plan_action` / `next_in_scope_stage` は
//! **`IntentExecution` 側へ移設**した。recompose オーバレイと checkbox は実行の状態で
//! あって定義の状態ではなく、定義側に置くと「呼出側が状態を持ち回って定義に問い直す」
//! Ask 形になるためである (tell-dont-ask.md)。定義側に残るのは静的グリッドの照会
//! (`grid().action(scope, slug)`) と文書順の全ステージ列 (`stages_in_scope`) だけで、
//! 実効プランの合成は集約が行う。

use std::collections::BTreeMap;

use super::definition_revision::DefinitionRevision;
use super::phase::PhaseId;
use super::plan_action::PlanAction;
use super::scope_grid::ScopeGrid;
use super::scope_metadata::ScopeMetadata;
use super::stage_graph::StageGraph;
use super::stage_node::StageNode;
use super::stage_slug::StageSlug;
use super::workflow_definition_id::WorkflowDefinitionId;

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

/// ワークフロー定義のエンティティ (構築後 immutable)。
///
/// 等価は**内容と識別子の両方**で決まる (derive)。読取モデルは 3 入力から毎回組み立て直す
/// 値であり、「同じ系譜の同じ内容」を 1 つの等価関係で表すのが自然だからである。id だけの
/// 同一性比較が要るのは `IntentExecution` 側の定義照合で、そちらは `id()` 同士を突き合わせる
/// (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/domain-equality.md)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowDefinition {
    id: WorkflowDefinitionId,
    revision: DefinitionRevision,
    graph: StageGraph,
    grid: ScopeGrid,
    scopes: BTreeMap<String, ScopeMetadata>,
}

impl WorkflowDefinition {
    /// 識別子・内容版と 3 入力をそのまま束ねる。
    ///
    /// `id` / `revision` は **Repository 実装が付与する** (ADR-008)。ドメインは revision を
    /// 計算しない — 正準 JSON とダイジェストはアダプタ層の責務である。
    ///
    /// グリッド列と `.md` の**不一致は検証しない** — 双方向の不一致がどちらも正当な
    /// 観測可能契約だからである (zero-EXECUTE スコープ / ランタイム不可視スコープ)。
    #[must_use]
    pub const fn new(
        id: WorkflowDefinitionId,
        revision: DefinitionRevision,
        graph: StageGraph,
        grid: ScopeGrid,
        scopes: BTreeMap<String, ScopeMetadata>,
    ) -> WorkflowDefinition {
        WorkflowDefinition {
            id,
            revision,
            graph,
            grid,
            scopes,
        }
    }

    /// この定義の系譜 ID。内容が変わっても不変 (ADR-008)。
    #[must_use]
    pub const fn id(&self) -> &WorkflowDefinitionId {
        &self.id
    }

    /// この定義の内容版。3 入力が 1 バイトでも変われば変わる (ADR-008)。
    #[must_use]
    pub const fn revision(&self) -> &DefinitionRevision {
        &self.revision
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
    // テストは固定長フィクスチャの添字参照を許容 (clippy.toml に相当設定が無いため file 単位で
    // allow)。
    #![allow(clippy::indexing_slicing)]

    use super::*;
    use crate::workflow_definition::{ExecutionKind, StageMode, StageNodeBuilder, StageNumber};
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

    fn id(value: &str) -> WorkflowDefinitionId {
        WorkflowDefinitionId::parse(value).unwrap()
    }

    fn revision(fill: char) -> DefinitionRevision {
        DefinitionRevision::parse(&format!("sha256:{}", fill.to_string().repeat(64))).unwrap()
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
        let grid = ScopeGrid::from_graph(&graph);
        WorkflowDefinition::new(
            id("claude"),
            revision('0'),
            graph,
            grid,
            registry(&REGISTERED),
        )
    }

    // ---- エンティティの識別子と内容版 (ADR-008) ----

    #[test]
    fn the_definition_carries_the_identity_and_the_revision_the_repository_assigned() {
        let wd = sample();
        assert_eq!(wd.id(), &id("claude"));
        assert_eq!(wd.revision(), &revision('0'));
        assert_eq!(wd.id().as_str(), "claude");
        assert!(wd.revision().as_str().starts_with("sha256:"));
    }

    #[test]
    fn two_definitions_with_the_same_content_but_different_lineage_are_not_equal() {
        let one = sample();
        let graph = one.graph().clone();
        let grid = one.grid().clone();
        let other = WorkflowDefinition::new(
            id("kiro"),
            revision('0'),
            graph,
            grid,
            registry(&REGISTERED),
        );
        assert_ne!(one, other);
        assert_ne!(one.id(), other.id());
        // 内容版は同じ — 系譜だけが違う。
        assert_eq!(one.revision(), other.revision());
    }

    #[test]
    fn the_revision_changes_without_the_identity_changing() {
        let one = sample();
        let other = WorkflowDefinition::new(
            one.id().clone(),
            revision('1'),
            one.graph().clone(),
            one.grid().clone(),
            registry(&REGISTERED),
        );
        // ピン更新 = 内容版だけが進む。系譜 ID は不変 (ADR-008)。
        assert_eq!(one.id(), other.id());
        assert_ne!(one.revision(), other.revision());
        assert_ne!(one, other);
    }

    #[test]
    fn the_six_predicates_survive_the_entity_change() {
        // FR8.4 で 2 述語を集約へ移したあとに定義側へ残る照会一式。
        let wd = sample();
        assert!(wd.is_valid_scope("alpha"));
        assert_eq!(wd.valid_scopes(), ["alpha", "beta", "delta"]);
        assert!(wd.scope_metadata("alpha").is_some());
        assert!(wd.subgraph_for_scope("alpha").is_ok());
        assert_eq!(wd.stages_in_scope("alpha").len(), 6);
        assert!(
            wd.first_in_scope_stage_of_phase(PhaseId::Ideation, "alpha")
                .is_some()
        );
    }

    #[test]
    fn stages_in_scope_reports_the_phase_of_every_stage_alongside_the_action() {
        // Started の StageEntry 列 (集約側) はこの 3 つ組から作られるため、PhaseId が
        // 文書順で正しく載っていることが FR8.4 移設後の前提になる。
        let wd = sample();
        let rows = wd.stages_in_scope("alpha");
        let phases: Vec<PhaseId> = rows.iter().map(|(_, phase, _)| *phase).collect();
        assert_eq!(
            phases,
            [
                PhaseId::Initialization,
                PhaseId::Ideation,
                PhaseId::Ideation,
                PhaseId::Inception,
                PhaseId::Construction,
                PhaseId::Operation,
            ]
        );
        // 索引 0 だけが initialization — 集約の gated(s) 判定の材料。
        assert_eq!(rows[0].1, PhaseId::Initialization);
        assert!(
            rows[1..]
                .iter()
                .all(|(_, p, _)| *p != PhaseId::Initialization)
        );
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
    fn the_static_grid_query_is_three_valued() {
        // FR8.4 で `effective_plan_action` を集約へ移設したあと、定義側に残るのは
        // 静的グリッドの照会だけ。3 値契約 (EXECUTE / SKIP / 未コンパイル) はここが持つ。
        let wd = sample();
        assert_eq!(
            wd.grid().action("alpha", &slug("threat-model")),
            Some(PlanAction::Execute)
        );
        assert_eq!(
            wd.grid().action("beta", &slug("threat-model")),
            Some(PlanAction::Skip)
        );
        // グリッド列にない slug は None (SKIP に畳まない)
        assert_eq!(wd.grid().action("alpha", &slug("no-such-stage")), None);
        // 列そのものが無い有効スコープも None
        assert_eq!(wd.grid().action("delta", &slug("bootstrap")), None);
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
        let grid = ScopeGrid::from_graph(&graph);
        let wd = WorkflowDefinition::new(
            id("claude"),
            revision('0'),
            graph,
            grid,
            registry(&["alpha"]),
        );

        let numeric: Vec<&str> = wd
            .subgraph_for_scope("alpha")
            .unwrap()
            .iter()
            .map(|n| n.slug().as_str())
            .collect();
        assert_eq!(numeric, vec!["boot", "early", "late"]);

        // stages_in_scope は文書順
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
        let grid = ScopeGrid::from_graph(&graph);
        WorkflowDefinition::new(
            id("claude"),
            revision('0'),
            graph,
            grid,
            registry(&REGISTERED),
        )
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
            for phase in PhaseId::ALL {
                prop_assert!(wd.first_in_scope_stage_of_phase(phase, &name).is_none());
            }
            prop_assert!(wd.stages_in_scope(&name).is_empty());
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
