//! `DefinitionContentDto` — 定義の内容 (3 入力のモデル) の行の形 (**読む側**)。
//!
//! 誕生 ([`DefinedDto`]) と改訂 ([`RedefinedDto`]) はどちらもこの形を運ぶ。内容の綴りが
//! 1 か所に束なるので、面ごとの乖離が構造的に起きない。
//!
//! [`DefinedDto`]: super::defined_dto::DefinedDto
//! [`RedefinedDto`]: super::redefined_dto::RedefinedDto

use std::collections::BTreeMap;

use core_command_domain::workflow_definition::{ScopeGrid, ScopeMetadata, StageGraph, StageSlug};
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::dto_vocabulary::{plan_action_of, plan_action_spelling};
use super::scope_metadata_dto::ScopeMetadataDto;
use super::stage_node_dto::StageNodeDto;

/// 定義の内容 (3 入力のモデル) の行の形。**フィールド名と並びが契約**である。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct DefinitionContentDto {
    /// 文書順のステージ列 (読込時に数値順へ正規化しない — F2)。
    graph: Vec<StageNodeDto>,
    /// `<scope> -> <slug> -> EXECUTE|SKIP`。**`stage-graph.json` 面の中間 `"stages"` キーは
    /// 持たない** — あれはレガシー互換のための 2 段構造であって、行の形ではない。
    grid: BTreeMap<String, BTreeMap<String, String>>,
    /// スコープカタログ。各要素が自分の名前を持つので、鍵で二重に持たない。
    scopes: Vec<ScopeMetadataDto>,
}

impl DefinitionContentDto {
    /// ドメインの 3 入力から DTO を組む (書き — テストが行を用意する口)。
    pub(super) fn of(
        graph: &StageGraph,
        grid: &ScopeGrid,
        scopes: &BTreeMap<String, ScopeMetadata>,
    ) -> DefinitionContentDto {
        DefinitionContentDto {
            graph: graph.nodes().iter().map(StageNodeDto::of).collect(),
            grid: grid
                .columns()
                .iter()
                .map(|(scope, cells)| {
                    (
                        scope.clone(),
                        cells
                            .iter()
                            .map(|(slug, action)| {
                                (
                                    slug.as_str().to_string(),
                                    plan_action_spelling(*action).to_string(),
                                )
                            })
                            .collect(),
                    )
                })
                .collect(),
            scopes: scopes.values().map(ScopeMetadataDto::of).collect(),
        }
    }

    /// ドメインの 3 入力へ戻す (読み)。
    ///
    /// # Errors
    ///
    /// 閉集合外の綴り・文法外の識別子は `Malformed`、グラフの不変条件違反 (slug 重複など)
    /// は `InvariantViolation`。
    pub(super) fn to_domain(
        &self,
    ) -> Result<(StageGraph, ScopeGrid, BTreeMap<String, ScopeMetadata>), DtoDecodeError> {
        let mut nodes = Vec::with_capacity(self.graph.len());
        for node in &self.graph {
            nodes.push(node.to_domain()?);
        }
        let graph = StageGraph::new(nodes).map_err(|_| DtoDecodeError::InvariantViolation)?;

        let mut columns = BTreeMap::new();
        for (scope, cells) in &self.grid {
            let mut column = BTreeMap::new();
            for (slug, action) in cells {
                column.insert(
                    StageSlug::parse(slug)
                        .map_err(|_| DtoDecodeError::malformed("grid_slug", slug.clone()))?,
                    plan_action_of(action, "grid_action")?,
                );
            }
            columns.insert(scope.clone(), column);
        }

        let mut scopes = BTreeMap::new();
        for scope in &self.scopes {
            let metadata = scope.to_domain()?;
            scopes.insert(metadata.name().to_string(), metadata);
        }
        Ok((graph, ScopeGrid::new(columns), scopes))
    }
}

#[cfg(test)]
mod tests {
    use super::super::definition_dto_tests::content;
    use super::*;

    #[test]
    fn the_three_inputs_survive_the_round_trip() {
        let (graph, grid, scopes) = content();
        let (decoded_graph, decoded_grid, decoded_scopes) =
            DefinitionContentDto::of(&graph, &grid, &scopes)
                .to_domain()
                .unwrap();
        assert_eq!(decoded_graph, graph);
        assert_eq!(decoded_grid, grid);
        assert_eq!(decoded_scopes, scopes);
    }

    #[test]
    fn a_grid_cell_that_cannot_be_carried_into_the_domain_is_refused() {
        // グリッドの鍵 (slug) と値 (Execute/Skip) はどちらも閉集合である。
        let (graph, grid, scopes) = content();
        let sound = DefinitionContentDto::of(&graph, &grid, &scopes);

        let mut bad_slug = sound.clone();
        bad_slug.grid = [(
            "feature".to_string(),
            [("Bad Slug".to_string(), "Execute".to_string())]
                .into_iter()
                .collect(),
        )]
        .into_iter()
        .collect();
        assert_eq!(
            bad_slug.to_domain().unwrap_err(),
            DtoDecodeError::malformed("grid_slug", "Bad Slug")
        );

        let mut bad_action = sound;
        bad_action.grid = [(
            "feature".to_string(),
            [("code-generation".to_string(), "EXECUTE".to_string())]
                .into_iter()
                .collect(),
        )]
        .into_iter()
        .collect();
        assert_eq!(
            bad_action.to_domain().unwrap_err(),
            DtoDecodeError::malformed("grid_action", "EXECUTE")
        );
    }

    #[test]
    fn a_graph_that_breaks_its_invariant_is_refused_as_an_invariant_violation() {
        // slug の重複はグラフの不変条件違反 — 綴りの問題ではないので材料は欄名を持たない。
        let (graph, grid, scopes) = content();
        let mut dto = DefinitionContentDto::of(&graph, &grid, &scopes);
        let duplicated = dto.graph.first().cloned().unwrap();
        dto.graph.push(duplicated);
        assert_eq!(
            dto.to_domain().unwrap_err(),
            DtoDecodeError::InvariantViolation
        );
    }
}
