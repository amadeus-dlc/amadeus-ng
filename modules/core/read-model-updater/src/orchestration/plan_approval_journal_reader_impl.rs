//! ワークスペース全体の承認ストアを読むRMUのSQLite接続。
use super::dto::PlanApprovalEventDto;
use super::store_failure::SqliteResultExt;
use super::{CorruptCause, JournalReadError, PlanApprovalJournalEntry, PlanApprovalJournalReader};
use crate::read_tables::PlanApprovalTables;
use core_command_domain::workspace::StorePath;
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
const CREATE: &str = "CREATE TABLE IF NOT EXISTS read_plan_operation(operation_id TEXT PRIMARY KEY, status TEXT NOT NULL, space TEXT, execution_id TEXT, as_of INTEGER NOT NULL, kind TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS read_plan_answer(operation_id TEXT PRIMARY KEY,status TEXT NOT NULL,emitted TEXT,stage TEXT NOT NULL,error TEXT,as_of INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS read_plan_generation(operation_id TEXT PRIMARY KEY,status TEXT NOT NULL,unit TEXT,error TEXT,as_of INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS amadeus_plan_projection_checkpoint(projection TEXT PRIMARY KEY, last_seq INTEGER NOT NULL, anchor_event_id TEXT);";
/// 書き手のRepository/DTOには依存しない別接続。
#[derive(Debug)]
pub struct PlanApprovalJournalReaderImpl {
    connection: Connection,
    path: StorePath,
}
impl PlanApprovalJournalReaderImpl {
    /// 既存のストアへ接続する。空DBは作成しない。
    /// # Errors
    /// 接続・投影表の用意に失敗した場合。
    pub fn open(path: &StorePath) -> Result<Self, JournalReadError> {
        let connection = Connection::open_with_flags(
            path.as_path(),
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .at_store(path.as_path())?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .at_store(path.as_path())?;
        connection.execute_batch(CREATE).at_store(path.as_path())?;
        let has_kind: i64 = connection
            .query_row(
                "SELECT count(*) FROM pragma_table_info('read_plan_operation') WHERE name='kind'",
                [],
                |row| row.get(0),
            )
            .at_store(path.as_path())?;
        if has_kind == 0 {
            connection.execute("ALTER TABLE read_plan_operation ADD COLUMN kind TEXT NOT NULL DEFAULT 'publication'", []).at_store(path.as_path())?;
        }
        Ok(Self {
            connection,
            path: path.clone(),
        })
    }
    fn corrupt(cause: CorruptCause) -> JournalReadError {
        JournalReadError::Corrupt {
            aggregate_id: "workspace".to_string(),
            seq_nr: None,
            cause,
        }
    }
}
impl PlanApprovalJournalReader for PlanApprovalJournalReaderImpl {
    fn all_events(&self) -> Result<Vec<PlanApprovalJournalEntry>, JournalReadError> {
        // 共有DBにはHookHealth等の別ストリームも入る。承認集約のaidだけを読む。
        // rowidは共有通番なので、承認集約固有のseq_nrと一致するとは限らない。
        let mut statement = self.connection.prepare("SELECT rowid, aid, seq_nr, occurred_at, payload, manifest FROM journal WHERE aid='workspace' ORDER BY rowid").at_store(self.path.as_path())?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .at_store(self.path.as_path())?;
        let mut entries = Vec::new();
        let mut expected_sequence = 0_i64;
        for row in rows {
            let (_rowid, aid, sequence, at, bytes, manifest) = row.at_store(self.path.as_path())?;
            if aid != "workspace" || sequence < 1 || manifest != "plan-approval-event/1" {
                return Err(Self::corrupt(CorruptCause::InvariantViolation));
            }
            expected_sequence = expected_sequence
                .checked_add(1)
                .ok_or_else(|| Self::corrupt(CorruptCause::InvariantViolation))?;
            if sequence != expected_sequence {
                return Err(Self::corrupt(CorruptCause::InvariantViolation));
            }
            let sequence = usize::try_from(sequence)
                .map_err(|_| Self::corrupt(CorruptCause::InvariantViolation))?;
            let payload: PlanApprovalEventDto = serde_json::from_slice(&bytes)
                .map_err(|_| Self::corrupt(CorruptCause::UndecodablePayload))?;
            let event = payload
                .to_domain()
                .map_err(|_| Self::corrupt(CorruptCause::UndecodablePayload))?;
            entries.push(PlanApprovalJournalEntry::new(
                sequence,
                chrono::DateTime::from_timestamp_nanos(at),
                event,
            ));
        }
        Ok(entries)
    }
    fn replace(&mut self, tables: &PlanApprovalTables) -> Result<(), JournalReadError> {
        let sequence = i64::try_from(tables.sequence())
            .map_err(|_| Self::corrupt(CorruptCause::InvariantViolation))?;
        let path = self.path.as_path();
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .at_store(path)?;
        let checkpoint: Option<(i64, Option<String>)> = transaction.query_row("SELECT last_seq, anchor_event_id FROM amadeus_plan_projection_checkpoint WHERE projection='plan-approval'", [], |row| Ok((row.get(0)?, row.get(1)?))).optional().at_store(path)?;
        if checkpoint
            .as_ref()
            .is_some_and(|(current, _)| *current > sequence)
        {
            return Err(Self::corrupt(CorruptCause::CheckpointAnchorMismatch));
        }
        if let Some((sequence, Some(anchor))) = &checkpoint {
            let bytes: Option<Vec<u8>> = transaction
                .query_row(
                    "SELECT payload FROM journal WHERE aid='workspace' AND seq_nr=?1",
                    [sequence],
                    |row| row.get(0),
                )
                .optional()
                .at_store(path)?;
            let valid = bytes
                .as_deref()
                .and_then(|bytes| serde_json::from_slice::<PlanApprovalEventDto>(bytes).ok())
                .and_then(|dto| dto.to_domain().ok())
                .is_some_and(|event| event.id().as_str() == anchor);
            if !valid {
                return Err(Self::corrupt(CorruptCause::CheckpointAnchorMismatch));
            }
        }
        let root = path
            .parent()
            .ok_or_else(|| Self::corrupt(CorruptCause::InvariantViolation))?;
        super::plan_approval_files::publish(root, tables.files(), tables.directory_present())?;
        transaction
            .execute("DELETE FROM read_plan_operation", [])
            .at_store(path)?;
        for row in tables.rows() {
            transaction.execute("INSERT INTO read_plan_operation(operation_id,status,space,execution_id,as_of,kind) VALUES (?1,?2,?3,?4,?5,?6)", params![row.id(),row.status(),row.space(),row.execution_id(),sequence,row.kind()]).at_store(path)?;
        }
        transaction
            .execute("DELETE FROM read_plan_answer", [])
            .at_store(path)?;
        for answer in tables.answers() {
            transaction.execute("INSERT INTO read_plan_answer(operation_id,status,emitted,stage,error,as_of) VALUES(?1,?2,?3,?4,?5,?6)",params![answer.id(),answer.status(),answer.emitted(),answer.stage(),answer.error(),sequence]).at_store(path)?;
        }
        transaction
            .execute("DELETE FROM read_plan_generation", [])
            .at_store(path)?;
        for generation in tables.generations() {
            transaction.execute("INSERT INTO read_plan_generation(operation_id,status,unit,error,as_of) VALUES(?1,?2,?3,?4,?5)",params![generation.id(),generation.status(),generation.unit(),generation.error(),sequence]).at_store(path)?;
        }
        transaction.execute("INSERT INTO amadeus_plan_projection_checkpoint(projection,last_seq,anchor_event_id) VALUES ('plan-approval',?1,?2) ON CONFLICT(projection) DO UPDATE SET last_seq=excluded.last_seq, anchor_event_id=excluded.anchor_event_id", params![sequence, tables.anchor_event_id()]).at_store(path)?;
        transaction.commit().at_store(path)
    }
}
