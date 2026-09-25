//! `WorkspaceDoctorJournalReader` の SQLite 実装 — 本家の `journal` 表から自己診断の事実を読む。
//!
//! 読むのは `journal` 表 1 つだけで、リードモデルの表には触れない。カーソルは `journal` の
//! `rowid` (全集約横断の通番 — `JournalReaderImpl` と同じ前提: 追記専用なのでコミット順に
//! 単調増加する)。

use chrono::DateTime;
use core_command_domain::workspace::{
    DoctorCheck, DoctorCheckId, DoctorChecks, HookHealthTarget, WorkspaceDoctorEvent,
    WorkspaceDoctorEventId, WorkspaceDoctorId,
};
use rusqlite::Connection;
use serde::Deserialize;

use super::journal_reader_impl::corrupt_error;
use super::store_failure::SqliteResultExt;
use super::{
    CorruptCause, GlobalSeqNr, JournalReadError, WorkspaceDoctorJournalEntry,
    WorkspaceDoctorJournalReader,
};

/// 読む行の型判別子 — 書く側 (`core-command-interface-adapter`) と対になる。
const MANIFEST: &str = "workspace-doctor-event/1";

/// 集約に属さない失敗 (カーソルの値域) の識別子欄に置く印。
const NO_AGGREGATE: &str = "-";

/// 位置より後の自分の行 (位置の昇順)。
const SELECT_AFTER: &str = "SELECT rowid, aid, seq_nr, occurred_at, payload FROM journal
     WHERE manifest = ?1 AND rowid > ?2 ORDER BY rowid";

/// 位置までの自分の行 (位置の昇順)。
const SELECT_THROUGH: &str = "SELECT rowid, aid, seq_nr, occurred_at, payload FROM journal
     WHERE manifest = ?1 AND rowid <= ?2 ORDER BY rowid";

/// ジャーナル行の payload — **読む側の DTO** (書く側の DTO とは別に持つ)。
#[derive(Debug, Clone, Deserialize)]
struct EventWire {
    id: String,
    aggregate_id: String,
    target: String,
    checks: Vec<CheckWire>,
}

/// 1 行ぶんの payload。
#[derive(Debug, Clone, Deserialize)]
struct CheckWire {
    check_id: String,
    passed: bool,
    label: String,
    fix: Option<String>,
}

/// `journal` 表の生の 1 行。
struct JournalRow {
    rowid: i64,
    aggregate_id: String,
    seq_nr: i64,
    occurred_at: i64,
    payload: Vec<u8>,
}

/// 自己診断の事実をジャーナルから読む実装。状態を持たない。
#[derive(Debug, Clone, Copy, Default)]
pub struct WorkspaceDoctorJournalReaderImpl;

impl WorkspaceDoctorJournalReader for WorkspaceDoctorJournalReaderImpl {
    fn events_after(
        &self,
        connection: &Connection,
        after: GlobalSeqNr,
    ) -> Result<Vec<WorkspaceDoctorJournalEntry>, JournalReadError> {
        scan(connection, SELECT_AFTER, after)
    }

    fn events_through(
        &self,
        connection: &Connection,
        to: GlobalSeqNr,
    ) -> Result<Vec<WorkspaceDoctorJournalEntry>, JournalReadError> {
        scan(connection, SELECT_THROUGH, to)
    }
}

/// 自分の manifest の行を `bound` で絞って読み、事実へ写す。
fn scan(
    connection: &Connection,
    sql: &str,
    bound: GlobalSeqNr,
) -> Result<Vec<WorkspaceDoctorJournalEntry>, JournalReadError> {
    let bound = i64::try_from(bound.to_u64())
        .map_err(|_| corrupt_error(NO_AGGREGATE, None, CorruptCause::InvariantViolation))?;
    let mut statement = connection.prepare(sql).at_connection(connection)?;
    let rows = statement
        .query_map(rusqlite::params![MANIFEST, bound], |row| {
            Ok(JournalRow {
                rowid: row.get(0)?,
                aggregate_id: row.get(1)?,
                seq_nr: row.get(2)?,
                occurred_at: row.get(3)?,
                payload: row.get(4)?,
            })
        })
        .at_connection(connection)?;
    let mut entries = Vec::new();
    for row in rows {
        let row = row.at_connection(connection)?;
        entries.push(entry(&row)?);
    }
    Ok(entries)
}

