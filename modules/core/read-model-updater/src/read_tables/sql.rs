//! `read_*` 表の DDL と**全差し替え** (公開型ゼロの内部モジュール)。
//!
//! 表はイベントストアと同じ DB ファイルに置く (裁定 §10 — 非正規データを引くには SQLite の
//! ほうが自由度が高い)。名前は `read_` 接頭で本家の `journal` / `snapshot` と衝突しない。
//! DDL は `CREATE TABLE IF NOT EXISTS` — チェックポイント表と同じ流儀で冪等である。
//!
//! 書込は差分ではなく**全差し替え** (`DELETE` してから `INSERT`)。投影が全履歴からの
//! 再計算だからであり、行の差し替えとチェックポイントの前進は呼出側が 1 トランザクションに
//! 閉じる (裁定 §3)。
//!
//! `as_of` 列はどの表にも在り、値は [`ReadTables::as_of`] 1 つである。行ごとに違う値を
//! 持たないので、行型には持たせずここで書く — 「いつ時点の行か」はスナップショット全体の
//! 性質である。
//!
//! [`ReadTables::as_of`]: super::ReadTables::as_of

use rusqlite::{Connection, Transaction, params};

use super::ReadTables;

/// 数を SQLite の `INTEGER` (i64) へ写す。
///
/// rusqlite の `usize` / `u64` 向け `ToSql` は `fallible_uint` フィーチャの裏に在り、この
/// ワークスペースでは有効にしていない。収まらない値を静かに丸めるのは行に嘘を書くことなので、
/// 変換の失敗をそのまま SQLite の失敗として返す (呼出側が I/O の失敗へ写す)。
fn integer(value: usize) -> Result<i64, rusqlite::Error> {
    i64::try_from(value).map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

/// 省略可能な数を `INTEGER | NULL` へ写す。
fn optional_integer(value: Option<usize>) -> Result<Option<i64>, rusqlite::Error> {
    value.map(integer).transpose()
}

/// 13 表の DDL (この順に作る)。
const CREATE_TABLES: &str = "
CREATE TABLE IF NOT EXISTS read_definition (
  definition_id TEXT PRIMARY KEY,
  revision      TEXT    NOT NULL,
  stage_count   INTEGER NOT NULL,
  scope_count   INTEGER NOT NULL,
  as_of         INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS read_definition_stage (
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
  as_of                   INTEGER NOT NULL,
  PRIMARY KEY (definition_id, stage_slug)
);
CREATE TABLE IF NOT EXISTS read_definition_scope (
  definition_id        TEXT    NOT NULL,
  scope                TEXT    NOT NULL,
  depth                TEXT,
  keywords             TEXT    NOT NULL,
  skeleton             TEXT,
  review_cap           TEXT,
  freeform_default     INTEGER NOT NULL,
  has_grid_column      INTEGER NOT NULL,
  cost_total           INTEGER,
  cost_execute         INTEGER,
  cost_gates           INTEGER,
  cost_per_unit_stages INTEGER,
  as_of                INTEGER NOT NULL,
  PRIMARY KEY (definition_id, scope)
);
CREATE TABLE IF NOT EXISTS read_definition_scope_keyword (
  definition_id TEXT    NOT NULL,
  keyword       TEXT    NOT NULL,
  scope         TEXT    NOT NULL,
  as_of         INTEGER NOT NULL,
  PRIMARY KEY (definition_id, keyword)
);
CREATE TABLE IF NOT EXISTS read_definition_scope_stage (
  definition_id  TEXT    NOT NULL,
  scope          TEXT    NOT NULL,
  stage_slug     TEXT    NOT NULL,
  action         TEXT,
  in_scope_order INTEGER,
  as_of          INTEGER NOT NULL,
  PRIMARY KEY (definition_id, scope, stage_slug)
);
CREATE TABLE IF NOT EXISTS read_definition_scope_phase_entry (
  definition_id    TEXT    NOT NULL,
  scope            TEXT    NOT NULL,
  phase            TEXT    NOT NULL,
  first_stage_slug TEXT    NOT NULL,
  as_of            INTEGER NOT NULL,
  PRIMARY KEY (definition_id, scope, phase)
);
CREATE TABLE IF NOT EXISTS read_intent (
  intent_id           TEXT PRIMARY KEY,
  definition_id       TEXT    NOT NULL,
  definition_revision TEXT    NOT NULL,
  scope               TEXT    NOT NULL,
  request             TEXT    NOT NULL,
  depth               TEXT,
  test_strategy       TEXT,
  review              TEXT,
  created_at          TEXT    NOT NULL,
  project_type        TEXT    NOT NULL,
  project_kind        TEXT    NOT NULL,
  languages           TEXT    NOT NULL,
  frameworks          TEXT    NOT NULL,
  build_system        TEXT    NOT NULL,
  as_of               INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS read_intent_stage (
  intent_id   TEXT    NOT NULL,
  stage_index INTEGER NOT NULL,
  slug        TEXT    NOT NULL,
  phase       TEXT    NOT NULL,
  plan_action TEXT    NOT NULL,
  conditional INTEGER NOT NULL,
  number      TEXT    NOT NULL,
  name        TEXT    NOT NULL,
  lead_agent  TEXT    NOT NULL,
  gated       INTEGER NOT NULL,
  as_of       INTEGER NOT NULL,
  PRIMARY KEY (intent_id, stage_index)
);
CREATE TABLE IF NOT EXISTS read_execution (
  execution_id     TEXT PRIMARY KEY,
  intent_id        TEXT    NOT NULL,
  status           TEXT    NOT NULL,
  cursor_index     INTEGER,
  cursor_slug      TEXT,
  parked_at_index  INTEGER,
  parked_at_slug   TEXT,
  parked_active    INTEGER NOT NULL,
  accepts_commands INTEGER NOT NULL,
  autonomy         TEXT    NOT NULL,
  seq_nr           INTEGER NOT NULL,
  last_updated_at  TEXT    NOT NULL,
  state_binding    TEXT    NOT NULL,
  as_of            INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS read_execution_stage (
  execution_id   TEXT    NOT NULL,
  stage_index    INTEGER NOT NULL,
  slug           TEXT    NOT NULL,
  phase          TEXT    NOT NULL,
  checkbox       TEXT,
  effective_plan TEXT,
  approved       INTEGER,
  revision_count INTEGER,
  gated          INTEGER,
  as_of          INTEGER NOT NULL,
  PRIMARY KEY (execution_id, stage_index)
);
CREATE TABLE IF NOT EXISTS read_next_answer (
  execution_id  TEXT    NOT NULL,
  request_kind  TEXT    NOT NULL,
  decision_kind TEXT    NOT NULL,
  stage_index   INTEGER,
  stage_slug    TEXT,
  gated         INTEGER,
  checkbox      TEXT,
  as_of         INTEGER NOT NULL,
  PRIMARY KEY (execution_id, request_kind)
);
CREATE TABLE IF NOT EXISTS read_next_jump (
  execution_id TEXT    NOT NULL,
  target_index INTEGER NOT NULL,
  target_slug  TEXT    NOT NULL,
  outcome      TEXT    NOT NULL,
  refusal      TEXT,
  as_of        INTEGER NOT NULL,
  PRIMARY KEY (execution_id, target_index)
);
CREATE TABLE IF NOT EXISTS read_next_jump_phase (
  execution_id TEXT    NOT NULL,
  phase        TEXT    NOT NULL,
  target_index INTEGER NOT NULL,
  target_slug  TEXT,
  as_of        INTEGER NOT NULL,
  PRIMARY KEY (execution_id, phase)
);
";

/// 全差し替えの `DELETE` (DDL と同じ 13 表・同じ順)。
const DELETE_TABLES: &str = "
DELETE FROM read_definition;
DELETE FROM read_definition_stage;
DELETE FROM read_definition_scope;
DELETE FROM read_definition_scope_keyword;
DELETE FROM read_definition_scope_stage;
DELETE FROM read_definition_scope_phase_entry;
DELETE FROM read_intent;
DELETE FROM read_intent_stage;
DELETE FROM read_execution;
DELETE FROM read_execution_stage;
DELETE FROM read_next_answer;
DELETE FROM read_next_jump;
DELETE FROM read_next_jump_phase;
";

/// 13 表を (無ければ) 作る。冪等なので何度呼んでもよい。
///
/// # Errors
///
/// SQLite の失敗をそのまま返す (呼出側が I/O の失敗へ写す)。
pub(crate) fn ensure_tables(connection: &Connection) -> Result<(), rusqlite::Error> {
    connection.execute_batch(CREATE_TABLES)
}

/// 13 表の行を全部差し替える。
///
/// トランザクションは**呼出側が持つ** — チェックポイントの前進と同じ 1 つの Tx に閉じる
/// ためである (裁定 §3)。
///
/// # Errors
///
/// SQLite の失敗をそのまま返す (呼出側が I/O の失敗へ写す)。
#[allow(
    clippy::too_many_lines,
    reason = "13 表の INSERT を 1 か所に並べる — 表と列の対応が一覧で読めることを優先する"
)]
pub(crate) fn replace_all(
    transaction: &Transaction<'_>,
    tables: &ReadTables,
) -> Result<(), rusqlite::Error> {
    transaction.execute_batch(DELETE_TABLES)?;
    // 「いつ時点の行か」はスナップショット全体の性質なので、全表に同じ値を書く。
    let as_of = integer(
        usize::try_from(
            tables
                .as_of()
                .unwrap_or(crate::orchestration::GlobalSeqNr::ZERO)
                .to_u64(),
        )
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?,
    )?;

    for row in tables.definitions() {
        transaction.execute(
            "INSERT INTO read_definition
             (definition_id, revision, stage_count, scope_count, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                row.definition_id(),
                row.revision(),
                integer(row.stage_count())?,
                integer(row.scope_count())?,
                as_of
            ],
        )?;
    }

    for row in tables.definition_stages() {
        transaction.execute(
            "INSERT INTO read_definition_stage
             (definition_id, stage_slug, position, number, name, phase, execution, condition,
              lead_agent, support_agents, mode, for_each, workspace_requires, produces,
              optional_produces, produces_kinds, consumes, requires_stage, sensors, scopes,
              reviewer, reviewer_max_iterations, review_class, summary_confirmation, plugin,
              enabled, gated, inputs, outputs, rules_in_context, sensors_applicable, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                     ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30,
                     ?31, ?32)",
            params![
                row.definition_id(),
                row.stage_slug(),
                integer(row.position())?,
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
        )?;
    }

    for row in tables.definition_scopes() {
        transaction.execute(
            "INSERT INTO read_definition_scope
             (definition_id, scope, depth, keywords, skeleton, review_cap, freeform_default,
              has_grid_column, cost_total, cost_execute, cost_gates, cost_per_unit_stages, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                row.definition_id(),
                row.scope(),
                row.depth(),
                row.keywords(),
                row.skeleton(),
                row.review_cap(),
                row.freeform_default(),
                row.has_grid_column(),
                optional_integer(row.cost_total())?,
                optional_integer(row.cost_execute())?,
                optional_integer(row.cost_gates())?,
                optional_integer(row.cost_per_unit_stages())?,
                as_of
            ],
        )?;
    }

    for row in tables.definition_scope_keywords() {
        transaction.execute(
            "INSERT INTO read_definition_scope_keyword
             (definition_id, keyword, scope, as_of)
             VALUES (?1, ?2, ?3, ?4)",
            params![row.definition_id(), row.keyword(), row.scope(), as_of],
        )?;
    }

    for row in tables.definition_scope_stages() {
        transaction.execute(
            "INSERT INTO read_definition_scope_stage
             (definition_id, scope, stage_slug, action, in_scope_order, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                row.definition_id(),
                row.scope(),
                row.stage_slug(),
                row.action(),
                optional_integer(row.in_scope_order())?,
                as_of
            ],
        )?;
    }

    for row in tables.definition_scope_phase_entries() {
        transaction.execute(
            "INSERT INTO read_definition_scope_phase_entry
             (definition_id, scope, phase, first_stage_slug, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                row.definition_id(),
                row.scope(),
                row.phase(),
                row.first_stage_slug(),
                as_of
            ],
        )?;
    }

    for row in tables.intents() {
        transaction.execute(
            "INSERT INTO read_intent
             (intent_id, definition_id, definition_revision, scope, request, depth,
              test_strategy, review, created_at, project_type, project_kind, languages,
              frameworks, build_system, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                row.intent_id(),
                row.definition_id(),
                row.definition_revision(),
                row.scope(),
                row.request(),
                row.depth(),
                row.test_strategy(),
                row.review(),
                row.created_at(),
                row.project_type(),
                row.project_kind(),
                row.languages(),
                row.frameworks(),
                row.build_system(),
                as_of
            ],
        )?;
    }

    for row in tables.intent_stages() {
        transaction.execute(
            "INSERT INTO read_intent_stage
             (intent_id, stage_index, slug, phase, plan_action, conditional, number, name,
              lead_agent, gated, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                row.intent_id(),
                integer(row.stage_index())?,
                row.slug(),
                row.phase(),
                row.plan_action(),
                row.conditional(),
                row.number(),
                row.name(),
                row.lead_agent(),
                row.gated(),
                as_of
            ],
        )?;
    }

    for row in tables.executions() {
        transaction.execute(
            "INSERT INTO read_execution
             (execution_id, intent_id, status, cursor_index, cursor_slug, parked_at_index,
              parked_at_slug, parked_active, accepts_commands, autonomy, seq_nr,
              last_updated_at, state_binding, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                row.execution_id(),
                row.intent_id(),
                row.status(),
                optional_integer(row.cursor_index())?,
                row.cursor_slug(),
                optional_integer(row.parked_at_index())?,
                row.parked_at_slug(),
                row.parked_active(),
                row.accepts_commands(),
                row.autonomy(),
                integer(row.seq_nr())?,
                row.last_updated_at(),
                row.state_binding(),
                as_of
            ],
        )?;
    }

    for row in tables.execution_stages() {
        transaction.execute(
            "INSERT INTO read_execution_stage
             (execution_id, stage_index, slug, phase, checkbox, effective_plan, approved,
              revision_count, gated, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                row.execution_id(),
                integer(row.stage_index())?,
                row.slug(),
                row.phase(),
                row.checkbox(),
                row.effective_plan(),
                row.approved(),
                row.revision_count(),
                row.gated(),
                as_of
            ],
        )?;
    }

    for row in tables.next_answers() {
        transaction.execute(
            "INSERT INTO read_next_answer
             (execution_id, request_kind, decision_kind, stage_index, stage_slug, gated,
              checkbox, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                row.execution_id(),
                row.request_kind(),
                row.decision_kind(),
                optional_integer(row.stage_index())?,
                row.stage_slug(),
                row.gated(),
                row.checkbox(),
                as_of
            ],
        )?;
    }

    for row in tables.next_jumps() {
        transaction.execute(
            "INSERT INTO read_next_jump
             (execution_id, target_index, target_slug, outcome, refusal, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                row.execution_id(),
                integer(row.target_index())?,
                row.target_slug(),
                row.outcome(),
                row.refusal(),
                as_of
            ],
        )?;
    }

    for row in tables.next_jump_phases() {
        transaction.execute(
            "INSERT INTO read_next_jump_phase
             (execution_id, phase, target_index, target_slug, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                row.execution_id(),
                row.phase(),
                integer(row.target_index())?,
                row.target_slug(),
                as_of
            ],
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    // 想定外ケースの即時失敗はテストの検証手段である (house style)。
    #![allow(
        clippy::panic,
        reason = "表が作られていないことを名前つきで即時失敗させる (house style)"
    )]

    use super::*;
    use crate::orchestration::JournalBatch;

    /// 13 表の名前 (DDL と `DELETE` が同じ集合を指していることを固定する)。
    const TABLES: [&str; 13] = [
        "read_definition",
        "read_definition_stage",
        "read_definition_scope",
        "read_definition_scope_keyword",
        "read_definition_scope_stage",
        "read_definition_scope_phase_entry",
        "read_intent",
        "read_intent_stage",
        "read_execution",
        "read_execution_stage",
        "read_next_answer",
        "read_next_jump",
        "read_next_jump_phase",
    ];

    #[test]
    fn the_ddl_creates_all_thirteen_tables_and_is_idempotent() {
        let connection = Connection::open_in_memory().expect("メモリ DB は開ける");
        ensure_tables(&connection).expect("初回の DDL");
        ensure_tables(&connection).expect("2 回目も通る (IF NOT EXISTS)");
        for name in TABLES {
            let found: String = connection
                .query_row(
                    "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    params![name],
                    |row| row.get(0),
                )
                .unwrap_or_else(|_| panic!("{name} が作られている"));
            assert_eq!(found, name);
        }
    }

    #[test]
    fn every_table_carries_the_as_of_column() {
        let connection = Connection::open_in_memory().expect("メモリ DB は開ける");
        ensure_tables(&connection).expect("DDL");
        for name in TABLES {
            let count: i64 = connection
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM pragma_table_info('{name}') WHERE name = 'as_of'"
                    ),
                    [],
                    |row| row.get(0),
                )
                .expect("pragma は引ける");
            assert_eq!(count, 1, "{name} に as_of 列がある");
        }
    }

    #[test]
    fn a_scan_position_that_does_not_fit_the_column_fails_instead_of_being_rounded() {
        // `as_of` は全表に同じ値で書かれる。`INTEGER` (i64) に収まらない走査位置を静かに
        // 丸めると、行が「いつ時点か」を偽る。収まらないなら 1 行も書かずに失敗する。
        let connection = Connection::open_in_memory().expect("メモリ DB は開ける");
        ensure_tables(&connection).expect("DDL");
        let mut connection = connection;
        let transaction = connection.transaction().expect("Tx は張れる");

        let tables = ReadTables::project(&JournalBatch::new(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Some(crate::orchestration::GlobalSeqNr::new(u64::MAX)),
        ))
        .expect("空の履歴でも投影はできる");

        let error = replace_all(&transaction, &tables).expect_err("i64 に収まらない走査位置");
        assert!(
            matches!(error, rusqlite::Error::ToSqlConversionFailure(_)),
            "変換の失敗がそのまま SQLite の失敗として上がる (実際: {error:?})"
        );
    }

    #[test]
    fn the_delete_batch_covers_the_same_thirteen_tables() {
        // `DELETE` が 1 表でも欠けると、その表だけ古い行が残る (全差し替えの穴)。
        for name in TABLES {
            assert!(
                DELETE_TABLES.contains(&format!("DELETE FROM {name};")),
                "{name} の DELETE が抜けている"
            );
        }
        assert_eq!(
            DELETE_TABLES.matches("DELETE FROM").count(),
            TABLES.len(),
            "DELETE の本数と表の数が一致する"
        );
    }
}
