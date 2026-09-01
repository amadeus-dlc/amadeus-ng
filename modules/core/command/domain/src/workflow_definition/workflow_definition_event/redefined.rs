//! `Redefined` — `WorkflowDefinitionEvent::Redefined` のペイロード。

use std::collections::BTreeMap;

use crate::workflow_definition::{DefinitionRevision, ScopeGrid, ScopeMetadata, StageGraph};

/// `Redefined` のペイロード — 改訂後の内容版と内容そのもの。
///
/// 系譜 ID は載せない — 改訂は既存のストリームに追記される事実であり、どの集約に起きたかは
/// ジャーナル行の集約識別子が持つ (`coding-rules/aggregate-references.md` と同じ理由で、
/// 変異イベントは自集約の識別子を複製しない)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redefined {
    revision: DefinitionRevision,
    graph: StageGraph,
    grid: ScopeGrid,
    scopes: BTreeMap<String, ScopeMetadata>,
}

impl Redefined {
    /// 改訂後の内容版と 3 入力のモデルを束ねる。
    #[must_use]
    pub const fn new(
        revision: DefinitionRevision,
        graph: StageGraph,
        grid: ScopeGrid,
        scopes: BTreeMap<String, ScopeMetadata>,
    ) -> Redefined {
        Redefined {
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
}
