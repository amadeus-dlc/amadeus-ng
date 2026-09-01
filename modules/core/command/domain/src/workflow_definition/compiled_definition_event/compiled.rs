//! `Compiled` — 配布束の誕生イベントのペイロード。

use std::collections::BTreeMap;

use crate::workflow_definition::compiled_definition_id::CompiledDefinitionId;
use crate::workflow_definition::definition_revision::DefinitionRevision;
use crate::workflow_definition::scope_grid::ScopeGrid;
use crate::workflow_definition::scope_metadata::ScopeMetadata;
use crate::workflow_definition::stage_graph::StageGraph;

/// 配布束がコンパイルされて存在するようになった、という事実の材料。
///
/// **内容そのもの**を運ぶ — イベントが材料の複製を運ぶのは歴史であり違反ではない
/// (`coding-rules/aggregate-references.md`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compiled {
    id: CompiledDefinitionId,
    revision: DefinitionRevision,
    graph: StageGraph,
    grid: ScopeGrid,
    scopes: BTreeMap<String, ScopeMetadata>,
}

impl Compiled {
    /// 材料をそのまま束ねる。
    #[must_use]
    pub const fn new(
        id: CompiledDefinitionId,
        revision: DefinitionRevision,
        graph: StageGraph,
        grid: ScopeGrid,
        scopes: BTreeMap<String, ScopeMetadata>,
    ) -> Compiled {
        Compiled {
            id,
            revision,
            graph,
            grid,
            scopes,
        }
    }

    /// 配布束の識別子。
    #[must_use]
    pub const fn id(&self) -> &CompiledDefinitionId {
        &self.id
    }

    /// 内容ダイジェスト。
    #[must_use]
    pub const fn revision(&self) -> &DefinitionRevision {
        &self.revision
    }

    /// ステージグラフ (文書順)。
    #[must_use]
    pub const fn graph(&self) -> &StageGraph {
        &self.graph
    }

    /// EXECUTE / SKIP グリッド。
    #[must_use]
    pub const fn grid(&self) -> &ScopeGrid {
        &self.grid
    }

    /// スコープメタデータ (名前の辞書順)。
    #[must_use]
    pub const fn scopes(&self) -> &BTreeMap<String, ScopeMetadata> {
        &self.scopes
    }
}
