//! `DefinitionView` — ワークフロー定義リードモデル 3 入力を束ねたクエリモデル。
//!
//! 「何を実行しうるか」の静的定義を 1 つのビューにまとめ、クエリ側のユースケースが依存する
//! 5 述語を純関数として提供する (01 §3.1 / 10 §3)。**集約ではない** — クエリ側は集約を
//! 再構成しない (`coding-rules/cqrs-boundaries.md` 規則 6)。コマンドも状態遷移もイベントも
//! 持たず、読むだけである。
//!
//! # 観測可能契約 (12 §6.1)
//!
//! - **未知スコープの非対称**: [`DefinitionView::subgraph_for_scope`] だけが
//!   `Err(UnknownScope)`。[`DefinitionView::first_in_scope_stage_of_phase`] /
//!   [`DefinitionView::stages_in_scope`] は同じ未知スコープに対して `None` / 空を返す。
//! - **`.md` あり × グリッド列なし** = zero-EXECUTE な**正当**スコープ (エラーにしない)。
//! - **グリッド列あり × `.md` なし** = ランタイムから不可視 (有効スコープの権威は `.md`)。
//! - **グリッドに slug が無い** = `None`。`SKIP` に畳まない (3 値契約)。
//! - **文書順の保持**: `stages_in_scope` は文書順で全ステージを返し、`subgraph_for_scope`
//!   だけが数値順にソートする。
//!
//! `enabled: false` のノードは**除外しない**。意味論が未確定 (12 §7) のため、ビューは
//! [`StageView::is_enabled`] を露出するだけで判断は呼出側に委ねる。

use std::collections::BTreeMap;
use std::fmt;

use super::definition_id_view::DefinitionIdView;
use super::definition_revision_view::DefinitionRevisionView;
use super::phase_view::PhaseView;
use super::plan_action_view::PlanActionView;
use super::scope_grid_view::ScopeGridView;
use super::scope_metadata_view::ScopeMetadataView;
use super::stage_graph_view::StageGraphView;
use super::stage_slug_view::StageSlugView;
use super::stage_view::StageView;

/// `validScopes()` に無いスコープ名。
///
/// upstream の逐語文言 `Unknown scope: "<scope>". Valid scopes: <csv>` を組み立てるのに
/// 必要な材料をそのまま保持する (文言化は出す側の責務)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownScope {
    scope: String,
    valid_scopes: Vec<String>,
}

/// ワークフロー定義リードモデルのビュー (構築後 immutable)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionView {
    id: DefinitionIdView,
    revision: DefinitionRevisionView,
    graph: StageGraphView,
    grid: ScopeGridView,
    scopes: BTreeMap<String, ScopeMetadataView>,
}

impl UnknownScope {
    /// 拒否されたスコープ名と、拒否時点の有効スコープ一覧 (辞書順) を束ねる。
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

impl fmt::Display for UnknownScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown scope {:?}; valid scopes: {}",
            self.scope,
            self.valid_scopes.join(", ")
        )
    }
}

impl std::error::Error for UnknownScope {}

impl DefinitionView {
    /// 読み終えた 3 入力を束ねる。
    ///
    /// `id` / `revision` はアダプタ層が付与する (ADR-008) — ビューは revision を計算しない。
    /// グリッド列と `.md` の**不一致は検証しない**: 双方向の不一致がどちらも正当な観測可能
    /// 契約だからである (zero-EXECUTE スコープ / ランタイム不可視スコープ)。
    #[must_use]
    pub const fn new(
        id: DefinitionIdView,
        revision: DefinitionRevisionView,
        graph: StageGraphView,
        grid: ScopeGridView,
        scopes: BTreeMap<String, ScopeMetadataView>,
    ) -> DefinitionView {
        DefinitionView {
            id,
            revision,
            graph,
            grid,
            scopes,
        }
    }

    /// この定義の系譜 ID。内容が変わっても不変 (ADR-008)。
    #[must_use]
    pub const fn id(&self) -> &DefinitionIdView {
        &self.id
    }

    /// この定義の内容版。3 入力が 1 バイトでも変われば変わる (ADR-008)。
    #[must_use]
    pub const fn revision(&self) -> &DefinitionRevisionView {
        &self.revision
    }

