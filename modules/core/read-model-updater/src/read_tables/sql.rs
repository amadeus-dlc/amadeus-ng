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
//! # 表の形 — 単一主キー + FK + インデックス (オーナー裁定 2026-09-03)
//!
//! 主キーはどの表も **1 列 `id`** で、複合主キーにしない。集約そのものを表す 3 表は集約 id
//! を、それ以外は自然キーから導いた代理キー (`super::row_id`) を `id` に置く。自然キーの列は
//! 残り、その重複は [`CREATE_INDEXES`] の UNIQUE 索引が止める。関連行は FK 列で指し、
//! `FOREIGN KEY` 句は書かない (steering の 2 表が別 Tx で差し替わるため — [`CREATE_TABLES`]
//! の doc を参照)。クエリ側が `WHERE` に置く列にはセカンダリ索引を張る。
//!
//! [`ReadTables::as_of`]: super::ReadTables::as_of

use rusqlite::{Connection, Transaction, params};

use super::ReadTables;
use super::steering_tables::SteeringTables;

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

/// 17 表の DDL (この順に作る — ジャーナル由来 15 + 参照入力由来 2)。
///
/// **主キーはどの表も 1 列 `id`** である (オーナー裁定 2026-09-03 — 基本的な関係
/// モデリング)。集約そのものを表す 3 表 (`read_definition` / `read_intent` /
/// `read_execution`) の `id` は集約 id そのもので、それ以外は自然キーから導いた代理キー
/// (`row_id`) である。自然キーの列は残し、重複は UNIQUE 索引 ([`CREATE_INDEXES`]) が止める。
///
/// **`FOREIGN KEY` 句は書かない。** FK は列名と doc で表す — steering の 2 表は別の投影
/// 単位として**別トランザクション**で差し替わるので、参照整合を DB に強制させると
/// 投影の順序と衝突して書けなくなる断面が生まれる。対が揃っていることは投影核の契約
/// テストが固定する。
const CREATE_TABLES: &str = "
CREATE TABLE IF NOT EXISTS read_definition (
  id          TEXT PRIMARY KEY,
  revision    TEXT    NOT NULL,
  stage_count INTEGER NOT NULL,
  scope_count INTEGER NOT NULL,
  as_of       INTEGER NOT NULL
);
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
CREATE TABLE IF NOT EXISTS read_definition_scope (
  id                   TEXT    PRIMARY KEY,
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
  as_of                INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS read_definition_scope_keyword (
  id            TEXT    PRIMARY KEY,
  definition_id TEXT    NOT NULL,
  keyword       TEXT    NOT NULL,
  scope         TEXT    NOT NULL,
  as_of         INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS read_definition_scope_stage (
  id             TEXT    PRIMARY KEY,
  definition_id  TEXT    NOT NULL,
  scope          TEXT    NOT NULL,
  stage_slug     TEXT    NOT NULL,
  action         TEXT,
  in_scope_order INTEGER,
  as_of          INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS read_definition_scope_phase_entry (
  id               TEXT    PRIMARY KEY,
  definition_id    TEXT    NOT NULL,
  scope            TEXT    NOT NULL,
  phase            TEXT    NOT NULL,
  first_stage_slug TEXT    NOT NULL,
  as_of            INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS read_intent (
  id                  TEXT PRIMARY KEY,
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
  id          TEXT    PRIMARY KEY,
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
  as_of       INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS read_execution (
  id               TEXT PRIMARY KEY,
  intent_id        TEXT    NOT NULL,
  scope            TEXT    NOT NULL,
  status           TEXT    NOT NULL,
  cursor_index     INTEGER,
  cursor_slug      TEXT,
  parked_at_index  INTEGER,
  parked_at_slug   TEXT,
  parked_active    INTEGER NOT NULL,
  accepts_commands INTEGER NOT NULL,
  autonomy         TEXT    NOT NULL,
  skeleton_stance  TEXT,
  seq_nr           INTEGER NOT NULL,
  last_updated_at  TEXT    NOT NULL,
  state_binding    TEXT    NOT NULL,
  as_of            INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS read_execution_stage (
  id             TEXT    PRIMARY KEY,
  execution_id   TEXT    NOT NULL,
  stage_index    INTEGER NOT NULL,
  slug           TEXT    NOT NULL,
  phase          TEXT    NOT NULL,
  checkbox       TEXT,
  effective_plan TEXT,
  approved       INTEGER,
  revision_count INTEGER,
  gated          INTEGER,
  as_of          INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS read_next_answer (
  id            TEXT    PRIMARY KEY,
  execution_id  TEXT    NOT NULL,
  request_kind  TEXT    NOT NULL,
  decision_kind TEXT    NOT NULL,
  stage_index   INTEGER,
  stage_slug    TEXT,
  gate          TEXT,
  checkbox      TEXT,
  run_stage_id  TEXT,
  as_of         INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS read_next_jump (
  id           TEXT    PRIMARY KEY,
  execution_id TEXT    NOT NULL,
  target_index INTEGER NOT NULL,
  target_slug  TEXT    NOT NULL,
  outcome      TEXT    NOT NULL,
  refusal      TEXT,
  as_of        INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS read_next_jump_phase (
  id           TEXT    PRIMARY KEY,
  execution_id TEXT    NOT NULL,
  phase        TEXT    NOT NULL,
  target_index INTEGER NOT NULL,
  target_slug  TEXT,
  as_of        INTEGER NOT NULL
);
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
CREATE TABLE IF NOT EXISTS read_scope_change (
  id           TEXT    PRIMARY KEY,
  execution_id TEXT    NOT NULL,
  scope        TEXT    NOT NULL,
  kind         TEXT    NOT NULL,
  as_of        INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS read_steering_plan (
  id              TEXT PRIMARY KEY,
  phase           TEXT    NOT NULL,
  bundle_digest   TEXT    NOT NULL,
  part_count      INTEGER NOT NULL,
  delivered_paths TEXT    NOT NULL,
  source_digest   TEXT    NOT NULL
);
CREATE TABLE IF NOT EXISTS read_steering_part (
  id               TEXT    PRIMARY KEY,
  steering_plan_id TEXT    NOT NULL,
  phase            TEXT    NOT NULL,
  part_index       INTEGER NOT NULL,
  rules_content    TEXT    NOT NULL
);
";

/// 索引 — 自然キーの UNIQUE と、クエリ側が `WHERE` に置く列のセカンダリ。
///
/// 主キーが代理キー `id` になったぶん、**自然キーの重複を止めるのは UNIQUE 索引だけ**で
/// ある。並びは DDL と同じ順で、UNIQUE を先に、セカンダリを後に置く。
///
/// セカンダリは「引く列」にだけ張る。自然キーの UNIQUE 索引が**左端前置**で使える引当
/// (例: `read_execution_stage` を `execution_id` だけで引く) には重ねない。子から親へ
/// FK をたどる引当 (`read_next_answer.run_stage_id` → `read_run_stage.id` など) は親の
/// 主キーを使うので、子側の FK 列には索引を張らない。
const CREATE_INDEXES: &str = "
CREATE UNIQUE INDEX IF NOT EXISTS read_definition_stage_key
  ON read_definition_stage(definition_id, stage_slug);
CREATE UNIQUE INDEX IF NOT EXISTS read_definition_scope_key
  ON read_definition_scope(definition_id, scope);
CREATE UNIQUE INDEX IF NOT EXISTS read_definition_scope_keyword_key
  ON read_definition_scope_keyword(definition_id, keyword);
CREATE UNIQUE INDEX IF NOT EXISTS read_definition_scope_stage_key
  ON read_definition_scope_stage(definition_id, scope, stage_slug);
CREATE UNIQUE INDEX IF NOT EXISTS read_definition_scope_phase_entry_key
  ON read_definition_scope_phase_entry(definition_id, scope, phase);
CREATE UNIQUE INDEX IF NOT EXISTS read_intent_stage_key
  ON read_intent_stage(intent_id, stage_index);
CREATE UNIQUE INDEX IF NOT EXISTS read_execution_stage_key
  ON read_execution_stage(execution_id, stage_index);
CREATE UNIQUE INDEX IF NOT EXISTS read_next_answer_key
  ON read_next_answer(execution_id, request_kind);
CREATE UNIQUE INDEX IF NOT EXISTS read_next_jump_key
  ON read_next_jump(execution_id, target_index);
CREATE UNIQUE INDEX IF NOT EXISTS read_next_jump_phase_key
  ON read_next_jump_phase(execution_id, phase);
CREATE UNIQUE INDEX IF NOT EXISTS read_run_stage_key
  ON read_run_stage(definition_id, scope, stage_slug);
CREATE UNIQUE INDEX IF NOT EXISTS read_scope_change_key
  ON read_scope_change(execution_id, scope);
CREATE UNIQUE INDEX IF NOT EXISTS read_steering_plan_key
  ON read_steering_plan(phase);
CREATE UNIQUE INDEX IF NOT EXISTS read_steering_part_key
  ON read_steering_part(phase, part_index);
CREATE INDEX IF NOT EXISTS read_intent_definition_id
  ON read_intent(definition_id);
CREATE INDEX IF NOT EXISTS read_execution_intent_id
  ON read_execution(intent_id);
CREATE INDEX IF NOT EXISTS read_execution_state_binding
  ON read_execution(state_binding);
CREATE INDEX IF NOT EXISTS read_run_stage_digests
  ON read_run_stage(route_digest, directive_digest);
CREATE INDEX IF NOT EXISTS read_next_jump_target_slug
  ON read_next_jump(execution_id, target_slug);
CREATE INDEX IF NOT EXISTS read_steering_plan_bundle_digest
  ON read_steering_plan(bundle_digest);
CREATE INDEX IF NOT EXISTS read_steering_part_plan
  ON read_steering_part(steering_plan_id, part_index);
";

/// ジャーナル由来 15 表の全差し替えの `DELETE` (DDL と同じ順)。
///
/// steering の 2 表はここに**含めない** — 別の投影単位であり、別 Tx で差し替わる。
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
DELETE FROM read_run_stage;
DELETE FROM read_scope_change;
";

/// 参照入力由来 2 表の全差し替えの `DELETE`。
const DELETE_STEERING_TABLES: &str = "
DELETE FROM read_steering_plan;
DELETE FROM read_steering_part;
";

/// 読み面スキーマの版 (`PRAGMA user_version` に保存する値)。
///
/// **列の形を変えたら必ず 1 つ上げる。** `CREATE TABLE IF NOT EXISTS` は既存の表には
/// 何もしないので、列を足す・落とす・型を変える改訂は旧スキーマの表が残ったままになり、
/// `INSERT` が `no such column` で落ちる (b47 の `read_next_answer.gated INTEGER` →
/// `gate TEXT` が実例)。版が動いていれば [`recreate_tables`] が落として作り直す。
///
/// 行の正本はジャーナルであって読み面ではないので、**作り直しは情報を失わない**。
/// 「後方互換を残さない」(`coding-rules/no-backward-compatibility.md`) はコードの規則で
/// あり、機械が読む媒体を捨てて描き直すのはその帰結である。
pub(crate) const READ_SCHEMA_VERSION: i64 = 1;

/// 17 表の `DROP` (版が動いたときだけ打つ — 索引は表と一緒に落ちる)。
///
/// 落とすのは `read_*` 17 表**だけ**である。ジャーナル (`journal`) とスナップショット
/// (`snapshot`) は本家の表であり、チェックポイント表 (`amadeus_projection_checkpoint`) は
/// Markdown 面と共有の位置なので、どちらもここには現れない。
const DROP_TABLES: &str = "
DROP TABLE IF EXISTS read_definition;
DROP TABLE IF EXISTS read_definition_stage;
DROP TABLE IF EXISTS read_definition_scope;
DROP TABLE IF EXISTS read_definition_scope_keyword;
DROP TABLE IF EXISTS read_definition_scope_stage;
DROP TABLE IF EXISTS read_definition_scope_phase_entry;
DROP TABLE IF EXISTS read_intent;
DROP TABLE IF EXISTS read_intent_stage;
DROP TABLE IF EXISTS read_execution;
DROP TABLE IF EXISTS read_execution_stage;
DROP TABLE IF EXISTS read_next_answer;
DROP TABLE IF EXISTS read_next_jump;
DROP TABLE IF EXISTS read_next_jump_phase;
DROP TABLE IF EXISTS read_run_stage;
DROP TABLE IF EXISTS read_scope_change;
DROP TABLE IF EXISTS read_steering_plan;
DROP TABLE IF EXISTS read_steering_part;
";

/// 保存されている読み面スキーマの版 (未設定の DB は `0`)。
///
/// `PRAGMA user_version` は SQLite がヘッダに持つ 32bit の欄で、本家の event-store も
/// 我々のチェックポイント表も使っていない (実測。同じ DB ファイルの中で衝突しない)。
///
/// # Errors
///
/// SQLite の失敗をそのまま返す (呼出側が I/O の失敗へ写す)。
pub(crate) fn read_schema_version(connection: &Connection) -> Result<i64, rusqlite::Error> {
    connection.query_row("PRAGMA user_version", [], |row| row.get(0))
}

/// 読み面スキーマの版を書く。
///
/// `PRAGMA` は束縛変数を取れないので、値は `i64` として整形する (外から来る文字列は
/// 通らないので SQL の注入面は無い)。
///
/// # Errors
///
/// SQLite の失敗をそのまま返す (呼出側が I/O の失敗へ写す)。
pub(crate) fn set_schema_version(
    connection: &Connection,
    version: i64,
) -> Result<(), rusqlite::Error> {
    connection.execute_batch(&format!("PRAGMA user_version = {version}"))
}

/// 17 表を落として作り直す (版が動いたときの作り直し)。
///
/// 落とすのは `read_*` だけで、ジャーナル・スナップショット・チェックポイントは触らない。
///
/// # Errors
///
/// SQLite の失敗をそのまま返す (呼出側が I/O の失敗へ写す)。
pub(crate) fn recreate_tables(connection: &Connection) -> Result<(), rusqlite::Error> {
    connection.execute_batch(DROP_TABLES)?;
    ensure_tables(connection)
}

/// 17 表と索引を (無ければ) 作る。冪等なので何度呼んでもよい。
///
/// 索引は表と同じ口で作る — 表だけ在って UNIQUE 索引が無い断面を作ると、自然キーの
/// 重複が静かに通る。
///
/// # Errors
///
/// SQLite の失敗をそのまま返す (呼出側が I/O の失敗へ写す)。
pub(crate) fn ensure_tables(connection: &Connection) -> Result<(), rusqlite::Error> {
    connection.execute_batch(CREATE_TABLES)?;
    connection.execute_batch(CREATE_INDEXES)
}

/// ジャーナル由来 15 表の行を全部差し替える。
///
/// トランザクションは**呼出側が持つ** — チェックポイントの前進と同じ 1 つの Tx に閉じる
/// ためである (裁定 §3)。
///
/// # Errors
///
/// SQLite の失敗をそのまま返す (呼出側が I/O の失敗へ写す)。
#[allow(
    clippy::too_many_lines,
    reason = "15 表の INSERT を 1 か所に並べる — 表と列の対応が一覧で読めることを優先する"
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
             (id, revision, stage_count, scope_count, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                row.id(),
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
             (id, definition_id, stage_slug, position, number, name, phase, execution,
              condition, lead_agent, support_agents, mode, for_each, workspace_requires,
              produces, optional_produces, produces_kinds, consumes, requires_stage, sensors,
              scopes, reviewer, reviewer_max_iterations, review_class, summary_confirmation,
              plugin, enabled, gated, inputs, outputs, rules_in_context, sensors_applicable,
              as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                     ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30,
                     ?31, ?32, ?33)",
            params![
                row.id(),
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
             (id, definition_id, scope, depth, keywords, skeleton, review_cap,
              freeform_default, has_grid_column, cost_total, cost_execute, cost_gates,
              cost_per_unit_stages, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                row.id(),
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
             (id, definition_id, keyword, scope, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                row.id(),
                row.definition_id(),
                row.keyword(),
                row.scope(),
                as_of
            ],
        )?;
    }

    for row in tables.definition_scope_stages() {
        transaction.execute(
            "INSERT INTO read_definition_scope_stage
             (id, definition_id, scope, stage_slug, action, in_scope_order, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                row.id(),
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
             (id, definition_id, scope, phase, first_stage_slug, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                row.id(),
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
             (id, definition_id, definition_revision, scope, request, depth,
              test_strategy, review, created_at, project_type, project_kind, languages,
              frameworks, build_system, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                row.id(),
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
             (id, intent_id, stage_index, slug, phase, plan_action, conditional, number, name,
              lead_agent, gated, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                row.id(),
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
             (id, intent_id, scope, status, cursor_index, cursor_slug,
              parked_at_index, parked_at_slug, parked_active, accepts_commands, autonomy,
              skeleton_stance, seq_nr, last_updated_at, state_binding, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                row.id(),
                row.intent_id(),
                row.scope(),
                row.status(),
                optional_integer(row.cursor_index())?,
                row.cursor_slug(),
                optional_integer(row.parked_at_index())?,
                row.parked_at_slug(),
                row.parked_active(),
                row.accepts_commands(),
                row.autonomy(),
                row.skeleton_stance(),
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
             (id, execution_id, stage_index, slug, phase, checkbox, effective_plan, approved,
              revision_count, gated, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                row.id(),
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
             (id, execution_id, request_kind, decision_kind, stage_index, stage_slug, gate,
              checkbox, run_stage_id, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                row.id(),
                row.execution_id(),
                row.request_kind(),
                row.decision_kind(),
                optional_integer(row.stage_index())?,
                row.stage_slug(),
                row.gate(),
                row.checkbox(),
                row.run_stage_id(),
                as_of
            ],
        )?;
    }

    for row in tables.next_jumps() {
        transaction.execute(
            "INSERT INTO read_next_jump
             (id, execution_id, target_index, target_slug, outcome, refusal, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                row.id(),
                row.execution_id(),
                integer(row.target_index())?,
                row.target_slug(),
                row.outcome(),
                row.refusal(),
                as_of
            ],
        )?;
    }

    for row in tables.run_stages() {
        transaction.execute(
            "INSERT INTO read_run_stage
             (id, definition_id, scope, stage_slug, phase, steering_plan_id, lead_agent,
              support_agents, mode, gate_default, in_scope, inline_context_paths_rel,
              stage_file_rel, memory_path_rel, consumes_rel, produces_rel, sensors_applicable,
              reviewer, reviewer_max_iterations, review_class, protocol_modules, next_stage_name,
              route_digest, directive_digest, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                     ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25)",
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
        )?;
    }

    for row in tables.scope_changes() {
        transaction.execute(
            "INSERT INTO read_scope_change
             (id, execution_id, scope, kind, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![row.id(), row.execution_id(), row.scope(), row.kind(), as_of],
        )?;
    }

    for row in tables.next_jump_phases() {
        transaction.execute(
            "INSERT INTO read_next_jump_phase
             (id, execution_id, phase, target_index, target_slug, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                row.id(),
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

/// steering の 2 表の行を全部差し替える。
///
/// **ジャーナル由来の差し替えとは別のトランザクションである** — 参照入力はジャーナルの
/// 走査位置と無関係に変わるので、チェックポイントの前進と束ねる理由が無い。整合性の鍵は
/// `source_digest` であり、行と一緒に書かれる (設計 §3)。
///
/// トランザクションは**呼出側が持つ** (ジャーナル側と同じ流儀)。
///
/// # Errors
///
/// SQLite の失敗をそのまま返す (呼出側が I/O の失敗へ写す)。
pub(crate) fn replace_steering(
    transaction: &Transaction<'_>,
    tables: &SteeringTables,
) -> Result<(), rusqlite::Error> {
    transaction.execute_batch(DELETE_STEERING_TABLES)?;
    // 素になった参照入力はスナップショット全体の性質なので、全行に同じ値を書く
    // (`as_of` と同じ流儀)。
    let source_digest = tables.source_digest();

    for row in tables.plans() {
        transaction.execute(
            "INSERT INTO read_steering_plan
             (id, phase, bundle_digest, part_count, delivered_paths, source_digest)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                row.id(),
                row.phase(),
                row.bundle_digest(),
                integer(row.part_count())?,
                row.delivered_paths(),
                source_digest
            ],
        )?;
    }

    for row in tables.parts() {
        transaction.execute(
            "INSERT INTO read_steering_part
             (id, steering_plan_id, phase, part_index, rules_content)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                row.id(),
                row.steering_plan_id(),
                row.phase(),
                integer(row.part_index())?,
                row.rules_content()
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
    use crate::read_tables::{MemoryRules, RuleContent};
    use std::collections::BTreeMap;

    /// ジャーナル由来の表の名前 (DDL と `DELETE` が同じ集合を指していることを固定する)。
    const TABLES: [&str; 15] = [
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
        "read_run_stage",
        "read_scope_change",
    ];

    /// 参照入力由来の表の名前 (別 Tx で差し替わる — `as_of` を持たない)。
    const STEERING_TABLES: [&str; 2] = ["read_steering_plan", "read_steering_part"];

    /// 自然キーの UNIQUE インデックス (表・索引名・列)。
    ///
    /// 主キーは代理キー `id` なので、**自然キーの重複を止めるのはこの索引だけ**である。
    /// 集約そのものを表す 3 表 (`read_definition` / `read_intent` / `read_execution`) は
    /// 自然キー = 主キーなので、ここには載らない。
    const NATURAL_KEY_INDEXES: [(&str, &str, &[&str]); 14] = [
        (
            "read_definition_stage",
            "read_definition_stage_key",
            &["definition_id", "stage_slug"],
        ),
        (
            "read_definition_scope",
            "read_definition_scope_key",
            &["definition_id", "scope"],
        ),
        (
            "read_definition_scope_keyword",
            "read_definition_scope_keyword_key",
            &["definition_id", "keyword"],
        ),
        (
            "read_definition_scope_stage",
            "read_definition_scope_stage_key",
            &["definition_id", "scope", "stage_slug"],
        ),
        (
            "read_definition_scope_phase_entry",
            "read_definition_scope_phase_entry_key",
            &["definition_id", "scope", "phase"],
        ),
        (
            "read_intent_stage",
            "read_intent_stage_key",
            &["intent_id", "stage_index"],
        ),
        (
            "read_execution_stage",
            "read_execution_stage_key",
            &["execution_id", "stage_index"],
        ),
        (
            "read_next_answer",
            "read_next_answer_key",
            &["execution_id", "request_kind"],
        ),
        (
            "read_next_jump",
            "read_next_jump_key",
            &["execution_id", "target_index"],
        ),
        (
            "read_next_jump_phase",
            "read_next_jump_phase_key",
            &["execution_id", "phase"],
        ),
        (
            "read_run_stage",
            "read_run_stage_key",
            &["definition_id", "scope", "stage_slug"],
        ),
        (
            "read_scope_change",
            "read_scope_change_key",
            &["execution_id", "scope"],
        ),
        ("read_steering_plan", "read_steering_plan_key", &["phase"]),
        (
            "read_steering_part",
            "read_steering_part_key",
            &["phase", "part_index"],
        ),
    ];

    /// クエリ側が `WHERE` に置く列のセカンダリ索引 (表・索引名・列)。
    ///
    /// 自然キーの UNIQUE 索引が左端前置で使える引当 (例 `read_execution_stage` を
    /// `execution_id` で引く) はここに重ねない。
    const LOOKUP_INDEXES: [(&str, &str, &[&str]); 7] = [
        (
            "read_intent",
            "read_intent_definition_id",
            &["definition_id"],
        ),
        ("read_execution", "read_execution_intent_id", &["intent_id"]),
        (
            "read_execution",
            "read_execution_state_binding",
            &["state_binding"],
        ),
        (
            "read_run_stage",
            "read_run_stage_digests",
            &["route_digest", "directive_digest"],
        ),
        (
            "read_next_jump",
            "read_next_jump_target_slug",
            &["execution_id", "target_slug"],
        ),
        (
            "read_steering_plan",
            "read_steering_plan_bundle_digest",
            &["bundle_digest"],
        ),
        (
            "read_steering_part",
            "read_steering_part_plan",
            &["steering_plan_id", "part_index"],
        ),
    ];

    /// 表の主キー列 (`pk` の昇順)。
    fn primary_key(connection: &Connection, table: &str) -> Vec<String> {
        let mut statement = connection
            .prepare(&format!(
                "SELECT name FROM pragma_table_info('{table}') WHERE pk > 0 ORDER BY pk"
            ))
            .expect("pragma は引ける");
        statement
            .query_map([], |row| row.get(0))
            .expect("問い合わせ")
            .collect::<Result<Vec<String>, _>>()
            .expect("収集")
    }

    /// 索引の列 (`seqno` の昇順) と一意性。索引が無ければ `None`。
    fn index_shape(
        connection: &Connection,
        table: &str,
        index: &str,
    ) -> Option<(Vec<String>, bool)> {
        let unique: bool = connection
            .query_row(
                &format!("SELECT \"unique\" FROM pragma_index_list('{table}') WHERE name = ?1"),
                params![index],
                |row| row.get(0),
            )
            .ok()?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT name FROM pragma_index_info('{index}') ORDER BY seqno"
            ))
            .expect("pragma は引ける");
        let columns = statement
            .query_map([], |row| row.get(0))
            .expect("問い合わせ")
            .collect::<Result<Vec<String>, _>>()
            .expect("収集");
        Some((columns, unique))
    }

    #[test]
    fn every_table_has_a_single_primary_key_column_named_id() {
        // 複合主キーにしない (オーナー裁定 2026-09-03)。関連行は FK 列 1 つで指せる。
        let connection = Connection::open_in_memory().expect("メモリ DB は開ける");
        ensure_tables(&connection).expect("DDL");
        for name in TABLES.into_iter().chain(STEERING_TABLES) {
            assert_eq!(primary_key(&connection, name), ["id"], "{name}");
        }
    }

    #[test]
    fn every_natural_key_has_a_unique_index() {
        let connection = Connection::open_in_memory().expect("メモリ DB は開ける");
        ensure_tables(&connection).expect("DDL");
        for (table, index, columns) in NATURAL_KEY_INDEXES {
            let (found, unique) =
                index_shape(&connection, table, index).unwrap_or_else(|| panic!("{index} が無い"));
            assert_eq!(found, columns, "{index} の列");
            assert!(unique, "{index} は UNIQUE でなければ自然キーを守れない");
        }
    }

    #[test]
    fn the_columns_the_query_side_filters_on_are_indexed() {
        let connection = Connection::open_in_memory().expect("メモリ DB は開ける");
        ensure_tables(&connection).expect("DDL");
        for (table, index, columns) in LOOKUP_INDEXES {
            let (found, _) =
                index_shape(&connection, table, index).unwrap_or_else(|| panic!("{index} が無い"));
            assert_eq!(found, columns, "{index} の列");
        }
    }

    #[test]
    fn a_second_row_with_the_same_natural_key_is_rejected_even_under_a_new_id() {
        // 代理キーが違えば主キーは通る。自然キーの重複を止めるのは UNIQUE 索引であり、
        // それが無いと全差し替えの取りこぼしが二重行として静かに積もる。
        let connection = Connection::open_in_memory().expect("メモリ DB は開ける");
        ensure_tables(&connection).expect("DDL");
        let insert = "INSERT INTO read_next_answer
             (id, execution_id, request_kind, decision_kind, as_of)
             VALUES (?1, 'e1', 'bare', 'done', 0)";
        connection
            .execute(insert, params!["first"])
            .expect("1 行目");
        let error = connection
            .execute(insert, params!["second"])
            .expect_err("同じ自然キーの 2 行目");
        assert!(
            error.to_string().to_uppercase().contains("UNIQUE"),
            "UNIQUE 制約で落ちる (実際: {error})"
        );
    }

    #[test]
    fn the_ddl_creates_every_table_and_is_idempotent() {
        let connection = Connection::open_in_memory().expect("メモリ DB は開ける");
        ensure_tables(&connection).expect("初回の DDL");
        ensure_tables(&connection).expect("2 回目も通る (IF NOT EXISTS)");
        for name in TABLES.into_iter().chain(STEERING_TABLES) {
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

    /// 表の列の有無 (`pragma_table_info` の 1 行検索)。
    fn has_column(connection: &Connection, table: &str, column: &str) -> bool {
        let count: i64 = connection
            .query_row(
                &format!("SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name = ?1"),
                params![column],
                |row| row.get(0),
            )
            .expect("pragma は引ける");
        count == 1
    }

    #[test]
    fn the_steering_tables_carry_no_scan_position_and_name_their_source_instead() {
        // steering の面は参照入力由来である — ジャーナルの走査位置とは無関係なので
        // `as_of` を持たない。いつ時点かを名乗るのは `source_digest` である。
        let connection = Connection::open_in_memory().expect("メモリ DB は開ける");
        ensure_tables(&connection).expect("DDL");
        for name in STEERING_TABLES {
            assert!(!has_column(&connection, name, "as_of"), "{name}");
        }
        assert!(has_column(
            &connection,
            "read_steering_plan",
            "source_digest"
        ));
    }

    #[test]
    fn the_steering_rows_replace_wholesale_and_report_the_source_they_came_from() {
        let mut connection = Connection::open_in_memory().expect("メモリ DB は開ける");
        ensure_tables(&connection).expect("DDL");

        let big = "x".repeat(12 * 1024);
        let rules = MemoryRules::new(
            vec![RuleContent::new(
                "org.md".to_string(),
                format!("# A\n{big}\n# B\n{big}\n"),
            )],
            BTreeMap::new(),
        );
        let tables = SteeringTables::pack(&rules).expect("パックできる");
        let transaction = connection.transaction().expect("Tx は張れる");
        replace_steering(&transaction, &tables).expect("書ける");
        transaction.commit().expect("commit");

        let plans: i64 = connection
            .query_row("SELECT COUNT(*) FROM read_steering_plan", [], |row| {
                row.get(0)
            })
            .expect("引ける");
        assert_eq!(plans, 5, "5 フェーズすべてに計画の行が立つ");
        let parts: i64 = connection
            .query_row("SELECT COUNT(*) FROM read_steering_part", [], |row| {
                row.get(0)
            })
            .expect("引ける");
        assert_eq!(parts, 10, "2 部 × 5 フェーズ");
        let digest: String = connection
            .query_row(
                "SELECT source_digest FROM read_steering_plan ORDER BY phase LIMIT 1",
                [],
                |row| row.get(0),
            )
            .expect("引ける");
        assert_eq!(digest, tables.source_digest());

        // 2 度目は全差し替え — 前の行が残らない。
        let smaller = SteeringTables::pack(&MemoryRules::default()).expect("空も計画できる");
        let transaction = connection.transaction().expect("Tx は張れる");
        replace_steering(&transaction, &smaller).expect("書ける");
        transaction.commit().expect("commit");
        let parts: i64 = connection
            .query_row("SELECT COUNT(*) FROM read_steering_part", [], |row| {
                row.get(0)
            })
            .expect("引ける");
        assert_eq!(parts, 0, "古い部が残らない");
    }

    #[test]
    fn the_journal_rows_and_the_steering_rows_are_replaced_independently() {
        // steering は別 Tx で差し替わる — ジャーナル側の全差し替えが steering の行を
        // 消してしまうと、参照入力が変わっていないのに束が消える。
        let mut connection = Connection::open_in_memory().expect("メモリ DB は開ける");
        ensure_tables(&connection).expect("DDL");
        let steering = SteeringTables::pack(&MemoryRules::new(
            vec![RuleContent::new(
                "org.md".to_string(),
                "# Org\n".to_string(),
            )],
            BTreeMap::new(),
        ))
        .expect("パックできる");
        let transaction = connection.transaction().expect("Tx は張れる");
        replace_steering(&transaction, &steering).expect("書ける");
        transaction.commit().expect("commit");

        let transaction = connection.transaction().expect("Tx は張れる");
        replace_all(
            &transaction,
            &ReadTables::project(&JournalBatch::empty()).expect("投影"),
        )
        .expect("書ける");
        transaction.commit().expect("commit");

        let parts: i64 = connection
            .query_row("SELECT COUNT(*) FROM read_steering_part", [], |row| {
                row.get(0)
            })
            .expect("引ける");
        assert_eq!(parts, 5, "ジャーナル側の差し替えは steering の行に触らない");
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
    fn the_delete_batch_covers_the_same_journal_tables() {
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
