//! 自己診断の 2 DAO の契約 — **1 表を鍵で引き、当たらなければ空**。
//!
//! 行は RMU 本体 (`WorkspaceDoctorReadModelUpdater`) が書いたものである。期待値をテストへ
//! 書き下さず、投影が実際に書いた行を引けることを見る (既存の `read_model_dao_contract` と
//! 同じ立て付け)。ジャーナル行は本家ストアの形 (`journal` 表) に合わせて用意する。
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]
// ジャーナル payload は契約 JSON ではなく、永続化 DTO のワイヤ形式である。
#![allow(clippy::disallowed_methods)]

use std::io::ErrorKind;
use std::path::Path;

use core_query_interface_adapter::ReadModelDaos;
use core_query_use_case::orchestration::{DoctorCheckDao, DoctorReportDao};
use core_read_model_updater::orchestration::{ReadModelUpdater, WorkspaceDoctorReadModelUpdater};

const MANIFEST: &str = "workspace-doctor-event/1";
const TARGET: &str = "spaces/default/intents";
/// `WorkspaceDoctorId::for_target` が導く綴り (namespace + sha256 16 進 64 桁)。
const REPORT_ID: &str =
    "workspace-doctor:fd9fe7c1c349372f408ee6fd4060f95b4c7b3e4c814727bf3eff713c34922ea0";
const EVENT_ID: &str = "0199aaaa-bbbb-7ccc-8ddd-eeeeffff0001";

/// 本家 `journal` 表の最小の殻 (投影が読む列だけ)。
const JOURNAL_SHELL: &str = "CREATE TABLE IF NOT EXISTS journal (
  pkey TEXT NOT NULL, skey TEXT NOT NULL, aid TEXT NOT NULL, seq_nr INTEGER NOT NULL,
  payload BLOB NOT NULL, occurred_at INTEGER NOT NULL, manifest TEXT NOT NULL DEFAULT '',
  PRIMARY KEY (pkey, skey))";

fn payload() -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "id": EVENT_ID,
        "aggregate_id": REPORT_ID,
        "target": TARGET,
        "checks": [
            {"check_id": "D1.a", "passed": true, "label": "bun installed", "fix": null},
            {"check_id": "D2.f", "passed": false, "label": "Hooks have never executed",
             "fix": "1. Run /hooks"},
            {"check_id": "D3.a", "passed": true, "label": "workspace shell ready", "fix": null},
        ],
    }))
    .unwrap()
}

/// 診断事実を 1 件持つストアを作り、RMU に `read_doctor_*` を書かせる。
fn projected(store: &Path) {
    let connection = rusqlite::Connection::open(store).unwrap();
    connection.execute_batch(JOURNAL_SHELL).unwrap();
    connection
        .execute(
            "INSERT INTO journal(pkey,skey,aid,seq_nr,payload,occurred_at,manifest)
             VALUES ('p','s',?1,1,?2,0,?3)",
            rusqlite::params![REPORT_ID, payload(), MANIFEST],
        )
        .unwrap();
    drop(connection);
    // 共通契約の境界は非同期だが、この投影器の内部は同期 I/O だけなので、同期の
    // フィクスチャからは current_thread ランタイムでその場で待つ。
    let mut updater = WorkspaceDoctorReadModelUpdater::open(store).unwrap();
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(updater.update_read_models())
        .unwrap();
}

struct Fixture {
    _dir: tempfile::TempDir,
    daos: ReadModelDaos,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store.sqlite");
    projected(&store);
    Fixture {
        _dir: dir,
        daos: ReadModelDaos::open(&store).unwrap(),
    }
}

#[test]
fn the_summary_row_is_pulled_by_the_diagnosed_target() {
    let fixture = fixture();
    let summary = fixture.daos.doctor_report().find(TARGET).unwrap().unwrap();
    assert_eq!(summary.report_id(), REPORT_ID);
    assert_eq!(summary.passed(), 2);
    assert_eq!(summary.failed(), 1);
    assert_eq!(summary.exit_code(), 1);
}

#[test]
fn a_target_that_was_never_diagnosed_yields_no_row() {
    let fixture = fixture();
    assert!(
        fixture
            .daos
            .doctor_report()
            .find("spaces/other/intents")
            .unwrap()
            .is_none()
    );
}

#[test]
fn the_check_rows_come_back_in_the_stored_display_order() {
    let fixture = fixture();
    let checks = fixture.daos.doctor_check().find(REPORT_ID).unwrap();
    assert_eq!(
        checks
            .iter()
            .map(|check| check.check_id().to_string())
            .collect::<Vec<_>>(),
        vec!["D1.a", "D2.f", "D3.a"]
    );
    assert_eq!(checks[1].label(), "Hooks have never executed");
    assert_eq!(checks[1].fix(), Some("1. Run /hooks"));
    assert!(!checks[1].is_passed());
    assert_eq!(checks[0].fix(), None);
    assert!(checks[0].is_passed());
}

#[test]
fn an_unknown_report_has_no_check_rows_rather_than_an_error() {
    let fixture = fixture();
    assert!(
        fixture
            .daos
            .doctor_check()
            .find(
                "workspace-doctor:0000000000000000000000000000000000000000000000000000000000000000"
            )
            .unwrap()
            .is_empty()
    );
}

#[test]
fn a_store_that_cannot_be_opened_is_a_read_failure_not_an_absence() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nope.sqlite");
    let error = ReadModelDaos::open(&missing).unwrap_err();
    assert!(
        matches!(error.kind(), ErrorKind::NotFound | ErrorKind::Other),
        "実際: {error:?}"
    );
}

#[test]
fn an_ephemeral_shared_memory_store_is_read_through_the_same_daos() {
    // C7 DC1 — 初回状態はファイルを 1 つも作らない。同じ DAO が一時ストアも読む。
    let uri = format!(
        "file:aidlc-doctor-dao-{}?mode=memory&cache=shared",
        std::process::id()
    );
    let keep_alive = rusqlite::Connection::open(&uri).unwrap();
    projected(Path::new(&uri));
    let daos = ReadModelDaos::open(Path::new(&uri)).unwrap();
    let summary = daos.doctor_report().find(TARGET).unwrap().unwrap();
    assert_eq!(summary.failed(), 1);
    assert_eq!(
        daos.doctor_check().find(summary.report_id()).unwrap().len(),
        3
    );
    assert!(!Path::new(&uri).exists(), "実ファイルを作らない");
    drop(keep_alive);
}
