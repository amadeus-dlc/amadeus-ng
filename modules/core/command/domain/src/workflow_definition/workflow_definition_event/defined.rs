//! `Defined` — `WorkflowDefinitionEvent::Defined` のペイロード。

use std::collections::BTreeMap;

use crate::workflow_definition::{
    DefinitionRevision, ScopeGrid, ScopeMetadata, StageGraph, WorkflowDefinitionId,
};

/// `Defined` のペイロード — 確立された定義の系譜 ID・内容版・内容そのもの。
///
/// 系譜 ID を運ぶのは genesis だけである (以後の改訂で識別子は変わらない)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Defined {
    id: WorkflowDefinitionId,
    revision: DefinitionRevision,
    graph: StageGraph,
    grid: ScopeGrid,
    scopes: BTreeMap<String, ScopeMetadata>,
}

impl Defined {
    /// 系譜 ID・内容版・3 入力のモデルを束ねる。
    #[must_use]
    pub const fn new(
        id: WorkflowDefinitionId,
        revision: DefinitionRevision,
        graph: StageGraph,
        grid: ScopeGrid,
        scopes: BTreeMap<String, ScopeMetadata>,
    ) -> Defined {
        Defined {
            id,
            revision,
            graph,
            grid,
            scopes,
        }
    }

    /// 確立された定義の系譜 ID (内容が変わっても不変 — ADR-008)。
    #[must_use]
    pub const fn id(&self) -> &WorkflowDefinitionId {
        &self.id
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
