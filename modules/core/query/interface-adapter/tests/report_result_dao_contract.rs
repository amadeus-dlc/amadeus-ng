//! 報告結果をIDで分離して返すQuery契約。
#![allow(clippy::unwrap_used, clippy::expect_used)]
use core_query_interface_adapter::ReadModelDaos;
use core_query_use_case::orchestration::ReportResultUseCase;
use rusqlite::Connection;
#[test]
fn querying_a_report_does_not_substitute_another_reports_result() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("read.db");
    let db = Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE read_report_result(report_id TEXT, execution_id TEXT, stage TEXT, scope TEXT, result_kind TEXT, steps TEXT, no_op_reason TEXT, current_stage TEXT); INSERT INTO read_report_result VALUES ('report-a','execution','requirements-analysis','bugfix','committed','[\"approve\"]',NULL,NULL); INSERT INTO read_report_result VALUES ('report-b','execution','code-generation','bugfix','no_op','[]','already_awaiting',NULL);").unwrap();
    let query = ReportResultUseCase::new(ReadModelDaos::open(&path).unwrap().report_result());
    assert!(query.execute("absent").unwrap().is_none());
    let first = query.execute("report-a").unwrap();
    assert!(first.is_some(), "指定した報告の結果が必要");
    assert_eq!(first.unwrap().stage(), "requirements-analysis");
    assert_eq!(
        query.execute("report-b").unwrap().unwrap().no_op_reason(),
        Some("already_awaiting")
    );
}

#[test]
fn absent_storage_is_an_error_and_is_not_created_by_a_query() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("missing.db");
    assert!(ReadModelDaos::open(&path).is_err());
    assert!(!path.exists());
}

#[test]
fn a_missing_report_table_is_not_an_empty_success() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("read.db");
    drop(Connection::open(&path).unwrap());
    let query = ReportResultUseCase::new(ReadModelDaos::open(&path).unwrap().report_result());
    assert!(query.execute("report-a").is_err());
}

#[test]
fn querying_does_not_repair_an_incompatible_result_schema() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("read.db");
    let db = Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE read_report_result(report_id TEXT); INSERT INTO read_report_result VALUES ('report-a');").unwrap();
    let before = std::fs::read(&path).unwrap();
    let query = ReportResultUseCase::new(ReadModelDaos::open(&path).unwrap().report_result());
    assert!(query.execute("report-a").is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn report_identity_is_a_bound_value_not_a_query_fragment() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("read.db");
    let db = Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE read_report_result(report_id TEXT, execution_id TEXT, stage TEXT, scope TEXT, result_kind TEXT, steps TEXT, no_op_reason TEXT, current_stage TEXT); INSERT INTO read_report_result VALUES ('report-a','execution','requirements-analysis','bugfix','committed','[\"approve\"]',NULL,NULL);").unwrap();
    let before = std::fs::read(&path).unwrap();
    let query = ReportResultUseCase::new(ReadModelDaos::open(&path).unwrap().report_result());
    assert!(query.execute("' OR 1=1 --").unwrap().is_none());
    assert!(query.execute("report-a").unwrap().is_some());
    assert_eq!(std::fs::read(&path).unwrap(), before);
}
