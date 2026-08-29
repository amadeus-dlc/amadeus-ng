//! `JournalEntry` — 横断読取が返すジャーナル 1 行 (entities.md / C6 journal)。

use chrono::{DateTime, Utc};

use core_command_domain::orchestration::{IntentExecutionEvent, IntentExecutionId};

use super::global_seq_nr::GlobalSeqNr;

/// 全集約横断で読んだジャーナル 1 行。
///
/// 投影 (U4) は「どの集約の何番目のイベントか」を知らないとリードモデルを描けない。
/// 行が持つ材料 — 横断通番・集約識別子・集約内通番・発生時刻・ドメインイベント — を
/// **1 つの読取レコードとして**返すのはそのためである。
///
/// # なぜ本家の封筒型を返さないのか
///
/// 本家 event-store-adapter-rs v3.0.0 の `EventEnvelope` はほぼ同じ 4 点を運ぶが、
/// [`JournalReader`] は RMU クレートが所有しており、投影核の入口にライブラリ型を出さない
/// (ADR-009 2026-08-28 / 2026-08-29 追記)。ポートの語彙を我々が所有するために、境界で
/// 我々の型へ写す ([`upstream-contracts.md`] の「食い違いは境界で変換する」)。横断通番
/// (`global_seq`) は本家の封筒にそもそも無い材料でもある。
///
/// フィールドは private。読取は境界越えのアクセサで公開する (field-visibility.md)。
///
/// [`JournalReader`]: super::journal_reader::JournalReader
/// [`upstream-contracts.md`]: https://github.com/amadeus-dlc/amadeus-ng/blob/main/aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/upstream-contracts.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    global_seq: GlobalSeqNr,
    execution_id: IntentExecutionId,
    seq_nr: usize,
    occurred_at: DateTime<Utc>,
    event: IntentExecutionEvent,
}

impl JournalEntry {
    /// 行の材料 5 点から読取レコードを組む (検証はしない — 行を読んだ側が既に済ませている)。
    #[must_use]
    pub const fn new(
        global_seq: GlobalSeqNr,
        execution_id: IntentExecutionId,
        seq_nr: usize,
        occurred_at: DateTime<Utc>,
        event: IntentExecutionEvent,
    ) -> JournalEntry {
        JournalEntry {
            global_seq,
            execution_id,
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

    /// この行が属する集約の識別子。
    #[must_use]
    pub const fn execution_id(&self) -> &IntentExecutionId {
        &self.execution_id
    }

    /// 集約内で 1 から単調増加する順序番号。
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
    pub const fn event(&self) -> &IntentExecutionEvent {
        &self.event
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_command_domain::orchestration::{IntentExecutionEvent, Parked};
    use core_command_domain::workflow_definition::StageSlug;

    fn intent() -> IntentExecutionId {
        IntentExecutionId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap()
    }

    fn at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-08-29T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn entry(global: u64, seq_nr: usize) -> JournalEntry {
        JournalEntry::new(
            GlobalSeqNr::new(global),
            intent(),
            seq_nr,
            at(),
            IntentExecutionEvent::Unparked,
        )
    }

    #[test]
    fn the_entry_carries_every_material_the_journal_row_had() {
        let row = entry(7, 3);
        assert_eq!(row.global_seq(), GlobalSeqNr::new(7));
        assert_eq!(row.execution_id(), &intent());
        assert_eq!(row.seq_nr(), 3);
        assert_eq!(row.occurred_at(), &at());
        assert_eq!(row.event(), &IntentExecutionEvent::Unparked);
    }

    #[test]
    fn the_entry_keeps_the_event_it_was_given() {
        let parked =
            IntentExecutionEvent::Parked(Parked::new(StageSlug::parse("intent-capture").unwrap()));
        let row = JournalEntry::new(GlobalSeqNr::new(1), intent(), 1, at(), parked.clone());
        assert_eq!(row.event(), &parked);
    }

    #[test]
    fn entries_compare_by_value() {
        assert_eq!(entry(1, 1), entry(1, 1));
        assert_ne!(entry(1, 1), entry(2, 1));
        assert_ne!(entry(1, 1), entry(1, 2));
    }
}
