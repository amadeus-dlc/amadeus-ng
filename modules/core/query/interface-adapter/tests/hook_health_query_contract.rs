//! HookHealth投影をIDで読むQueryの契約。
#![allow(clippy::unwrap_used)]
use core_query_interface_adapter::ReadModelDaos;
use core_query_use_case::orchestration::HookHealthUseCase;
#[test]
fn query_reads_only_the_requested_projected_health_row() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("aidlc-runtime.sqlite");
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE read_hook_health(id TEXT PRIMARY KEY,target TEXT NOT NULL,hook TEXT NOT NULL,heartbeat TEXT NOT NULL,seq_nr INTEGER NOT NULL,drops INTEGER NOT NULL,latest_drop TEXT);")
        .unwrap();
    db.execute(
        "INSERT INTO read_hook_health VALUES(?1,?2,?3,?4,?5,?6,?7)",
        rusqlite::params![
            "hook-health:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "spaces/default/intents",
            "write-audit-log",
            "2026-09-08T01:00:00Z",
            3_i64,
            1_i64,
            "EISDIR write failed"
        ],
    )
    .unwrap();
    drop(db);
    let daos = ReadModelDaos::open(&path).unwrap();
    let query = HookHealthUseCase::new(daos.hook_health());
    let row = query
        .execute("hook-health:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        .unwrap()
        .unwrap();
    assert_eq!(row.target(), "spaces/default/intents");
    assert_eq!(row.hook(), "write-audit-log");
    assert_eq!(row.seq_nr(), 3);
    assert_eq!(row.drops(), 1);
    assert_eq!(row.latest_drop(), Some("EISDIR write failed"));
    assert!(
        query
            .execute("hook-health:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
            .unwrap()
            .is_none()
    );
}
