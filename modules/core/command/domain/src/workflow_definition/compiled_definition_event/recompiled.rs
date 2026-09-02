//! `Recompiled` — 配布束が再コンパイルされた (内容が入れ替わった) イベントのペイロード。

use std::collections::BTreeMap;

use crate::workflow_definition::compiled_definition_event_id::CompiledDefinitionEventId;
use crate::workflow_definition::compiled_definition_id::CompiledDefinitionId;
use crate::workflow_definition::scope_grid::ScopeGrid;
use crate::workflow_definition::scope_metadata::ScopeMetadata;
use crate::workflow_definition::stage_graph::StageGraph;

/// 源 (ステージ定義・エージェント・スコープ・プラグイン) が変わり、配布束が新しい内容へ
/// 再コンパイルされた、という事実の材料。新しい内容そのものを運ぶ (`Compiled` の鏡)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recompiled {
    id: CompiledDefinitionEventId,
    aggregate_id: CompiledDefinitionId,
    graph: StageGraph,
    grid: ScopeGrid,
    scopes: BTreeMap<String, ScopeMetadata>,
}

impl Recompiled {
    /// イベント識別子・配布束の識別子と材料をそのまま束ねる。
    #[must_use]
    pub const fn new(
        id: CompiledDefinitionEventId,
        aggregate_id: CompiledDefinitionId,
        graph: StageGraph,
        grid: ScopeGrid,
        scopes: BTreeMap<String, ScopeMetadata>,
    ) -> Recompiled {
        Recompiled {
            id,
            aggregate_id,
            graph,
            grid,
            scopes,
        }
    }

    /// このイベント自身の識別子 — ドメインイベントはエンティティの一種なので自前の id を
    /// 持つ (`coding-rules/domain-object-kinds.md`)。
    #[must_use]
    pub const fn id(&self) -> &CompiledDefinitionEventId {
        &self.id
    }

    /// **どの集約の事実か** — 配布束の識別子。
    #[must_use]
    pub const fn aggregate_id(&self) -> &CompiledDefinitionId {
        &self.aggregate_id
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
