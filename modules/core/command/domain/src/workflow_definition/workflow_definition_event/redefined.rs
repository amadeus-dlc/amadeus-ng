//! `Redefined` — `WorkflowDefinitionEvent::Redefined` のペイロード。

use std::collections::BTreeMap;

use crate::workflow_definition::{
    DefinitionRevision, ScopeGrid, ScopeMetadata, StageGraph, WorkflowDefinitionEventId,
    WorkflowDefinitionId,
};

/// `Redefined` のペイロード — 改訂後の内容版と内容そのもの。
///
/// 系譜 ID は `aggregate_id` として載せる — ドメインイベントはエンティティの一種であり、
/// どの集約の事実かを自分で述べる (オーナー裁定 2026-09-02)。復号境界はこれと行の `aid` を
/// 照合するので、行と payload が別々の歴史を語る破損を検出できる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redefined {
    id: WorkflowDefinitionEventId,
    aggregate_id: WorkflowDefinitionId,
    revision: DefinitionRevision,
    graph: StageGraph,
    grid: ScopeGrid,
    scopes: BTreeMap<String, ScopeMetadata>,
}

impl Redefined {
    /// イベント識別子・系譜 ID・改訂後の内容版と 3 入力のモデルを束ねる。
    #[must_use]
    pub const fn new(
        id: WorkflowDefinitionEventId,
        aggregate_id: WorkflowDefinitionId,
        revision: DefinitionRevision,
        graph: StageGraph,
        grid: ScopeGrid,
        scopes: BTreeMap<String, ScopeMetadata>,
    ) -> Redefined {
        Redefined {
            id,
            aggregate_id,
            revision,
            graph,
            grid,
            scopes,
        }
    }

    /// 改訂後の内容版。
    #[must_use]
    pub const fn revision(&self) -> &DefinitionRevision {
        &self.revision
    }

    /// 改訂後のステージグラフ。
    #[must_use]
    pub const fn graph(&self) -> &StageGraph {
        &self.graph
    }

    /// 改訂後の静的 EXECUTE / SKIP グリッド。
    #[must_use]
    pub const fn grid(&self) -> &ScopeGrid {
        &self.grid
    }

    /// 改訂後のスコープカタログ。
    #[must_use]
    pub const fn scopes(&self) -> &BTreeMap<String, ScopeMetadata> {
        &self.scopes
    }

    /// このイベント自身の識別子 — ドメインイベントはエンティティの一種なので自前の id を
    /// 持つ (`coding-rules/domain-object-kinds.md`)。
    #[must_use]
    pub const fn id(&self) -> &WorkflowDefinitionEventId {
        &self.id
    }

    /// **どの集約の事実か** — この事実が起きた定義の識別子。
    #[must_use]
    pub const fn aggregate_id(&self) -> &WorkflowDefinitionId {
        &self.aggregate_id
    }
}
