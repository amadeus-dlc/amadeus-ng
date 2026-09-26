//! 構造化面の 20 表 (`read_*`) の表ごとの DAO の表駆動の単体試験。
//!
//! 表の DDL (主キー・自然キーの UNIQUE 索引・引当の索引・`as_of` 列) と、全行の削除・書込・
//! 内容の読取を、20 表まとめて確かめる。20 表を束ねるのは構造化面の手順
//! ([`super::structured_surface::StructuredSurface`]) なので、それを通して呼ぶ (表ごとの DAO の
//! 呼び分けを試験側に書き下さない)。1 表ごとの SQL は各 `*_dao_impl.rs` にあり、行の往復は
//! `tests/structured_read_model_updater_contract.rs` が実ストアの履歴で見る。
//!
//! 旧 `read_tables/sql.rs` (Issue #153 の PR4 で表ごとの DAO へ分けた) の単体試験の移し先である。

// 想定外ケースの即時失敗はテストの検証手段である (house style)。
#![allow(
    clippy::panic,
    reason = "表が作られていないことを名前つきで即時失敗させる (house style)"
)]

use std::collections::BTreeMap;

use rusqlite::{Connection, params};

use super::structured_surface::StructuredSurface;
use super::{
    ArtifactJournalEntry, CorruptCause, GlobalSeqNr, JournalBatch, JournalReadError,
    SteeringPartDao as _, SteeringPartDaoImpl, SteeringPlanDao as _, SteeringPlanDaoImpl,
};
use crate::read_tables::{MemoryRules, ReadTables, RuleContent, SteeringTables};

/// 構造化面の 20 表の名前 (表ごとの DAO の DDL と全行の削除が同じ集合を指していることを固定する)。
const TABLES: [&str; 20] = [
    "read_answer_result",
    "read_report_result",
    "read_jump_result",
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
    "read_session_audit",
    "read_artifact_audit",
];

/// 自然キーの UNIQUE インデックス (表・索引名・列)。
///
/// 主キーは代理キー `id` なので、**自然キーの重複を止めるのはこの索引だけ**である。
/// 集約そのものを表す 3 表 (`read_definition` / `read_intent` / `read_execution`) は
/// 自然キー = 主キーなので、ここには載らない。
const NATURAL_KEY_INDEXES: [(&str, &str, &[&str]); 12] = [
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
];

