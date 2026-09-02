//! `Recompiled` — 配布束が再コンパイルされた (内容が入れ替わった) イベントのペイロード。

use std::collections::BTreeMap;

use crate::workflow_definition::compiled_definition_id::CompiledDefinitionId;
use crate::workflow_definition::scope_grid::ScopeGrid;
use crate::workflow_definition::scope_metadata::ScopeMetadata;
use crate::workflow_definition::stage_graph::StageGraph;

/// 源 (ステージ定義・エージェント・スコープ・プラグイン) が変わり、配布束が新しい内容へ
/// 再コンパイルされた、という事実の材料。新しい内容そのものを運ぶ (`Compiled` の鏡)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recompiled {
    id: CompiledDefinitionId,
    graph: StageGraph,
    grid: ScopeGrid,
    scopes: BTreeMap<String, ScopeMetadata>,
}

impl Recompiled {
    /// 材料をそのまま束ねる。
    #[must_use]
    pub const fn new(
        id: CompiledDefinitionId,
        graph: StageGraph,
        grid: ScopeGrid,
        scopes: BTreeMap<String, ScopeMetadata>,
    ) -> Recompiled {
        Recompiled {
            id,
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

    /// 新しいステージグラフ (文書順)。
    #[must_use]
    pub const fn graph(&self) -> &StageGraph {
        &self.graph
    }

    /// 新しい EXECUTE / SKIP グリッド。
    #[must_use]
    pub const fn grid(&self) -> &ScopeGrid {
        &self.grid
    }

    /// 新しいスコープメタデータ (名前の辞書順)。
    #[must_use]
    pub const fn scopes(&self) -> &BTreeMap<String, ScopeMetadata> {
        &self.scopes
    }
}
