//! `DefinitionEntry` — 横断読取が返す**定義ストリーム**のジャーナル 1 行。

use chrono::{DateTime, Utc};

use core_command_domain::workflow_definition::{WorkflowDefinitionEvent, WorkflowDefinitionId};

use super::global_seq_nr::GlobalSeqNr;

/// 全集約横断で読んだ定義ジャーナル 1 行。
///
/// 実行の行 ([`JournalEntry`]) と同じ 5 点 — 横断通番・集約識別子・集約内通番・発生時刻・
/// ドメインイベント — を運ぶ。型が別なのは集約が別だからであり、投影核が
/// 「どの集約の何番目のイベントか」を知らないとリードモデルを描けないという理由は同じで
/// ある。
///
/// # `Redefined` は識別子を運ばない
///
/// 改訂イベントは自集約の識別子を複製しない (`coding-rules/aggregate-references.md`) ので、
/// 定義 id の出所は**行の `aid` 列**である。誕生 (`Defined`) だけは payload にも系譜 ID を
/// 持つので、読取は両者の一致を検査してからこの型を組む — 食い違う行はどちらかが嘘を
/// ついており、解釈せず `Corrupt` で止まる。
///
/// フィールドは private。読取は境界越えのアクセサで公開する (field-visibility.md)。
///
/// [`JournalEntry`]: super::journal_entry::JournalEntry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionEntry {
    global_seq: GlobalSeqNr,
    definition_id: WorkflowDefinitionId,
    seq_nr: usize,
    occurred_at: DateTime<Utc>,
    event: WorkflowDefinitionEvent,
}

impl DefinitionEntry {
    /// 行の材料 5 点から読取レコードを組む (検証はしない — 行を読んだ側が既に済ませている)。
    #[must_use]
    pub const fn new(
        global_seq: GlobalSeqNr,
        definition_id: WorkflowDefinitionId,
        seq_nr: usize,
        occurred_at: DateTime<Utc>,
        event: WorkflowDefinitionEvent,
    ) -> DefinitionEntry {
        DefinitionEntry {
            global_seq,
            definition_id,
            seq_nr,
            occurred_at,
            event,
        }
    }

    /// 全集約横断の通番 (チェックポイントが進む単位)。
    #[must_use]
    pub const fn global_seq(&self) -> GlobalSeqNr {
        self.global_seq
    }

    /// この行が属する定義集約の識別子 (行の `aid` 列由来)。
    #[must_use]
    pub const fn definition_id(&self) -> &WorkflowDefinitionId {
        &self.definition_id
    }

    /// 集約内で 1 から単調増加する順序番号 (`Defined` は必ず 1)。
    #[must_use]
    pub const fn seq_nr(&self) -> usize {
        self.seq_nr
    }

    /// イベントの発生時刻 (ドメイン供給値。ストア刻印ではない)。
    #[must_use]
    pub const fn occurred_at(&self) -> &DateTime<Utc> {
        &self.occurred_at
    }

    /// ドメインイベント本体。
    #[must_use]
    pub const fn event(&self) -> &WorkflowDefinitionEvent {
        &self.event
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use core_command_domain::workflow_definition::{
        Defined, DefinitionRevision, ExecutionKind, PhaseId, Redefined, ScopeGrid, StageGraph,
        StageMode, StageNodeBuilder, StageNumber, StageSlug, WorkflowDefinitionEventId,
    };

    /// b40 のテスト用固定イベント識別子 (定義面)。
    fn definition_event_id() -> WorkflowDefinitionEventId {
        WorkflowDefinitionEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0003").unwrap()
    }

    fn definition_id() -> WorkflowDefinitionId {
        WorkflowDefinitionId::parse("claude").unwrap()
    }

    fn at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-09-02T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn revision(fill: char) -> DefinitionRevision {
        DefinitionRevision::parse(&format!("sha256:{}", fill.to_string().repeat(64))).unwrap()
    }

    fn graph() -> StageGraph {
        StageGraph::new(vec![
            StageNodeBuilder::new(
                StageSlug::parse("state-init").unwrap(),
                StageNumber::parse("0.1").unwrap(),
                "State Init".to_string(),
                PhaseId::Initialization,
                ExecutionKind::Always,
                StageMode::Inline,
            )
            .build(),
        ])
        .unwrap()
    }

    fn defined() -> WorkflowDefinitionEvent {
        WorkflowDefinitionEvent::Defined(Defined::new(
            definition_event_id(),
            definition_id(),
            revision('0'),
            graph(),
            ScopeGrid::from_graph(&graph()),
            BTreeMap::new(),
        ))
    }

    fn redefined() -> WorkflowDefinitionEvent {
        WorkflowDefinitionEvent::Redefined(Redefined::new(
            definition_event_id(),
            definition_id(),
            revision('1'),
            graph(),
            ScopeGrid::from_graph(&graph()),
            BTreeMap::new(),
        ))
    }

    fn entry(global: u64, seq_nr: usize, event: WorkflowDefinitionEvent) -> DefinitionEntry {
        DefinitionEntry::new(
            GlobalSeqNr::new(global),
            definition_id(),
            seq_nr,
            at(),
            event,
        )
    }

    #[test]
    fn the_entry_carries_every_material_the_journal_row_had() {
        let row = entry(7, 1, defined());
        assert_eq!(row.global_seq(), GlobalSeqNr::new(7));
        assert_eq!(row.definition_id(), &definition_id());
        assert_eq!(row.seq_nr(), 1);
        assert_eq!(row.occurred_at(), &at());
        assert_eq!(row.event(), &defined());
    }

    #[test]
    fn the_entry_keeps_the_event_it_was_given() {
        // 改訂は識別子を運ばないので、定義 id は行の `aid` 由来のまま残る。
        let row = entry(8, 2, redefined());
        assert_eq!(row.event(), &redefined());
        assert_eq!(row.definition_id(), &definition_id());
    }

    #[test]
    fn entries_compare_by_value() {
        assert_eq!(entry(1, 1, defined()), entry(1, 1, defined()));
        assert_ne!(entry(1, 1, defined()), entry(2, 1, defined()));
        assert_ne!(entry(1, 1, defined()), entry(1, 2, defined()));
        assert_ne!(entry(1, 1, defined()), entry(1, 1, redefined()));
    }
}