/// クエリ側が `WHERE` に置く列のセカンダリ索引 (表・索引名・列)。
///
/// 自然キーの UNIQUE 索引が左端前置で使える引当 (例 `read_execution_stage` を
/// `execution_id` で引く) はここに重ねない。
const LOOKUP_INDEXES: [(&str, &str, &[&str]); 5] = [
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
fn index_shape(connection: &Connection, table: &str, index: &str) -> Option<(Vec<String>, bool)> {
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

/// 実物の組の手順。
fn surface() -> StructuredSurface {
    StructuredSurface::default()
}

/// 20 表 (と管理表) を DAO で作ったメモリ DB。
fn created() -> Connection {
    let mut connection = Connection::open_in_memory().expect("メモリ DB は開ける");
    let mut transaction = connection.transaction().expect("Tx は張れる");
    surface().create_tables(&mut transaction).expect("DDL");
    transaction.commit().expect("commit");
    connection
}

/// `tables` で 20 表を差し替えて確定する。
fn replaced(connection: &mut Connection, tables: &ReadTables) -> Result<(), JournalReadError> {
    let mut transaction = connection.transaction().expect("Tx は張れる");
    surface().replace(&mut transaction, tables)?;
    transaction.commit().expect("commit");
    Ok(())
}

fn count(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("数えられる")
}

#[test]
fn every_table_has_a_single_primary_key_column_named_id() {
    // 複合主キーにしない (オーナー裁定 2026-09-03)。関連行は FK 列 1 つで指せる。
    let connection = created();
    for name in TABLES {
        assert_eq!(primary_key(&connection, name), ["id"], "{name}");
    }
}

#[test]
fn every_natural_key_has_a_unique_index() {
    let connection = created();
    for (table, index, columns) in NATURAL_KEY_INDEXES {
        let (found, unique) =
            index_shape(&connection, table, index).unwrap_or_else(|| panic!("{index} が無い"));
        assert_eq!(found, columns, "{index} の列");
        assert!(unique, "{index} は UNIQUE でなければ自然キーを守れない");
    }
}

#[test]
fn the_columns_the_query_side_filters_on_are_indexed() {
    let connection = created();
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
    let connection = created();
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
    let mut connection = created();
    let mut transaction = connection.transaction().expect("Tx は張れる");
    surface()
        .create_tables(&mut transaction)
        .expect("2 回目も通る (IF NOT EXISTS)");
    transaction.commit().expect("commit");
    assert!(surface().tables_exist(&connection).expect("読める"));
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
fn a_table_whose_index_is_missing_does_not_count_as_existing() {
    // 開く段は表と索引が揃っているかを見る。索引だけが欠けた表も作り直しの対象である —
    // 以前の開く段は毎回 `CREATE INDEX IF NOT EXISTS` を打ち、崩れた表の形 (索引の列の欠け) を
    // そこで見つけていた。その検出を保つ。
    for (_, index, _) in NATURAL_KEY_INDEXES.iter().chain(LOOKUP_INDEXES.iter()) {
        let connection = created();
        connection
            .execute_batch(&format!("DROP INDEX {index}"))
            .expect("索引を落とす");
        assert!(
            !surface().tables_exist(&connection).expect("読める"),
            "{index} が欠けても揃っていると見た"
        );
    }
}

#[test]
fn dropping_the_tables_removes_every_one_and_the_existence_check_sees_it() {
    let mut connection = created();
    let mut transaction = connection.transaction().expect("Tx は張れる");
    surface().drop_tables(&mut transaction).expect("落とせる");
    transaction.commit().expect("commit");
    assert!(!surface().tables_exist(&connection).expect("読める"));
    for name in TABLES {
        let found: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                params![name],
                |row| row.get(0),
            )
            .expect("読める");
        assert_eq!(found, 0, "{name} が残っている");
    }
}

#[test]
fn every_table_carries_the_as_of_column() {
    let connection = created();
    for name in TABLES {
        let count: i64 = connection
            .query_row(
                &format!("SELECT COUNT(*) FROM pragma_table_info('{name}') WHERE name = 'as_of'"),
                [],
                |row| row.get(0),
            )
            .expect("pragma は引ける");
        assert_eq!(count, 1, "{name} に as_of 列がある");
    }
}

#[test]
fn a_full_replacement_empties_every_table() {
    // 全行の削除が 1 表でも欠けると、その表だけ古い行が残る (全差し替えの穴)。
    let mut connection = created();
    for name in TABLES {
        let columns: Vec<(String, bool)> = connection
            .prepare(&format!(
                "SELECT name, \"notnull\" FROM pragma_table_info('{name}')"
            ))
            .expect("pragma は引ける")
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("問い合わせ")
            .collect::<Result<_, _>>()
            .expect("収集");
        let names: Vec<&str> = columns.iter().map(|(column, _)| column.as_str()).collect();
        let values: Vec<&str> = columns
            .iter()
            .map(|(column, not_null)| {
                if column == "id" {
                    "'sentinel'"
                } else if *not_null {
                    "0"
                } else {
                    "NULL"
                }
            })
            .collect();
        connection
            .execute(
                &format!(
                    "INSERT INTO {name} ({}) VALUES ({})",
                    names.join(", "),
                    values.join(", ")
                ),
                [],
            )
            .unwrap_or_else(|error| panic!("{name} に印を置けない: {error}"));
    }
    replaced(
        &mut connection,
        &ReadTables::project(&JournalBatch::empty()).expect("投影"),
    )
    .expect("差し替えられる");
    for name in TABLES {
        assert_eq!(count(&connection, name), 0, "{name} に古い行が残った");
    }
}

#[test]
fn the_journal_rows_and_the_steering_rows_are_replaced_independently() {
    // steering は別 Tx で差し替わる — ジャーナル側の全差し替えが steering の行を
    // 消してしまうと、参照入力が変わっていないのに束が消える。
    let mut connection = created();
    let steering = SteeringTables::pack(&MemoryRules::new(
        vec![RuleContent::new(
            "org.md".to_string(),
            "# Org\n".to_string(),
        )],
        BTreeMap::new(),
    ))
    .expect("パックできる");
    let mut transaction = connection.transaction().expect("Tx は張れる");
    SteeringPlanDaoImpl
        .create_table(&mut transaction)
        .expect("DDL");
    SteeringPartDaoImpl
        .create_table(&mut transaction)
        .expect("DDL");
    SteeringPlanDaoImpl
        .replace(&mut transaction, steering.plans(), steering.source_digest())
        .expect("書ける");
    SteeringPartDaoImpl
        .replace(&mut transaction, steering.parts())
        .expect("書ける");
    transaction.commit().expect("commit");

    replaced(
        &mut connection,
        &ReadTables::project(&JournalBatch::empty()).expect("投影"),
    )
    .expect("書ける");

    assert_eq!(
        count(&connection, "read_steering_part"),
        5,
        "ジャーナル側の差し替えは steering の行に触らない"
    );
}

#[test]
fn a_scan_position_that_does_not_fit_the_column_fails_instead_of_being_rounded() {
    // `as_of` は全表に同じ値で書かれる。`INTEGER` (i64) に収まらない走査位置を静かに
    // 丸めると、行が「いつ時点か」を偽る。収まらないなら書かずに `Corrupt` で止める。
    let mut connection = created();
    let tables = ReadTables::project(&JournalBatch::new(
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Some(GlobalSeqNr::new(u64::MAX)),
    ))
    .expect("空の履歴でも投影はできる");
    assert_eq!(
        replaced(&mut connection, &tables),
        Err(JournalReadError::Corrupt {
            aggregate_id: "-".to_string(),
            seq_nr: None,
            cause: CorruptCause::InvariantViolation,
        })
    );
}

#[test]
fn artifact_only_publication_is_repeatable_and_does_not_roll_back_newer_rows() {
    use core_command_domain::workspace::{
        ArtifactAudit, ArtifactWriteObservation, HookHealthTarget, IntentDirName, SpaceName,
    };
    let target = HookHealthTarget::new(
        SpaceName::default(),
        Some(IntentDirName::parse("260909-artifact").expect("名前")),
    );
    let at = chrono::DateTime::parse_from_rfc3339("2026-09-09T00:00:00Z")
        .expect("時刻")
        .with_timezone(&chrono::Utc);
    let (mut aggregate, first) = ArtifactAudit::start(
        ArtifactWriteObservation::new(
            target.clone(),
            "Write".into(),
            "first.md".into(),
            "first".into(),
            true,
        ),
        at,
    )
    .expect("開始");
    let second = aggregate
        .record(
            ArtifactWriteObservation::new(
                target,
                "Edit".into(),
                "second.md".into(),
                "second".into(),
                false,
            ),
            at,
        )
        .expect("記録");
    let first_entry = ArtifactJournalEntry::new(GlobalSeqNr::new(1), 1, at, first);
    let old = ReadTables::project_audit_only(
        &JournalBatch::new(vec![], vec![], vec![], Some(GlobalSeqNr::new(1)))
            .with_artifacts(vec![first_entry.clone()]),
    )
    .expect("投影");
    let current = ReadTables::project_audit_only(
        &JournalBatch::new(vec![], vec![], vec![], Some(GlobalSeqNr::new(2))).with_artifacts(vec![
            first_entry,
            ArtifactJournalEntry::new(GlobalSeqNr::new(2), 2, at, second),
        ]),
    )
    .expect("投影");
    let mut connection = created();
    connection
        .execute(
            "INSERT INTO read_answer_result (id,answer_id,execution_id,stage,disposition,as_of) VALUES ('answer','answer','execution','stage','recorded',1)",
            [],
        )
        .expect("既存の行");
    for tables in [&old, &current, &current, &old] {
        replaced(&mut connection, tables).expect("書ける");
    }
    let observed: (String, String, i64, i64) = connection
        .query_row(
            "SELECT file,tool,created,as_of FROM read_artifact_audit",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("行");
    assert_eq!(observed, ("second.md".into(), "Edit".into(), 0, 2));
    assert_eq!(count(&connection, "read_artifact_audit"), 1);
    assert_eq!(
        count(&connection, "read_answer_result"),
        1,
        "監査だけの断面は既存の行を消さない"
    );
}

#[test]
fn the_content_of_the_twenty_tables_follows_their_rows() {
    let mut connection = created();
    let empty = surface().content(&connection).expect("読める");
    connection
        .execute(
            "INSERT INTO read_scope_change(id, execution_id, scope, kind, as_of)
             VALUES ('row', 'e', 's', 'k', 0)",
            [],
        )
        .expect("行を足す");
    let one = surface().content(&connection).expect("読める");
    assert_ne!(empty, one);
    assert_ne!(
        empty.digest().expect("ダイジェスト"),
        one.digest().expect("ダイジェスト")
    );
    replaced(
        &mut connection,
        &ReadTables::project(&JournalBatch::empty()).expect("投影"),
    )
    .expect("書ける");
    assert_eq!(surface().content(&connection).expect("読める"), empty);
}
