//! D5 の観測 — イベントストアを読取専用で開き、表と投影の位置を読む。
//!
//! 作成フラグを付けずに開くので、存在しないストアに空 DB を作ることは起きない。
//! どの文も 1 表しか読まない (`coding-rules/cqrs-boundaries.md` 規則 6)。

use std::path::Path;

use core_query_use_case::orchestration::{
    ObservationFailure, ProjectionObservationView, StoreObservationView, StoreSchemaView,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension as _};

/// この build の実行イベントの型判別子 (書く側 `intent-execution-event/1` の写し)。
const EXECUTION_MANIFEST: &str = "intent-execution-event/1";

/// 読取専用で開いた接続と表の一覧。
pub(super) struct OpenedStore {
    connection: Connection,
    tables: Vec<String>,
}

impl OpenedStore {
    fn has_table(&self, name: &str) -> bool {
        self.tables.iter().any(|table| table == name)
    }
}

/// ストアの 3 態を観測する。開けたときは接続を返して投影の観測に使う。
pub(super) fn observe(path: &Path) -> (StoreObservationView, Option<OpenedStore>) {
    if !path.exists() {
        return (StoreObservationView::Absent, None);
    }
    if path.is_dir() {
        return (
            StoreObservationView::Unreadable(ObservationFailure::new("is a directory".to_string())),
            None,
        );
    }
    let connection = match Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        Ok(connection) => connection,
        Err(error) => {
            return (
                StoreObservationView::Unreadable(ObservationFailure::new(error.to_string())),
                None,
            );
        }
    };
    let tables = connection
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<String>, _>>()
        });
    let tables = match tables {
        Ok(tables) => tables,
        Err(error) => {
            return (
                StoreObservationView::Unreadable(ObservationFailure::new(error.to_string())),
                None,
            );
        }
    };
    let version: Result<i64, _> = connection.query_row("PRAGMA user_version", [], |row| row.get(0));
    match version {
        Ok(version) => (
            StoreObservationView::Opened(StoreSchemaView::new(tables.clone(), version)),
            Some(OpenedStore { connection, tables }),
        ),
        Err(error) => (
            StoreObservationView::Unreadable(ObservationFailure::new(error.to_string())),
            None,
        ),
    }
}

/// 選択中の実行に対する投影の位置と整合の材料。
pub(super) fn projection(
    store: &OpenedStore,
    execution_id: &str,
    audit_shard_count: usize,
) -> Result<ProjectionObservationView, ObservationFailure> {
    let failure = |error: rusqlite::Error| ObservationFailure::new(error.to_string());
    let projection_name = format!("orchestration-{execution_id}");
    let execution_intent_id = if store.has_table("read_execution") {
        store
            .connection
            .query_row(
                "SELECT intent_id FROM read_execution WHERE id=?1",
                [execution_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(failure)?
    } else {
        None
    };
    let checkpoint = if store.has_table("amadeus_projection_checkpoint") {
        store
            .connection
            .query_row(
                "SELECT last_global_seq FROM amadeus_projection_checkpoint WHERE projection=?1",
                [&projection_name],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(failure)?
    } else {
        None
    };
    let latest_execution_event = if store.has_table("journal") {
        store
            .connection
            .query_row(
                "SELECT MAX(rowid) FROM journal WHERE aid=?1 AND manifest=?2",
                [execution_id, EXECUTION_MANIFEST],
                |row| row.get::<_, Option<i64>>(0),
            )
            .map_err(failure)?
    } else {
        None
    };
    let pending_publications = if store.has_table("amadeus_publication") {
        store
            .connection
            .query_row(
                "SELECT COUNT(*) FROM amadeus_publication WHERE projection=?1 AND committed=0",
                [&projection_name],
                |row| row.get::<_, i64>(0),
            )
            .map_err(failure)?
    } else {
        0
    };
    Ok(ProjectionObservationView::new(
        execution_intent_id,
        checkpoint,
        latest_execution_event,
        u64::try_from(pending_publications).unwrap_or(0),
        audit_shard_count,
    ))
}