/// 1 行を読取レコードへ写す (検査付き)。
fn entry(row: &JournalRow) -> Result<WorkspaceDoctorJournalEntry, JournalReadError> {
    let invalid = || corrupt_error(&row.aggregate_id, None, CorruptCause::InvariantViolation);
    let position = u64::try_from(row.rowid).map_err(|_| invalid())?;
    let seq_nr = usize::try_from(row.seq_nr).map_err(|_| invalid())?;
    let wire: EventWire = serde_json::from_slice(&row.payload)
        .map_err(|_| corrupt_error(&row.aggregate_id, None, CorruptCause::UndecodablePayload))?;
    // 行の `aid` 列と payload が名乗る集約が食い違う行は、どちらの集約の履歴にも置けない。
    if wire.aggregate_id != row.aggregate_id {
        return Err(corrupt_error(
            &row.aggregate_id,
            Some(seq_nr),
            CorruptCause::InvariantViolation,
        ));
    }
    Ok(WorkspaceDoctorJournalEntry::new(
        GlobalSeqNr::new(position),
        seq_nr,
        DateTime::from_timestamp_nanos(row.occurred_at),
        decode(&wire)?,
    ))
}

/// 読取 DTO をドメインイベントへ写す (検査付き — 迂回する構築口は無い)。
fn decode(wire: &EventWire) -> Result<WorkspaceDoctorEvent, JournalReadError> {
    let undecodable = || corrupt_error(&wire.aggregate_id, None, CorruptCause::UndecodablePayload);
    let id = WorkspaceDoctorEventId::parse(&wire.id).map_err(|_| undecodable())?;
    let aggregate_id = WorkspaceDoctorId::parse(&wire.aggregate_id).map_err(|_| undecodable())?;
    let target = HookHealthTarget::parse(&wire.target).map_err(|_| undecodable())?;
    let mut checks = Vec::with_capacity(wire.checks.len());
    for row in &wire.checks {
        checks.push(DoctorCheck::new(
            DoctorCheckId::parse(&row.check_id).map_err(|_| undecodable())?,
            row.passed,
            row.label.clone(),
            row.fix.clone(),
        ));
    }
    WorkspaceDoctorEvent::new(id, aggregate_id, target, DoctorChecks::new(checks))
        .map_err(|_| undecodable())
}

#[cfg(test)]
mod tests {
    // テストは固定長フィクスチャの添字参照を許容 (オーナー規約)。
    #![allow(clippy::indexing_slicing)]

    use super::*;

    const EVENT_ID: &str = "0199aaaa-bbbb-7ccc-8ddd-eeeeffff0001";
    const REPORT_ID: &str =
        "workspace-doctor:fd9fe7c1c349372f408ee6fd4060f95b4c7b3e4c814727bf3eff713c34922ea0";