    /// `stage-graph.json` 由来のステージグラフ (文書順を保持したまま)。
    #[must_use]
    pub const fn graph(&self) -> &StageGraphView {
        &self.graph
    }

    /// `scope-grid.json` 由来の静的 EXECUTE / SKIP グリッド。
    #[must_use]
    pub const fn grid(&self) -> &ScopeGridView {
        &self.grid
    }

    /// スコープ `.md` 由来のメタデータ (スコープ名の辞書順)。有効スコープの権威。
    #[must_use]
    pub const fn scopes(&self) -> &BTreeMap<String, ScopeMetadataView> {
        &self.scopes
    }

    /// スコープ `.md` のメタデータ。`.md` が無ければ `None` (= 無効スコープ)。
    #[must_use]
    pub fn scope_metadata(&self, scope: &str) -> Option<&ScopeMetadataView> {
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
    /// ランタイムでは topo ソートしない (compile のエッジ局所不変条件により数値順が有効な
    /// topo 順であることが保証されている — 12 §4.6)。
    ///
    /// # Errors
    ///
    /// スコープ `.md` が存在しなければ [`UnknownScope`] (有効スコープ一覧を添える)。
    /// **未知スコープで `Err` を返すのはこの述語だけ**である (非対称契約)。
    pub fn subgraph_for_scope(&self, scope: &str) -> Result<Vec<&StageView>, UnknownScope> {
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
            .filter(|node| self.grid.action(scope, node.slug()) == Some(PlanActionView::Execute))
            .collect())
    }

    /// `firstInScopeStageOfPhase` — `subgraph_for_scope` の**数値順**の並びで最初に当該
    /// フェーズに属するノード。walking skeleton のアンカーの導出元 (ハードコードではない)。
    ///
    /// 未知スコープは `None`。
    #[must_use]
    pub fn first_in_scope_stage_of_phase(
        &self,
        phase: PhaseView,
        scope: &str,
    ) -> Option<&StageView> {
        self.subgraph_for_scope(scope)
            .ok()?
            .into_iter()
            .find(|node| node.phase() == phase)
    }

