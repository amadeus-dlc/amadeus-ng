//! 壊れた保存物を**成功へ丸めない**契約 — 読取り側 DAO の破損・不在の扱い。
//!
//! 行が無いのは失敗ではない (`Ok(None)`) が、行があっても列が契約の幅に収まらない・
//! 登録簿が JSON でない・同じ鍵が重複するのは投影の破損であり、代わりの答えを作らずに
//! [`ReadModelReadError`] で上げる。
//!
//! [`ReadModelReadError`]: core_query_use_case::orchestration::ReadModelReadError

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::ErrorKind;

use core_query_interface_adapter::{IntentRecordDaoImpl, ReadModelDaos};
use core_query_use_case::orchestration::{IntentRecordDao, PlanApprovalOperationDao};

/// `read_plan_operation` 1 表だけを持つストアを、指定の `as_of` で組む。
fn plan_operation_store(dir: &tempfile::TempDir, as_of: i64) -> std::path::PathBuf {
    let path = dir.path().join("read-model.sqlite");
    let connection = rusqlite::Connection::open(&path).expect("フィクスチャのストアを作る");
    connection
        .execute_batch(
            "CREATE TABLE read_plan_operation(operation_id TEXT PRIMARY KEY, status TEXT NOT NULL, space TEXT, execution_id TEXT, as_of INTEGER NOT NULL, kind TEXT NOT NULL);",
        )
        .expect("表を作る");
    connection
        .execute(
            "INSERT INTO read_plan_operation VALUES ('op-1','prepared','default','exec-1',?1,'code-generation-plan')",
            [as_of],
        )
        .expect("行を書く");
    path
}

#[test]
fn a_plan_operation_row_with_a_negative_as_of_is_a_read_failure_not_a_view() {
    let dir = tempfile::tempdir().unwrap();
    let path = plan_operation_store(&dir, -1);
    let daos = ReadModelDaos::open(&path).expect("開ける");
    let dao = daos.plan_approval_operation();
    let pending = dao
        .find_pending()
        .expect_err("負の as_of は u64 に収まらない");
    assert_eq!(pending.kind(), ErrorKind::Other);
    assert_eq!(pending.path(), Some(path.as_path()));
    let found = dao.find("op-1").expect_err("同じ行は find でも失敗する");
    assert_eq!(found.kind(), ErrorKind::Other);
}

#[test]
fn a_plan_operation_row_within_range_is_returned_and_an_absent_key_is_none() {
    let dir = tempfile::tempdir().unwrap();
    let path = plan_operation_store(&dir, 42);
    let daos = ReadModelDaos::open(&path).expect("開ける");
    let dao = daos.plan_approval_operation();
    let pending = dao.find_pending().unwrap();
    assert_eq!(pending.len(), 1);
    let first = pending.first().expect("1 行");
    assert_eq!(first.id(), "op-1");
    assert_eq!(first.as_of(), 42);
    assert_eq!(first.kind(), "code-generation-plan");
    let found = dao.find("op-1").unwrap().expect("鍵が当たる");
    assert_eq!(found.status(), "prepared");
    assert_eq!(found.space(), Some("default"));
    assert_eq!(found.execution_id(), Some("exec-1"));
    assert_eq!(dao.find("op-9").unwrap(), None);
}

#[test]
fn an_intent_registry_that_is_missing_or_broken_is_a_read_failure_with_its_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("intents.json");
    let dao = IntentRecordDaoImpl::new(path.clone());
    let missing = dao.find("i-1").expect_err("登録簿が無い");
    assert_eq!(missing.kind(), ErrorKind::NotFound);
    assert_eq!(missing.path(), Some(path.as_path()));

    std::fs::write(&path, "{not json").unwrap();
    let broken = dao.find("i-1").expect_err("JSON でない登録簿");
    assert_eq!(broken.kind(), ErrorKind::InvalidData);
    assert_eq!(broken.path(), Some(path.as_path()));

    std::fs::write(&path, r#"{"uuid":"i-1","dirName":"x"}"#).unwrap();
    let not_array = dao.find("i-1").expect_err("配列でない登録簿");
    assert_eq!(not_array.kind(), ErrorKind::InvalidData);
}

#[test]
fn an_intent_registry_entry_is_found_once_and_a_duplicate_or_nameless_entry_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("intents.json");
    let dao = IntentRecordDaoImpl::new(path.clone());

    std::fs::write(
        &path,
        r#"[{"uuid":"i-0","dirName":"zero"},{"uuid":"i-1","dirName":"one-dir"}]"#,
    )
    .unwrap();
    let found = dao.find("i-1").unwrap().expect("鍵が当たる");
    assert_eq!(found.intent_id(), "i-1");
    assert_eq!(found.directory(), "one-dir");
    assert_eq!(dao.find("i-9").unwrap(), None, "無い鍵は不在");

    std::fs::write(
        &path,
        r#"[{"uuid":"i-1","dirName":"a"},{"uuid":"i-1","dirName":"b"}]"#,
    )
    .unwrap();
    let duplicated = dao.find("i-1").expect_err("同じ uuid が 2 件");
    assert_eq!(duplicated.kind(), ErrorKind::InvalidData);
    assert_eq!(duplicated.path(), Some(path.as_path()));

    std::fs::write(&path, r#"[{"uuid":"i-1"}]"#).unwrap();
    let nameless = dao.find("i-1").expect_err("dirName が無い");
    assert_eq!(nameless.kind(), ErrorKind::InvalidData);

    std::fs::write(&path, r#"[{"uuid":"i-1","dirName":7}]"#).unwrap();
    let typed = dao.find("i-1").expect_err("dirName が文字列でない");
    assert_eq!(typed.kind(), ErrorKind::InvalidData);
}