    /// 本家 `journal` 表の最小の殻 (読む列だけ)。
    fn journal() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE journal (
                   pkey TEXT NOT NULL, skey TEXT NOT NULL, aid TEXT NOT NULL,
                   seq_nr INTEGER NOT NULL, payload BLOB NOT NULL, occurred_at INTEGER NOT NULL,
                   manifest TEXT NOT NULL DEFAULT '', PRIMARY KEY (pkey, skey))",
            )
            .unwrap();
        connection
    }

    /// 本家シリアライザと同じ形式の payload バイト列。
    #[allow(
        clippy::disallowed_methods,
        reason = "本家シリアライザと同形式のフィクスチャ生成 (BR1.7 の射程外)"
    )]
    fn payload() -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "id": EVENT_ID,
            "aggregate_id": REPORT_ID,
            "target": "spaces/default/intents",
            "checks": [{"check_id": "D1.a", "passed": true, "label": "bun", "fix": null}],
        }))
        .unwrap()
    }

    fn append(connection: &Connection, skey: &str, seq_nr: i64, manifest: &str, bytes: &[u8]) {
        connection
            .execute(
                "INSERT INTO journal(pkey, skey, aid, seq_nr, payload, occurred_at, manifest)
                 VALUES ('p', ?1, ?2, ?3, ?4, 0, ?5)",
                rusqlite::params![skey, REPORT_ID, seq_nr, bytes, manifest],
            )
            .unwrap();
    }

    fn positions(entries: &[WorkspaceDoctorJournalEntry]) -> Vec<u64> {
        entries
            .iter()
            .map(|entry| entry.position().to_u64())
            .collect()
    }

    #[test]
    fn after_reads_only_our_rows_beyond_the_position_in_order() {
        let connection = journal();
        append(&connection, "1", 1, MANIFEST, &payload());
        append(&connection, "2", 1, "hook-health-event/1", b"{}");
        append(&connection, "3", 2, MANIFEST, &payload());
        let reader = WorkspaceDoctorJournalReaderImpl;
        assert_eq!(
            positions(&reader.events_after(&connection, GlobalSeqNr::ZERO).unwrap()),
            [1, 3],
            "他の manifest の行は読まない"
        );
        assert_eq!(
            positions(
                &reader
                    .events_after(&connection, GlobalSeqNr::new(1))
                    .unwrap()
            ),
            [3]
        );
        assert!(
            reader
                .events_after(&connection, GlobalSeqNr::new(3))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn through_reads_our_rows_up_to_and_including_the_position() {
        let connection = journal();
        append(&connection, "1", 1, MANIFEST, &payload());
        append(&connection, "2", 2, MANIFEST, &payload());
        let reader = WorkspaceDoctorJournalReaderImpl;
        let entries = reader
            .events_through(&connection, GlobalSeqNr::new(1))
            .unwrap();
        assert_eq!(positions(&entries), [1]);
        assert_eq!(entries[0].seq_nr(), 1);
        assert_eq!(entries[0].event().aggregate_id().as_str(), REPORT_ID);
    }

    #[test]
    fn a_payload_that_is_not_ours_is_corrupt_rather_than_skipped() {
        let connection = journal();
        append(&connection, "1", 1, MANIFEST, br#"{"nope":1}"#);
        let error = WorkspaceDoctorJournalReaderImpl
            .events_after(&connection, GlobalSeqNr::ZERO)
            .unwrap_err();
        assert_eq!(
            error,
            corrupt_error(REPORT_ID, None, CorruptCause::UndecodablePayload)
        );
    }

    #[test]
    fn a_row_whose_column_and_payload_name_different_aggregates_is_corrupt() {
        let connection = journal();
        connection
            .execute(
                "INSERT INTO journal(pkey, skey, aid, seq_nr, payload, occurred_at, manifest)
                 VALUES ('p', '1', 'someone-else', 1, ?1, 0, ?2)",
                rusqlite::params![payload(), MANIFEST],
            )
            .unwrap();
        let error = WorkspaceDoctorJournalReaderImpl
            .events_after(&connection, GlobalSeqNr::ZERO)
            .unwrap_err();
        assert_eq!(
            error,
            corrupt_error("someone-else", Some(1), CorruptCause::InvariantViolation)
        );
    }

    #[test]
    fn a_missing_journal_is_an_io_failure() {
        let connection = Connection::open_in_memory().unwrap();
        let error = WorkspaceDoctorJournalReaderImpl
            .events_after(&connection, GlobalSeqNr::ZERO)
            .unwrap_err();
        assert!(
            matches!(error, JournalReadError::Io { .. }),
            "実際: {error}"
        );
    }
}
