//! `Defined` — `WorkflowDefinitionEvent::Defined` のペイロード。

use std::collections::BTreeMap;

use crate::workflow_definition::{
    DefinitionRevision, ScopeGrid, ScopeMetadata, StageGraph, WorkflowDefinitionEventId,
    WorkflowDefinitionId,
};

/// `Defined` のペイロード — 確立された定義の系譜 ID・内容版・内容そのもの。
///
/// 系譜 ID は全変種が `aggregate_id` として運ぶ (イベントはエンティティ — オーナー裁定
/// 2026-09-02)。かつては genesis だけが持ち、改訂は行の `aid` に頼っていた。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Defined {
    id: WorkflowDefinitionEventId,
    aggregate_id: WorkflowDefinitionId,
    revision: DefinitionRevision,
    graph: StageGraph,
    grid: ScopeGrid,
    scopes: BTreeMap<String, ScopeMetadata>,
}

impl Defined {
    /// イベント識別子・系譜 ID・内容版・3 入力のモデルを束ねる。
    #[must_use]
    pub const fn new(
        id: WorkflowDefinitionEventId,
        aggregate_id: WorkflowDefinitionId,
        revision: DefinitionRevision,
        graph: StageGraph,
        grid: ScopeGrid,
        scopes: BTreeMap<String, ScopeMetadata>,
    ) -> Defined {
        Defined {
            id,
            aggregate_id,
            revision,
            graph,
            grid,
            scopes,
        }
    }

    /// このイベント自身の識別子 — ドメインイベントはエンティティの一種なので自前の id を
    /// 持つ (`coding-rules/domain-object-kinds.md`)。
    #[must_use]
    pub const fn id(&self) -> &WorkflowDefinitionEventId {
        &self.id
    }

    /// **どの集約の事実か** — 確立された定義の系譜 ID (内容が変わっても不変 — ADR-008)。
    #[must_use]
    pub const fn aggregate_id(&self) -> &WorkflowDefinitionId {
        &self.aggregate_id
    }

    /// 確立された時点の内容版 (3 入力の内容ダイジェスト)。
    #[must_use]
    pub const fn revision(&self) -> &DefinitionRevision {
        &self.revision
    }

    /// 確立された時点のステージグラフ (文書順を保持したまま)。
    #[must_use]
    pub const fn graph(&self) -> &StageGraph {
        &self.graph
    }

    /// 確立された時点の静的 EXECUTE / SKIP グリッド。
    #[must_use]
    pub const fn grid(&self) -> &ScopeGrid {
        &self.grid
    }

    /// 確立された時点のスコープカタログ (スコープ名の辞書順)。
    #[must_use]
    pub const fn scopes(&self) -> &BTreeMap<String, ScopeMetadata> {
        &self.scopes
    }
}
