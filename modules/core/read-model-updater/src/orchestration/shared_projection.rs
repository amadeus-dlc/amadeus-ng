//! space共通の構造化面の公開位置と内容同一性。
use super::journal_reader_impl::{corrupt_error, map_sqlite_error};
use super::{CorruptCause, GlobalSeqNr, JournalReadError, JournalReaderImpl, PublicationBatch};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use std::path::Path;

const SCHEMA: &str="CREATE TABLE IF NOT EXISTS amadeus_read_model_head (
 singleton INTEGER PRIMARY KEY CHECK(singleton=1), position INTEGER NOT NULL CHECK(position>=0),
 generation INTEGER NOT NULL CHECK(generation>0), revision TEXT NOT NULL, content_digest TEXT NOT NULL,
 verified INTEGER NOT NULL CHECK(verified IN (0,1)))";

pub(super) struct Head {
    position: i64,
    generation: i64,
    revision: String,
    digest: String,
    verified: bool,
}
impl Head {
    pub(super) const fn position(&self) -> i64 {
        self.position
    }
    pub(super) const fn generation(&self) -> i64 {
        self.generation
    }
    pub(super) fn is_current(&self) -> bool {
        self.revision == PublicationBatch::current_transform_revision()
    }
}

fn corrupt() -> JournalReadError {
    corrupt_error("-", None, CorruptCause::ProjectionSnapshotMismatch)
}

pub(super) fn initialize(connection: &Connection, path: &Path) -> Result<(), JournalReadError> {
    connection
        .execute_batch(SCHEMA)
        .map_err(|e| map_sqlite_error(&e, path))?;
    connection
        .execute(
            "INSERT OR IGNORE INTO amadeus_read_model_head VALUES (1,0,1,?1,'',0)",
            [PublicationBatch::current_transform_revision()],
        )
        .map_err(|e| map_sqlite_error(&e, path))?;
    Ok(())
}

pub(super) fn read(connection: &Connection, path: &Path) -> Result<Option<Head>, JournalReadError> {
    connection.query_row("SELECT position,generation,revision,content_digest,verified FROM amadeus_read_model_head WHERE singleton=1",[],|row|Ok(Head {position:row.get(0)?,generation:row.get(1)?,revision:row.get(2)?,digest:row.get(3)?,verified:row.get(4)?}))
        .optional().map_err(|e|map_sqlite_error(&e,path))
}

pub(super) fn invalidate(connection: &Connection, path: &Path) -> Result<(), JournalReadError> {
    connection
        .execute(
            "UPDATE amadeus_read_model_head SET verified=0 WHERE singleton=1",
            [],
        )
        .map_err(|e| map_sqlite_error(&e, path))?;
    Ok(())
}

pub(super) fn known_position(
    connection: &Connection,
    path: &Path,
) -> Result<i64, JournalReadError> {
    connection
        .query_row(
            "SELECT COALESCE(MAX(position),0) FROM (
        SELECT last_global_seq AS position FROM amadeus_projection_checkpoint
        UNION ALL SELECT as_of FROM read_execution UNION ALL SELECT as_of FROM read_intent
        UNION ALL SELECT as_of FROM read_definition)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| map_sqlite_error(&e, path))
}

pub(super) fn record(
    transaction: &Transaction<'_>,
    path: &Path,
    position: i64,
) -> Result<Head, JournalReadError> {
    let generation = read(transaction, path)?
        .map_or(0, |head| head.generation)
        .checked_add(1)
        .ok_or_else(corrupt)?;
    let digest =
        crate::read_tables::content_digest(transaction).map_err(|e| map_sqlite_error(&e, path))?;
    let revision = PublicationBatch::current_transform_revision();
    transaction.execute("INSERT INTO amadeus_read_model_head VALUES (1,?1,?2,?3,?4,1) ON CONFLICT(singleton) DO UPDATE SET position=excluded.position,generation=excluded.generation,revision=excluded.revision,content_digest=excluded.content_digest,verified=1",
        params![position,generation,revision,digest]).map_err(|e|map_sqlite_error(&e,path))?;
    Ok(Head {
        position,
        generation,
        revision,
        digest,
        verified: true,
    })
}

pub(super) fn verify(transaction: &Transaction<'_>, path: &Path) -> Result<Head, JournalReadError> {
    let head = read(transaction, path)?.ok_or_else(corrupt)?;
    // 旧インストールのキャッシュは、書込前に同じ位置のジャーナルから検証する。
    // open自体は復号やアンカー検査を強制せず、従来の読取エラー境界を保つ。
    if !head.verified {
        let position = head.position.max(known_position(transaction, path)?);
        let to = GlobalSeqNr::new(u64::try_from(position).map_err(|_| corrupt())?);
        let history =
            JournalReaderImpl::scan_range(transaction, path, GlobalSeqNr::ZERO, Some(to))?;
        if history.scanned_to().unwrap_or(GlobalSeqNr::ZERO) != to {
            return Err(corrupt());
        }
        let expected = crate::read_tables::ReadTables::project(&history).map_err(|_| corrupt())?;
        if !crate::read_tables::matches_rows(transaction, &expected)
            .map_err(|e| map_sqlite_error(&e, path))?
        {
            return Err(corrupt());
        }
        return record(transaction, path, position);
    }
    let actual =
        crate::read_tables::content_digest(transaction).map_err(|e| map_sqlite_error(&e, path))?;
    if head.position < 0
        || head.generation <= 0
        || head.revision != PublicationBatch::current_transform_revision()
        || head.digest != actual
    {
        return Err(corrupt());
    }
    Ok(head)
}
