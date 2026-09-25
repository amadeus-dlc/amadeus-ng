//! 自己診断 (`WorkspaceDoctor`) 専用のジャーナル読取・投影。他の履歴を解釈しない。
//!
//! 二層構造は他の面と同じである — 取得ループ (manifest で自分の行だけを引き、集約を
//! `replay` で起こす) と、集約のクエリ (`passed` / `failed` / `exit_code` / 表示順の行) の
//! **写し**を行にする投影だけを持つ。判断はここに 1 つも無い
//! (`coding-rules/cqrs-boundaries.md` 規則 3 の 2026-09-02 追記)。
//!
//! 行の値はクエリ側がそのまま表示する — 数える・並べ替える・文言を組むことをクエリ側に
//! させないため、集計と終了コードまで焼き込む (規則 6)。

use super::journal_reader_impl::corrupt_error;
use super::{CorruptCause, JournalReadError, ReadModelUpdater};
use crate::read_tables::doctor_check;
use chrono::{DateTime, Utc};
use core_command_domain::workspace::{
    DoctorCheck, DoctorCheckId, DoctorChecks, HookHealthTarget, WorkspaceDoctor,
    WorkspaceDoctorEvent, WorkspaceDoctorEventId, WorkspaceDoctorId,
};
use core_infrastructure::collections::FirstClassCollection as _;
use rusqlite::{Connection, OpenFlags, params};
use serde::Deserialize;

/// 読む行の型判別子 — 書く側 (`core-command-interface-adapter`) と対になる。
const MANIFEST: &str = "workspace-doctor-event/1";
/// この面のチェックポイント名。
const PROJECTION: &str = "workspace-doctor";
/// 自分の行だけを全履歴から引く。
const SELECT_EVENTS: &str =
    "SELECT aid, seq_nr, occurred_at, payload FROM journal WHERE manifest=?1 ORDER BY rowid";
/// 表の DDL (この面が所有する 2 表とチェックポイント)。
const CREATE_TABLES: &str = "\
CREATE TABLE IF NOT EXISTS read_doctor_report (
  id        TEXT    PRIMARY KEY,
  target    TEXT    NOT NULL,
  passed    INTEGER NOT NULL,
  failed    INTEGER NOT NULL,
  exit_code INTEGER NOT NULL,
  seq_nr    INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS read_doctor_report_target_idx ON read_doctor_report (target);
CREATE TABLE IF NOT EXISTS read_doctor_check (
  id        TEXT    PRIMARY KEY,
  report_id TEXT    NOT NULL,
  position  INTEGER NOT NULL,
  check_id  TEXT    NOT NULL,
  passed    INTEGER NOT NULL,
  label     TEXT    NOT NULL,
  fix       TEXT
);
CREATE INDEX IF NOT EXISTS read_doctor_check_report_idx ON read_doctor_check (report_id);
CREATE UNIQUE INDEX IF NOT EXISTS read_doctor_check_order_idx
  ON read_doctor_check (report_id, position);
CREATE TABLE IF NOT EXISTS workspace_doctor_projection_checkpoint (
  projection TEXT    PRIMARY KEY,
  last_seq   INTEGER NOT NULL
);";

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

/// 自己診断の共有 DB 投影器。
#[derive(Debug)]
pub struct WorkspaceDoctorReadModelUpdater {
    connection: Connection,
}

impl ReadModelUpdater for WorkspaceDoctorReadModelUpdater {
    type Error = JournalReadError;

    /// 自分の manifest だけを全履歴から再投影する。
    ///
    /// 内部は同期 I/O だけである。非同期なのは共通契約の境界だけ。
    ///
    /// # Errors
    ///
    /// 履歴の復号・再生・書込に失敗した場合。
    async fn update_read_models(&mut self) -> Result<(), JournalReadError> {
        let replayed = self.replay_all()?;
        self.write(&replayed)
    }
}