    /// `stagesInScope` — **全ステージ**について `(slug, phase, action)` を**文書順**で返す。
    ///
    /// `action` は静的グリッドの 3 値。未知スコープは空 (`subgraph_for_scope` との非対称)。
    #[must_use]
    pub fn stages_in_scope(
        &self,
        scope: &str,
    ) -> Vec<(&StageSlugView, PhaseView, Option<PlanActionView>)> {
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
    #![allow(clippy::indexing_slicing)]

    use super::*;
    use crate::workflow_view::{
        ExecutionKindView, StageModeView, StageNumberView, StageViewBuilder,
    };

    fn slug(s: &str) -> StageSlugView {
        StageSlugView::parse(s).unwrap()
    }

    fn node(name: &str, number: &str, phase: PhaseView, scopes: &[&str]) -> StageView {
        StageViewBuilder::new(
            slug(name),
            StageNumberView::parse(number).unwrap(),
            name.to_string(),
            phase,
            ExecutionKindView::Always,
            StageModeView::Inline,
        )
        .with_scopes(scopes.iter().map(|s| (*s).to_string()).collect())
        .build()
    }

    /// grid 列がある 2 スコープ (`alpha` / `beta`)、`.md` だけがある `delta`、
    /// grid 列だけがある `gamma`。
    fn definition() -> DefinitionView {
        let graph = StageGraphView::new(vec![
            node("bootstrap", "0.1", PhaseView::Initialization, &[]),
            node(
                "intent-capture",
                "1.1",
                PhaseView::Ideation,
                &["alpha", "beta"],
            ),
            node("requirements", "1.2", PhaseView::Ideation, &["alpha"]),
            node(
                "code-generation",
                "3.1",
                PhaseView::Construction,
                &["alpha"],
            ),
            node("ops-runbook", "4.1", PhaseView::Operation, &["gamma"]),
        ])
        .unwrap();
        let grid = ScopeGridView::from_graph(&graph);
        let scopes = ["alpha", "beta", "delta"]
            .iter()
            .map(|n| ((*n).to_string(), ScopeMetadataView::new(n).unwrap()))
            .collect();
        DefinitionView::new(
            DefinitionIdView::parse("claude").unwrap(),
            DefinitionRevisionView::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            graph,
            grid,
            scopes,
        )
    }

    #[test]
    fn the_identity_and_the_content_revision_are_read_back() {
        let definition = definition();
        assert_eq!(definition.id().as_str(), "claude");
        assert!(definition.revision().as_str().starts_with("sha256:"));
        assert_eq!(definition.graph().len(), 5);
        assert!(definition.grid().contains_scope("gamma"));
        assert!(definition.scope_metadata("alpha").is_some());
        assert!(definition.scope_metadata("gamma").is_none());
        assert_eq!(definition.scopes().len(), 3);
    }

    #[test]
    fn the_authority_for_valid_scopes_is_the_identity_file_not_the_grid() {
        let definition = definition();
        assert_eq!(definition.valid_scopes(), ["alpha", "beta", "delta"]);
        assert!(definition.is_valid_scope("delta"));
        // grid 列だけの `gamma` はランタイムから不可視。
        assert!(definition.grid().contains_scope("gamma"));
        assert!(!definition.is_valid_scope("gamma"));
    }

    #[test]
    fn the_subgraph_is_numeric_order_and_a_column_less_scope_is_simply_empty() {
        let definition = definition();
        let alpha: Vec<&str> = definition
            .subgraph_for_scope("alpha")
            .unwrap()
            .iter()
            .map(|n| n.slug().as_str())
            .collect();
        assert_eq!(
            alpha,
            [
                "bootstrap",
                "intent-capture",
                "requirements",
                "code-generation"
            ]
        );
        // `.md` はあるが grid 列が無い `delta` は zero-EXECUTE な正当スコープ。
        assert!(definition.subgraph_for_scope("delta").unwrap().is_empty());
    }

    #[test]
    fn only_the_subgraph_predicate_rejects_an_unknown_scope() {
        let definition = definition();
        let error = definition.subgraph_for_scope("gamma").unwrap_err();
        assert_eq!(error.scope(), "gamma");
        assert_eq!(error.valid_scopes(), ["alpha", "beta", "delta"]);
        assert!(
            error
                .to_string()
                .starts_with("unknown scope \"gamma\"; valid scopes: ")
        );
        // 同じ未知スコープでも他の述語は None / 空。
        assert_eq!(
            definition.first_in_scope_stage_of_phase(PhaseView::Ideation, "gamma"),
            None
        );
        assert!(definition.stages_in_scope("gamma").is_empty());
    }

    #[test]
    fn the_first_in_scope_stage_of_a_phase_is_derived_from_the_subgraph() {
        let definition = definition();
        assert_eq!(
            definition
                .first_in_scope_stage_of_phase(PhaseView::Ideation, "alpha")
                .map(|n| n.slug().as_str()),
            Some("intent-capture")
        );
        assert_eq!(
            definition.first_in_scope_stage_of_phase(PhaseView::Construction, "beta"),
            None
        );
    }

    #[test]
    fn stages_in_scope_walks_every_stage_in_document_order_with_the_three_valued_action() {
        let definition = definition();
        let rows = definition.stages_in_scope("beta");
        assert_eq!(rows.len(), 5);
        let listed: Vec<&str> = rows.iter().map(|(s, _, _)| s.as_str()).collect();
        assert_eq!(
            listed,
            [
                "bootstrap",
                "intent-capture",
                "requirements",
                "code-generation",
                "ops-runbook"
            ]
        );
        assert_eq!(rows[0].1, PhaseView::Initialization);
        assert_eq!(rows[0].2, Some(PlanActionView::Execute));
        assert_eq!(rows[2].2, Some(PlanActionView::Skip));

        // 列を持たない `delta` は全行が 3 値の None。
        let rows = definition.stages_in_scope("delta");
        assert_eq!(rows.len(), 5);
        assert!(rows.iter().all(|(_, _, action)| action.is_none()));
    }
}
