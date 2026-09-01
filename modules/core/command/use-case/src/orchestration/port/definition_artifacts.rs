//! `DefinitionArtifacts` — 取り込んだ配布物。定義を確立・改訂するための材料一式。

use std::collections::BTreeMap;

use core_command_domain::workflow_definition::{
    DefinitionRevision, ScopeGrid, ScopeMetadata, StageGraph, WorkflowDefinitionId,
};

/// 取り込んだ配布物 — 定義を確立・改訂するための材料一式。
///
/// # これは集約の写し (memento 双子) ではない
///
/// フィールドの並びは集約 `WorkflowDefinition` の内容と一致するが、性格が違う
/// (`coding-rules/aggregate-commands.md` が禁じた `IntentSnapshot` 型との違い):
///
/// - **永続化の復号中間表現ではない。** 集約の保存像を読み戻すための型ではなく、
///   外部配布物を読んだ結果である。
/// - **これを引数に取るファクトリを作らない。** `Intent::from_material` のような
///   「genesis と同一署名の双子」は生やさず、ユースケースが分解して `define` / `redefine`
///   の引数へ渡す。
/// - ドメインではなくポート層に住み、ドメイン型の合成でしかない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionArtifacts {
    id: WorkflowDefinitionId,
    revision: DefinitionRevision,
    graph: StageGraph,
    grid: ScopeGrid,
    scopes: BTreeMap<String, ScopeMetadata>,
}

impl DefinitionArtifacts {
    /// 読み取った 3 入力のモデルと、それが名乗る系譜 ID・内容版を束ねる。
    #[must_use]
    pub const fn new(
        id: WorkflowDefinitionId,
        revision: DefinitionRevision,
        graph: StageGraph,
        grid: ScopeGrid,
        scopes: BTreeMap<String, ScopeMetadata>,
    ) -> DefinitionArtifacts {
        DefinitionArtifacts {
            id,
            revision,
            graph,
            grid,
            scopes,
        }
    }

    /// 配布物が名乗る系譜 ID (`harness.json` の `name` — ADR-008)。
    #[must_use]
    pub const fn id(&self) -> &WorkflowDefinitionId {
        &self.id
    }

    /// 読めた 3 入力の内容ダイジェスト。
    ///
    /// 「ディスクにあったバイトの版」ではなく「**読めた 3 入力の内容**の版」である —
    /// グリッドが欠けて転置導出へ倒れた場合も、導出結果を同じ形へ直列化して算出するので、
    /// 同じ内容の grid ファイルが置かれたときと同じ値になる。
    #[must_use]
    pub const fn revision(&self) -> &DefinitionRevision {
        &self.revision
    }

    /// `stage-graph.json` 由来のステージグラフ (文書順を保持したまま)。
    #[must_use]
    pub const fn graph(&self) -> &StageGraph {
        &self.graph
    }

    /// `scope-grid.json` 由来の静的 EXECUTE / SKIP グリッド (欠損時は転置導出)。
    #[must_use]
    pub const fn grid(&self) -> &ScopeGrid {
        &self.grid
    }

    /// スコープ `.md` 由来のメタデータ (スコープ名の辞書順)。有効スコープの権威。
    #[must_use]
    pub const fn scopes(&self) -> &BTreeMap<String, ScopeMetadata> {
        &self.scopes
    }

    /// 材料を分解して手放す (`define` / `redefine` の引数へ渡すため)。
    ///
    /// 内容 3 点だけを返し、系譜 ID と内容版は返さない — 改訂は識別子を変えず、
    /// 内容版は呼出側が [`DefinitionArtifacts::revision`] で先に読むからである。
    #[must_use]
    pub fn into_content(self) -> (StageGraph, ScopeGrid, BTreeMap<String, ScopeMetadata>) {
        (self.graph, self.grid, self.scopes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use core_command_domain::workflow_definition::{
        ExecutionKind, PhaseId, StageMode, StageNodeBuilder, StageNumber, StageSlug,
    };

    #[test]
    fn the_material_reports_back_every_part_it_was_given() {
        // 取込が読んだ 5 つの材料は、そのまま `define` / `redefine` の引数になる。
        // `into_content` は内容 3 点だけを手放し、系譜 ID と内容版は先に読む形である。
        let graph = StageGraph::new(vec![
            StageNodeBuilder::new(
                StageSlug::parse("state-init").expect("slug"),
                StageNumber::parse("0.1").expect("番号"),
                "State Init".to_string(),
                PhaseId::Initialization,
                ExecutionKind::Always,
                StageMode::Inline,
            )
            .scopes(vec!["classic".to_string()])
            .build(),
        ])
        .expect("グラフ");
        let grid = ScopeGrid::from_graph(&graph);
        let scopes: BTreeMap<String, ScopeMetadata> = [(
            "classic".to_string(),
            ScopeMetadata::new("classic").expect("スコープ"),
        )]
        .into_iter()
        .collect();
        let id = WorkflowDefinitionId::parse("claude").expect("定義 id");
        let revision =
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision");

        let artifacts = DefinitionArtifacts::new(
            id.clone(),
            revision.clone(),
            graph.clone(),
            grid.clone(),
            scopes.clone(),
        );
        assert_eq!(artifacts.id(), &id);
        assert_eq!(artifacts.revision(), &revision);
        assert_eq!(artifacts.graph(), &graph);
        assert_eq!(artifacts.grid(), &grid);
        assert_eq!(artifacts.scopes(), &scopes);

        let (out_graph, out_grid, out_scopes) = artifacts.into_content();
        assert_eq!(out_graph, graph);
        assert_eq!(out_grid, grid);
        assert_eq!(out_scopes, scopes);
    }
}