impl WorkspaceDoctorReadModelUpdater {
    /// 既存の共有 DB へ接続する。DB 自体は作らない。
    ///
    /// 一時ストア (`file:...?mode=memory&cache=shared`) も同じ口で開けるよう URI を許す —
    /// 診断は初回状態でファイルを 1 つも作らない (契約 C7 DC1)。
    ///
    /// # Errors
    ///
    /// 共有 DB へ接続できない場合。
    pub fn open(path: &std::path::Path) -> Result<Self, JournalReadError> {
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_URI
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|_| corrupt_error(PROJECTION, None, CorruptCause::InvariantViolation))?;
        Ok(Self { connection })
    }

    /// 集約 ID ごとに履歴を束ねて再生する (集約 ID の辞書順)。
    fn replay_all(&self) -> Result<Vec<WorkspaceDoctor>, JournalReadError> {
        let mut statement = self
            .connection
            .prepare(SELECT_EVENTS)
            .map_err(|_| corrupt_error(PROJECTION, None, CorruptCause::InvariantViolation))?;
        let rows = statement
            .query_map([MANIFEST], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                ))
            })
            .map_err(|_| corrupt_error(PROJECTION, None, CorruptCause::InvariantViolation))?;
        let mut streams: std::collections::BTreeMap<
            String,
            Vec<(WorkspaceDoctorEvent, usize, DateTime<Utc>)>,
        > = std::collections::BTreeMap::new();
        for row in rows {
            let (aid, seq_nr, occurred_at, payload) =
                row.map_err(|_| corrupt_error(PROJECTION, None, CorruptCause::UndecodablePayload))?;
            let wire: EventWire = serde_json::from_slice(&payload)
                .map_err(|_| corrupt_error(&aid, None, CorruptCause::UndecodablePayload))?;
            let event = decode(&wire)?;
            let seq = usize::try_from(seq_nr)
                .map_err(|_| corrupt_error(&aid, None, CorruptCause::InvariantViolation))?;
            streams.entry(aid).or_default().push((
                event,
                seq,
                DateTime::from_timestamp_nanos(occurred_at),
            ));
        }
        let mut replayed = Vec::with_capacity(streams.len());
        for (aid, mut events) in streams {
            events.sort_by_key(|(_, seq, _)| *seq);
            let (genesis, genesis_seq, genesis_at) = events
                .first()
                .cloned()
                .ok_or_else(|| corrupt_error(&aid, None, CorruptCause::InvariantViolation))?;
            if genesis_seq != 1 || genesis.aggregate_id().as_str() != aid {
                return Err(corrupt_error(
                    &aid,
                    Some(genesis_seq),
                    CorruptCause::InvariantViolation,
                ));
            }
            let snapshot = WorkspaceDoctor::new(
                genesis.aggregate_id().clone(),
                genesis.target().clone(),
                genesis.checks().clone(),
                genesis_seq,
                0,
                genesis_at,
            )
            .map_err(|_| {
                corrupt_error(&aid, Some(genesis_seq), CorruptCause::InvariantViolation)
            })?;
            replayed.push(WorkspaceDoctor::replay(
                snapshot,
                events.into_iter().skip(1),
            ));
        }
        Ok(replayed)
    }

    /// 全行を差し替え、チェックポイントを 1 トランザクションで進める。
    fn write(&mut self, replayed: &[WorkspaceDoctor]) -> Result<(), JournalReadError> {
        let transaction = self
            .connection
            .transaction()
            .map_err(|_| corrupt_error(PROJECTION, None, CorruptCause::InvariantViolation))?;
        transaction
            .execute_batch(CREATE_TABLES)
            .map_err(|_| corrupt_error(PROJECTION, None, CorruptCause::InvariantViolation))?;
        for table in ["read_doctor_check", "read_doctor_report"] {
            transaction
                .execute(&format!("DELETE FROM {table}"), [])
                .map_err(|_| corrupt_error(PROJECTION, None, CorruptCause::InvariantViolation))?;
        }
        let mut last_seq = 0_i64;
        for doctor in replayed {
            let id = doctor.id().as_str();
            let seq_nr = i64::try_from(doctor.seq_nr())
                .map_err(|_| corrupt_error(id, None, CorruptCause::InvariantViolation))?;
            last_seq = last_seq.max(seq_nr);
            transaction
                .execute(
                    "INSERT INTO read_doctor_report VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        id,
                        doctor.target().relative_directory(),
                        i64::try_from(doctor.checks().passed()).map_err(|_| corrupt_error(
                            id,
                            None,
                            CorruptCause::InvariantViolation
                        ))?,
                        i64::try_from(doctor.checks().failed()).map_err(|_| corrupt_error(
                            id,
                            None,
                            CorruptCause::InvariantViolation
                        ))?,
                        i64::from(doctor.checks().exit_code()),
                        seq_nr,
                    ],
                )
                .map_err(|_| corrupt_error(id, None, CorruptCause::InvariantViolation))?;
            let rows = doctor.checks().fold_left(Vec::new(), |mut rows, check| {
                rows.push(check.clone());
                rows
            });
            for (position, check) in rows.iter().enumerate() {
                transaction
                    .execute(
                        "INSERT INTO read_doctor_check VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![
                            doctor_check(id, position),
                            id,
                            i64::try_from(position).map_err(|_| corrupt_error(
                                id,
                                None,
                                CorruptCause::InvariantViolation
                            ))?,
                            check.id().as_str(),
                            i64::from(check.is_passed()),
                            check.label(),
                            check.fix(),
                        ],
                    )
                    .map_err(|_| corrupt_error(id, None, CorruptCause::InvariantViolation))?;
            }
        }
        transaction
            .execute(
                "INSERT INTO workspace_doctor_projection_checkpoint VALUES (?1, ?2)
                 ON CONFLICT(projection) DO UPDATE SET last_seq = excluded.last_seq",
                params![PROJECTION, last_seq],
            )
            .map_err(|_| corrupt_error(PROJECTION, None, CorruptCause::InvariantViolation))?;
        transaction
            .commit()
            .map_err(|_| corrupt_error(PROJECTION, None, CorruptCause::InvariantViolation))
    }
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
