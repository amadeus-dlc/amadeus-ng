//! `RunStageDao` の SQLite 実装 — `read_run_stage` 表 1 つだけを読み書きする。

use rusqlite::{Connection, Transaction, params};

use super::column_value::{count_schema_objects, read_content, scan_position};
use super::store_failure::SqliteResultExt;
use super::{GlobalSeqNr, JournalReadError, RunStageDao, RunStageRow, TableContent};

/// 表と索引の DDL (冪等)。
///
/// 列の並びは内容の照合 ([`RunStageDao::find_content`]) の並びでもある。列を足す・落とす・型を
/// 変えるときは、読み面の版 (`read_model_schema::READ_SCHEMA_VERSION`) を上げる。
///
/// 索引は表と同じ口で作る — 表だけ在って索引が無い断面を作ると、自然キーの重複が静かに通るか、
/// クエリ側の引当が全走査になる。
const CREATE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS read_run_stage (
  id                       TEXT    PRIMARY KEY,
  definition_id            TEXT    NOT NULL,
  scope                    TEXT    NOT NULL,
  stage_slug               TEXT    NOT NULL,
  phase                    TEXT    NOT NULL,
  steering_plan_id         TEXT    NOT NULL,
  lead_agent               TEXT    NOT NULL,
  support_agents           TEXT    NOT NULL,
  mode                     TEXT    NOT NULL,
  gate_default             INTEGER NOT NULL,
  in_scope                 INTEGER NOT NULL,
  inline_context_paths_rel TEXT    NOT NULL,
  stage_file_rel           TEXT    NOT NULL,
  memory_path_rel          TEXT    NOT NULL,
  consumes_rel             TEXT    NOT NULL,
  consumes_brownfield_rel  TEXT    NOT NULL,
  consumes_greenfield_rel  TEXT    NOT NULL,
  produces_rel             TEXT    NOT NULL,
  sensors_applicable       TEXT    NOT NULL,
  reviewer                 TEXT,
  reviewer_max_iterations  INTEGER,
  review_class             TEXT,
  protocol_modules         TEXT    NOT NULL,
  next_stage_name          TEXT,
  route_digest             TEXT    NOT NULL,
  directive_digest         TEXT    NOT NULL,
  as_of                    INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS read_run_stage_key
  ON read_run_stage(definition_id, scope, stage_slug);
CREATE INDEX IF NOT EXISTS read_run_stage_digests
  ON read_run_stage(route_digest, directive_digest);";

/// 表と索引が揃っているか (`sqlite_master` を引くだけ — 書込ロックを取らない)。数えた本数が
/// [`SCHEMA_OBJECTS`] に届いていれば揃っている。
const TABLE_EXISTS: &str = "SELECT COUNT(*) FROM sqlite_master WHERE (type = 'table' AND name = 'read_run_stage') OR (type = 'index' AND name IN ('read_run_stage_key', 'read_run_stage_digests'))";

/// 揃っているべき本数 (表 1 つと、その索引)。
const SCHEMA_OBJECTS: i64 = 3;

/// 表を落とす (索引も一緒に落ちる)。
const DROP_TABLE: &str = "DROP TABLE IF EXISTS read_run_stage";

/// 全行を消す (全差し替えの前半)。
const DELETE_ALL: &str = "DELETE FROM read_run_stage";

/// 行を足す。
const WRITE: &str = "INSERT INTO read_run_stage
             (id, definition_id, scope, stage_slug, phase, steering_plan_id, lead_agent,
              support_agents, mode, gate_default, in_scope, inline_context_paths_rel,
              stage_file_rel, memory_path_rel, consumes_rel, consumes_brownfield_rel,
              consumes_greenfield_rel, produces_rel, sensors_applicable,
              reviewer, reviewer_max_iterations, review_class, protocol_modules, next_stage_name,
              route_digest, directive_digest, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                     ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27)";

/// 全行の生の値 (主キーの昇順)。
const SELECT_CONTENT: &str = "SELECT * FROM read_run_stage ORDER BY id";

/// `read_run_stage` 表の DAO の実装。状態を持たない。
#[derive(Debug, Clone, Copy, Default)]
pub struct RunStageDaoImpl;

impl RunStageDao for RunStageDaoImpl {
    fn create_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError> {
        transaction
            .execute_batch(CREATE_TABLE)
            .at_connection(transaction)
    }

    fn table_exists(&self, connection: &Connection) -> Result<bool, JournalReadError> {
        Ok(count_schema_objects(connection, TABLE_EXISTS)? == SCHEMA_OBJECTS)
    }

    fn drop_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError> {
        transaction
            .execute_batch(DROP_TABLE)
            .at_connection(transaction)
    }

    fn delete_all(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError> {
        transaction
            .execute(DELETE_ALL, [])
            .at_connection(transaction)?;
        Ok(())
    }

    fn insert(
        &self,
        transaction: &mut Transaction<'_>,
        rows: &[RunStageRow],
        as_of: GlobalSeqNr,
    ) -> Result<(), JournalReadError> {
        let as_of = scan_position(as_of)?;
        for row in rows {
            transaction
                .execute(
                    WRITE,
                    params![
                        row.id(),
                        row.definition_id(),
                        row.scope(),
                        row.stage_slug(),
                        row.phase(),
                        row.steering_plan_id(),
                        row.lead_agent(),
                        row.support_agents(),
                        row.mode(),
                        row.gate_default(),
                        row.in_scope(),
                        row.inline_context_paths_rel(),
                        row.stage_file_rel(),
                        row.memory_path_rel(),
                        row.consumes_rel(),
                        row.consumes_brownfield_rel(),
                        row.consumes_greenfield_rel(),
                        row.produces_rel(),
                        row.sensors_applicable(),
                        row.reviewer(),
                        row.reviewer_max_iterations(),
                        row.review_class(),
                        row.protocol_modules(),
                        row.next_stage_name(),
                        row.route_digest(),
                        row.directive_digest(),
                        as_of
                    ],
                )
                .at_connection(transaction)?;
        }
        Ok(())
    }

    fn find_content(&self, connection: &Connection) -> Result<TableContent, JournalReadError> {
        read_content(connection, SELECT_CONTENT)
    }
}
