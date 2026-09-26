//! `DefinitionStageDao` の SQLite 実装 — `read_definition_stage` 表 1 つだけを読み書きする。

use rusqlite::{Connection, Transaction, params};

use super::column_value::{count_schema_objects, integer, read_content, scan_position};
use super::store_failure::SqliteResultExt;
use super::{DefinitionStageDao, DefinitionStageRow, GlobalSeqNr, JournalReadError, TableContent};

/// 表と索引の DDL (冪等)。
///
/// 列の並びは内容の照合 ([`DefinitionStageDao::find_content`]) の並びでもある。列を足す・落とす・型を
/// 変えるときは、読み面の版 (`read_model_schema::READ_SCHEMA_VERSION`) を上げる。
///
/// 索引は表と同じ口で作る — 表だけ在って索引が無い断面を作ると、自然キーの重複が静かに通るか、
/// クエリ側の引当が全走査になる。
const CREATE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS read_definition_stage (
  id                      TEXT    PRIMARY KEY,
  definition_id           TEXT    NOT NULL,
  stage_slug              TEXT    NOT NULL,
  position                INTEGER NOT NULL,
  number                  TEXT    NOT NULL,
  name                    TEXT    NOT NULL,
  phase                   TEXT    NOT NULL,
  execution               TEXT    NOT NULL,
  condition               TEXT    NOT NULL,
  lead_agent              TEXT    NOT NULL,
  support_agents          TEXT    NOT NULL,
  mode                    TEXT    NOT NULL,
  for_each                TEXT,
  workspace_requires      INTEGER NOT NULL,
  produces                TEXT    NOT NULL,
  optional_produces       TEXT    NOT NULL,
  produces_kinds          TEXT    NOT NULL,
  consumes                TEXT    NOT NULL,
  requires_stage          TEXT    NOT NULL,
  sensors                 TEXT    NOT NULL,
  scopes                  TEXT    NOT NULL,
  reviewer                TEXT,
  reviewer_max_iterations INTEGER,
  review_class            TEXT,
  summary_confirmation    TEXT,
  plugin                  TEXT,
  enabled                 INTEGER,
  gated                   INTEGER NOT NULL,
  inputs                  TEXT    NOT NULL,
  outputs                 TEXT    NOT NULL,
  rules_in_context        TEXT    NOT NULL,
  sensors_applicable      TEXT    NOT NULL,
  as_of                   INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS read_definition_stage_key
  ON read_definition_stage(definition_id, stage_slug);";

/// 表と索引が揃っているか (`sqlite_master` を引くだけ — 書込ロックを取らない)。数えた本数が
/// [`SCHEMA_OBJECTS`] に届いていれば揃っている。
const TABLE_EXISTS: &str = "SELECT COUNT(*) FROM sqlite_master WHERE (type = 'table' AND name = 'read_definition_stage') OR (type = 'index' AND name IN ('read_definition_stage_key'))";

/// 揃っているべき本数 (表 1 つと、その索引)。
const SCHEMA_OBJECTS: i64 = 2;

/// 表を落とす (索引も一緒に落ちる)。
const DROP_TABLE: &str = "DROP TABLE IF EXISTS read_definition_stage";

/// 全行を消す (全差し替えの前半)。
const DELETE_ALL: &str = "DELETE FROM read_definition_stage";

/// 行を足す。
const WRITE: &str = "INSERT INTO read_definition_stage
             (id, definition_id, stage_slug, position, number, name, phase, execution,
              condition, lead_agent, support_agents, mode, for_each, workspace_requires,
              produces, optional_produces, produces_kinds, consumes, requires_stage, sensors,
              scopes, reviewer, reviewer_max_iterations, review_class, summary_confirmation,
              plugin, enabled, gated, inputs, outputs, rules_in_context, sensors_applicable,
              as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                     ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30,
                     ?31, ?32, ?33)";

/// 全行の生の値 (主キーの昇順)。
const SELECT_CONTENT: &str = "SELECT * FROM read_definition_stage ORDER BY id";

/// `read_definition_stage` 表の DAO の実装。状態を持たない。
#[derive(Debug, Clone, Copy, Default)]
pub struct DefinitionStageDaoImpl;

impl DefinitionStageDao for DefinitionStageDaoImpl {
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
        rows: &[DefinitionStageRow],
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
                        row.stage_slug(),
                        integer(row.position(), row.id())?,
                        row.number(),
                        row.name(),
                        row.phase(),
                        row.execution(),
                        row.condition(),
                        row.lead_agent(),
                        row.support_agents(),
                        row.mode(),
                        row.for_each(),
                        row.workspace_requires(),
                        row.produces(),
                        row.optional_produces(),
                        row.produces_kinds(),
                        row.consumes(),
                        row.requires_stage(),
                        row.sensors(),
                        row.scopes(),
                        row.reviewer(),
                        row.reviewer_max_iterations(),
                        row.review_class(),
                        row.summary_confirmation(),
                        row.plugin(),
                        row.enabled(),
                        row.gated(),
                        row.inputs(),
                        row.outputs(),
                        row.rules_in_context(),
                        row.sensors_applicable(),
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
