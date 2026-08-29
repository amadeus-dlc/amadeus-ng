//! `WorkflowDefinitionEvent` — 定義集約に起きた事実 (C5 の語彙に相当する定義側の 1 本目)。
//!
//! `WorkflowDefinition` は集約である (オーナー裁定 2026-08-29)。集約のファクトリは
//! **(集約インスタンス, 誕生イベント) の対を返す**ことが必須なので
//! (coding-rules/aggregate-commands.md)、定義側にもイベント語彙を持たせる。
//!
//! 現スコープでは**ジャーナルへ接続しない** — 型と形だけを規則へ適合させ、イベントを
//! `store` する先（定義の変異取込）は後続 intent の課題である。

use super::definition_revision::DefinitionRevision;
use super::workflow_definition_id::WorkflowDefinitionId;

/// 定義集約に起きた事実。現在は genesis の 1 変種だけである。
///
/// 変異（スコープの追加・グリッドの改訂など）が要件化したら、差分を運ぶ変種
/// (`ScopeComposed` 等) がここへ増える。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowDefinitionEvent {
    /// 定義が確立された。
    Defined(Defined),
}

/// `Defined` のペイロード — 確立された定義の系譜 ID と内容版。
///
/// **内容そのもの (stage graph / scope grid / scopes) は焼かない。** 実ファイル
/// (`stage-graph.json` / `scope-grid.json` / `scopes/*.md`) がこの集約のリードモデルであり、
/// 内容の正本はそちらである。イベントが運ぶのは「どの系譜のどの内容版が確立されたか」と
/// いう事実だけで、内容の変更は将来の差分イベントが運ぶ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Defined {
    id: WorkflowDefinitionId,
    revision: DefinitionRevision,
}

impl Defined {
    /// 系譜 ID と内容版を束ねる。
    #[must_use]
    pub const fn new(id: WorkflowDefinitionId, revision: DefinitionRevision) -> Defined {
        Defined { id, revision }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_definition::{DefinitionRevision, WorkflowDefinitionId};

    fn id() -> WorkflowDefinitionId {
        WorkflowDefinitionId::parse("claude").expect("テストの定義 id")
    }

    fn revision() -> DefinitionRevision {
        DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("テストの revision")
    }

    #[test]
    fn the_defined_payload_carries_the_lineage_and_the_content_revision() {
        let defined = Defined::new(id(), revision());
        assert_eq!(defined.id(), &id());
        assert_eq!(defined.revision(), &revision());
    }

    #[test]
    fn events_compare_by_value() {
        let first = WorkflowDefinitionEvent::Defined(Defined::new(id(), revision()));
        let second = WorkflowDefinitionEvent::Defined(Defined::new(id(), revision()));
        assert_eq!(first, second);
        let other = WorkflowDefinitionEvent::Defined(Defined::new(
            WorkflowDefinitionId::parse("kiro").expect("テストの定義 id"),
            revision(),
        ));
        assert_ne!(first, other);
    }
}
