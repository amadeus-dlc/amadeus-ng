//! HookHealth専用のジャーナル読取・投影。PlanApprovalの履歴を解釈しない。
use super::journal_reader_impl::corrupt_error;
use super::{CorruptCause, JournalReadError, ReadModelUpdater};
use chrono::{DateTime, Utc};
use core_command_domain::workspace::{
    HookAuditDropped, HookDropReason, HookDropSummary, HookFirstDropObserved, HookHealth,
    HookHealthEvent, HookHealthEventId, HookHealthId, HookHealthStarted, HookHealthTarget,
    HookHeartbeatObserved, HookName,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension as _, params};
use serde::Deserialize;
#[derive(Debug, Clone, Deserialize)]
struct EventWire {
    id: String,
    aggregate_id: String,
    kind: String,
    target: Option<String>,
    hook: Option<String>,
    reason: Option<String>,
}
/// QueryがIDで読むHookHealthの投影行。
#[derive(Debug, Clone, PartialEq, Eq)]
struct HookHealthReadRow {
    /// 集約ID。
    id: String,
    /// 観測領域。
    target: String,
    /// フック名。
    hook: String,
    /// 最終heartbeat。
    heartbeat: Option<String>,
    /// 履歴通番。
    seq_nr: usize,
    /// drop件数。
    drops: usize,
    /// 最新drop理由。
    latest_drop: Option<String>,
    /// この投影対象へ追記するdrop履歴。
    drop_lines: Vec<u8>,
}
/// HookHealthの共有DB投影器。
#[derive(Debug)]
pub struct HookHealthReadModelUpdater {
    connection: Connection,
    path: std::path::PathBuf,
}
impl ReadModelUpdater for HookHealthReadModelUpdater {
    type Error = JournalReadError;

    /// 自分のmanifestだけを全履歴から再投影する。
    ///
    /// 内部は同期 I/O だけである。非同期なのは共通契約の境界だけ。
    /// # Errors
    /// 履歴の復号・投影・公開に失敗した場合。
    async fn update_read_models(&mut self) -> Result<(), JournalReadError> {
        self.project()
    }
}

impl HookHealthReadModelUpdater {
    /// 既存共有DBへ接続する。DB自体は作らない。
    /// # Errors
    /// 共有DBへ接続できない場合。
    pub fn open(path: &std::path::Path) -> Result<Self, JournalReadError> {
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|_| corrupt_error("hook-health", None, CorruptCause::InvariantViolation))?;
        Ok(Self {
            connection,
            path: path.to_path_buf(),
        })
    }
    /// 自分のmanifestだけを全履歴から再投影する（[`ReadModelUpdater::update_read_models`] の本体）。
    fn project(&mut self) -> Result<(), JournalReadError> {
        let mut st=self.connection.prepare("SELECT aid,seq_nr,occurred_at,payload,manifest FROM journal WHERE manifest=?1 ORDER BY rowid").map_err(|_|corrupt_error("hook-health",None,CorruptCause::InvariantViolation))?;
        let rows = st
            .query_map(["hook-health-event/1"], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, Vec<u8>>(3)?,
                ))
            })
            .map_err(|_| corrupt_error("hook-health", None, CorruptCause::InvariantViolation))?;
        let mut groups: std::collections::BTreeMap<
            String,
            Vec<(HookHealthEvent, usize, DateTime<Utc>)>,
        > = std::collections::BTreeMap::new();
        for row in rows {
            let (aid, seq, at, bytes) = row.map_err(|_| {
                corrupt_error("hook-health", None, CorruptCause::UndecodablePayload)
            })?;
            let wire: EventWire = serde_json::from_slice(&bytes)
                .map_err(|_| corrupt_error(&aid, None, CorruptCause::UndecodablePayload))?;
            let event = Self::decode(wire)?;
            groups.entry(aid).or_default().push((
                event,
                usize::try_from(seq).map_err(|_| {
                    corrupt_error("hook-health", None, CorruptCause::InvariantViolation)
                })?,
                DateTime::from_timestamp_nanos(at),
            ));
        }
        let mut rows_out = Vec::new();
        for (aid, mut events) in groups {
            events.sort_by_key(|(_, seq, _)| *seq);
            let (first, first_seq, first_at) = events
                .first()
                .cloned()
                .ok_or_else(|| corrupt_error(&aid, None, CorruptCause::InvariantViolation))?;
            if first_seq != 1 || first.aggregate_id().as_str() != aid {
                return Err(corrupt_error(
                    &aid,
                    Some(first_seq),
                    CorruptCause::InvariantViolation,
                ));
            }
            let (target, hook, heartbeat, drop_summary) = match first {
                HookHealthEvent::Started(started) => (
                    started.target().clone(),
                    started.hook().clone(),
                    Some(first_at),
                    HookDropSummary::new(0, None),
                ),
                HookHealthEvent::FirstDropObserved(drop) => (
                    drop.target().clone(),
                    drop.hook().clone(),
                    None,
                    HookDropSummary::new(1, Some(drop.reason().clone())),
                ),
                _ => {
                    return Err(corrupt_error(
                        &aid,
                        Some(first_seq),
                        CorruptCause::InvariantViolation,
                    ));
                }
            };
            let id = HookHealthId::parse(&aid)
                .map_err(|_| corrupt_error(&aid, None, CorruptCause::UndecodablePayload))?;
            let health = HookHealth::new(
                id,
                target.clone(),
                hook.clone(),
                heartbeat,
                first_at,
                first_seq,
                0,
                drop_summary
                    .map_err(|_| corrupt_error(&aid, None, CorruptCause::InvariantViolation))?,
            )
            .map_err(|_| corrupt_error(&aid, None, CorruptCause::InvariantViolation))?;
            let drop_lines = events
                .iter()
                .filter_map(|(event, _, at)| match event {
                    HookHealthEvent::FirstDropObserved(value) => Some(format!(
                        "{}\t{}\n",
                        at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                        value.reason().as_str()
                    )),
                    HookHealthEvent::AuditDropped(value) => Some(format!(
                        "{}\t{}\n",
                        at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                        value.reason().as_str()
                    )),
                    _ => None,
                })
                .collect::<String>()
                .into_bytes();
            let health = HookHealth::replay(health, events.into_iter().skip(1));
            rows_out.push(HookHealthReadRow {
                id: aid,
                target: target.relative_directory(),
                hook: hook.as_str().to_string(),
                heartbeat: health
                    .heartbeat()
                    .map(|at| at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
                seq_nr: health.seq_nr(),
                drops: health.drops(),
                latest_drop: health.latest_drop().map(|v| v.as_str().to_string()),
                drop_lines,
            });
        }
        drop(st);
        let heartbeat_required = self.connection.query_row("SELECT \"notnull\" FROM pragma_table_info('read_hook_health') WHERE name='heartbeat'", [], |row| row.get::<_, i64>(0)).optional()
            .map_err(|_| corrupt_error("hook-health", None, CorruptCause::InvariantViolation))? == Some(1);
        let tx = self
            .connection
            .transaction()
            .map_err(|_| corrupt_error("hook-health", None, CorruptCause::InvariantViolation))?;
        if heartbeat_required {
            tx.execute_batch("DROP TABLE read_hook_health")
                .map_err(|_| {
                    corrupt_error("hook-health", None, CorruptCause::InvariantViolation)
                })?;
        }
        tx.execute_batch("CREATE TABLE IF NOT EXISTS read_hook_health(id TEXT PRIMARY KEY,target TEXT NOT NULL,hook TEXT NOT NULL,heartbeat TEXT,seq_nr INTEGER NOT NULL,drops INTEGER NOT NULL,latest_drop TEXT); CREATE TABLE IF NOT EXISTS hook_health_projection_checkpoint(projection TEXT PRIMARY KEY,last_seq INTEGER NOT NULL);").map_err(|_|corrupt_error("hook-health",None,CorruptCause::InvariantViolation))?;
        tx.execute("DELETE FROM read_hook_health", [])
            .map_err(|_| corrupt_error("hook-health", None, CorruptCause::InvariantViolation))?;
        for row in &rows_out {
            tx.execute(
                "INSERT INTO read_hook_health VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![
                    row.id,
                    row.target,
                    row.hook,
                    row.heartbeat,
                    i64::try_from(row.seq_nr).map_err(|_| corrupt_error(
                        "hook-health",
                        None,
                        CorruptCause::InvariantViolation
                    ))?,
                    i64::try_from(row.drops).map_err(|_| corrupt_error(
                        "hook-health",
                        None,
                        CorruptCause::InvariantViolation
                    ))?,
                    row.latest_drop
                ],
            )
            .map_err(|_| corrupt_error("hook-health", None, CorruptCause::InvariantViolation))?;
        }
        tx.execute("INSERT INTO hook_health_projection_checkpoint VALUES('hook-health',COALESCE((SELECT MAX(seq_nr) FROM read_hook_health),0)) ON CONFLICT(projection) DO UPDATE SET last_seq=excluded.last_seq",[]).map_err(|_|corrupt_error("hook-health",None,CorruptCause::InvariantViolation))?;
        tx.commit()
            .map_err(|_| corrupt_error("hook-health", None, CorruptCause::InvariantViolation))?;
        // heartbeatの公開ファイルは、このRMUが投影した行からだけ描く。
        let root = self
            .path
            .parent()
            .ok_or_else(|| corrupt_error("hook-health", None, CorruptCause::InvariantViolation))?;
        for row in &rows_out {
            let directory = root.join(&row.target).join(".aidlc-hooks-health");
            std::fs::create_dir_all(&directory)
                .map_err(|_| corrupt_error(&row.id, None, CorruptCause::InvariantViolation))?;
            if let Some(heartbeat) = &row.heartbeat {
                let heartbeat_path = directory.join(format!("{}.last", row.hook));
                std::fs::write(&heartbeat_path, heartbeat.as_bytes()).map_err(|error| {
                    JournalReadError::Io {
                        kind: error.kind(),
                        path: Some(heartbeat_path),
                    }
                })?;
            }
            if !row.drop_lines.is_empty() {
                let path = directory.join(format!("{}.drops", row.hook));
                let current = std::fs::read(&path).unwrap_or_default();
                let mut matched = 0;
                for offset in 0..row.drop_lines.len() {
                    if row
                        .drop_lines
                        .get(..=offset)
                        .is_some_and(|prefix| current.ends_with(prefix))
                    {
                        matched = offset + 1;
                    }
                }
                if matched < row.drop_lines.len() {
                    use std::io::Write as _;
                    let mut file = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&path)
                        .map_err(|_| {
                            corrupt_error(&row.id, None, CorruptCause::InvariantViolation)
                        })?;
                    file.write_all(row.drop_lines.get(matched..).unwrap_or_default())
                        .map_err(|_| {
                            corrupt_error(&row.id, None, CorruptCause::InvariantViolation)
                        })?;
                }
            }
        }
        Ok(())
    }
    fn decode(wire: EventWire) -> Result<HookHealthEvent, JournalReadError> {
        let id = HookHealthEventId::parse(&wire.id).map_err(|_| {
            corrupt_error(&wire.aggregate_id, None, CorruptCause::UndecodablePayload)
        })?;
        let aggregate = HookHealthId::parse(&wire.aggregate_id).map_err(|_| {
            corrupt_error(&wire.aggregate_id, None, CorruptCause::UndecodablePayload)
        })?;
        match wire.kind.as_str() {
            "first-drop" => Ok(HookHealthEvent::FirstDropObserved(
                HookFirstDropObserved::new(
                    id,
                    aggregate,
                    HookHealthTarget::parse(wire.target.as_deref().ok_or_else(|| {
                        corrupt_error("hook-health", None, CorruptCause::UndecodablePayload)
                    })?)
                    .map_err(|_| {
                        corrupt_error("hook-health", None, CorruptCause::UndecodablePayload)
                    })?,
                    HookName::parse(wire.hook.as_deref().ok_or_else(|| {
                        corrupt_error("hook-health", None, CorruptCause::UndecodablePayload)
                    })?)
                    .map_err(|_| {
                        corrupt_error("hook-health", None, CorruptCause::UndecodablePayload)
                    })?,
                    HookDropReason::parse(&wire.reason.ok_or_else(|| {
                        corrupt_error("hook-health", None, CorruptCause::UndecodablePayload)
                    })?)
                    .map_err(|_| {
                        corrupt_error("hook-health", None, CorruptCause::UndecodablePayload)
                    })?,
                ),
            )),
            "started" => Ok(HookHealthEvent::Started(HookHealthStarted::new(
                id,
                aggregate,
                HookHealthTarget::parse(wire.target.as_deref().ok_or_else(|| {
                    corrupt_error("hook-health", None, CorruptCause::UndecodablePayload)
                })?)
                .map_err(|_| {
                    corrupt_error("hook-health", None, CorruptCause::UndecodablePayload)
                })?,
                HookName::parse(wire.hook.as_deref().ok_or_else(|| {
                    corrupt_error("hook-health", None, CorruptCause::UndecodablePayload)
                })?)
                .map_err(|_| {
                    corrupt_error("hook-health", None, CorruptCause::UndecodablePayload)
                })?,
            ))),
            "heartbeat" => Ok(HookHealthEvent::HeartbeatObserved(
                HookHeartbeatObserved::new(id, aggregate),
            )),
            "dropped" => Ok(HookHealthEvent::AuditDropped(HookAuditDropped::new(
                id,
                aggregate,
                HookDropReason::new(wire.reason.ok_or_else(|| {
                    corrupt_error("hook-health", None, CorruptCause::UndecodablePayload)
                })?),
            ))),
            _ => Err(corrupt_error(
                "hook-health",
                None,
                CorruptCause::UndecodablePayload,
            )),
        }
    }
}
