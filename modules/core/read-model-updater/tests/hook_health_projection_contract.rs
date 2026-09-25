//! HookHealthストリームだけをRMUが投影し、承認ストリームを触らない契約。
// テストコードでは unwrap / expect / panic を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::needless_pass_by_value,
    clippy::type_complexity
)]
use chrono::DateTime;
use core_command_domain::workspace::{HookHealthId, HookHealthTarget, HookName, SpaceName};
use core_read_model_updater::orchestration::{
    CorruptCause, HookHealthReadModelUpdater, JournalReadError, ReadModelUpdater,
};
use event_store_adapter_rs::{
    EventStoreForSqlite,
    event_envelope::EventEnvelope,
    types::{AggregateId, EventStore},
};
use rusqlite::params;
use serde_json::Value;
use std::fmt;
use tempfile::tempdir;

/// 同期のテストから共通契約 (`ReadModelUpdater::update_read_models`) を待つ補助。
///
/// 契約の境界は非同期だが、この投影器の内部は同期 I/O だけなので、current_thread
/// ランタイムでその場で待てば足りる。
trait UpdateNow {
    fn update_now(&mut self) -> Result<(), JournalReadError>;
}

impl UpdateNow for HookHealthReadModelUpdater {
    fn update_now(&mut self) -> Result<(), JournalReadError> {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("current_thread ランタイムを組める")
            .block_on(self.update_read_models())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
struct Key(String);
impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl AggregateId for Key {
    fn type_name(&self) -> String {
        "HookHealth".into()
    }
    fn value(&self) -> String {
        self.0.clone()
    }
}
#[tokio::test]
async fn rmu_projects_only_hook_health_events_after_interleaved_journal_rows() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("aidlc-runtime.sqlite");
    let target = HookHealthTarget::new(SpaceName::parse("default").unwrap(), None);
    let hook = HookName::parse("write-audit-log").unwrap();
    let key = Key(HookHealthId::for_hook(&target, &hook).to_string());
    let aid = key.to_string();
    let at: DateTime<chrono::Utc> = "2026-09-08T01:00:00Z".parse().unwrap();
    let mut store: EventStoreForSqlite<Key, serde_json::Value, serde_json::Value> =
        EventStoreForSqlite::new(&path).unwrap();
    let started: Value = serde_json::from_str(&format!(r#"{{"id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000","aggregate_id":"{aid}","kind":"started","target":"spaces/default/intents","hook":"write-audit-log","reason":null}}"#)).unwrap();
    let envelope =
        EventEnvelope::new(key.clone(), 1, at, started).with_manifest("hook-health-event/1");
    store
        .persist_event_and_snapshot(envelope, serde_json::from_str("{}").unwrap(), 0)
        .await
        .unwrap();
    drop(store);
    let db = rusqlite::Connection::open(&path).unwrap();
    for (seq, kind, payload) in [
        (
            2,
            "heartbeat",
            serde_json::from_str::<Value>(&format!(r#"{{"id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0001","aggregate_id":"{aid}","kind":"heartbeat","target":null,"hook":null,"reason":null}}"#)).unwrap(),
        ),
        (
            3,
            "dropped",
            serde_json::from_str::<Value>(&format!(r#"{{"id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0002","aggregate_id":"{aid}","kind":"dropped","target":null,"hook":null,"reason":"EISDIR write failed"}}"#)).unwrap(),
        ),
    ] {
        db.execute("INSERT INTO journal(pkey,skey,aid,seq_nr,payload,occurred_at,manifest) VALUES('hook-test',?1,?2,?3,?4,?5,'hook-health-event/1')",params![format!("manual-{seq}"),aid,seq,payload.to_string().into_bytes(),at.timestamp_nanos_opt().unwrap()+seq as i64]).unwrap();
        let _ = kind;
    }
    db.execute(
        "INSERT INTO journal(pkey,skey,aid,seq_nr,payload,occurred_at,manifest) VALUES('plan-test','plan-1','workspace',1,?1,?2,'plan-approval-event/1')",
        params![b"{\"unrelated\":true}".to_vec(), at.timestamp_nanos_opt().unwrap()],
    ).unwrap();
    drop(db);
    let mut updater = HookHealthReadModelUpdater::open(&path).unwrap();
    updater.update_read_models().await.unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    let row: (String, i64, i64) = db
        .query_row(
            "SELECT target,seq_nr,drops FROM read_hook_health WHERE id=?1",
            [aid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(row, ("spaces/default/intents".into(), 3, 1));
    assert_eq!(
        db.query_row::<i64, _, _>(
            "SELECT count(*) FROM journal WHERE manifest='hook-health-event/1'",
            [],
            |r| r.get(0)
        )
        .unwrap(),
        3
    );
    assert_eq!(
        db.query_row::<i64, _, _>("SELECT count(*) FROM journal", [], |r| r.get(0))
            .unwrap(),
        4
    );
}

/// 空の共有DB（本家スキーマ）を作り、指定した行を `journal` 表へ直接書く。
fn journal_with_rows(dir: &std::path::Path, rows: &[(i64, &str)]) -> std::path::PathBuf {
    let path = dir.join("aidlc-runtime.sqlite");
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE journal(id INTEGER PRIMARY KEY AUTOINCREMENT,pkey TEXT NOT NULL,skey TEXT NOT NULL,aid TEXT NOT NULL,seq_nr INTEGER NOT NULL,payload BLOB NOT NULL,occurred_at INTEGER NOT NULL,manifest TEXT NOT NULL)").unwrap();
    let at: DateTime<chrono::Utc> = "2026-09-08T02:00:00Z".parse().unwrap();
    for (index, (seq, payload)) in rows.iter().enumerate() {
        let aid = serde_json::from_str::<Value>(payload)
            .ok()
            .and_then(|v| {
                v.get("aggregate_id")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "broken".to_string());
        db.execute("INSERT INTO journal(pkey,skey,aid,seq_nr,payload,occurred_at,manifest) VALUES('hook-test',?1,?2,?3,?4,?5,'hook-health-event/1')",params![format!("row-{index}"),aid,seq,payload.as_bytes().to_vec(),at.timestamp_nanos_opt().unwrap()+ *seq]).unwrap();
    }
    path
}
fn valid_aid() -> String {
    let target = HookHealthTarget::new(SpaceName::parse("default").unwrap(), None);
    HookHealthId::for_hook(&target, &HookName::parse("write-audit-log").unwrap()).to_string()
}
fn event(id_suffix: &str, aid: &str, kind: &str, target: &str, hook: &str, reason: &str) -> String {
    let field = |v: &str| {
        if v == "null" {
            "null".to_string()
        } else {
            format!("\"{v}\"")
        }
    };
    format!(
        r#"{{"id":"0190aaaa-bbbb-7ccc-9ddd-eeeeffff{id_suffix}","aggregate_id":"{aid}","kind":"{kind}","target":{},"hook":{},"reason":{}}}"#,
        field(target),
        field(hook),
        field(reason)
    )
}
fn cause_of(error: &JournalReadError) -> Option<(String, Option<usize>, CorruptCause)> {
    match error {
        JournalReadError::Corrupt {
            aggregate_id,
            seq_nr,
            cause,
        } => Some((aggregate_id.clone(), *seq_nr, *cause)),
        _ => None,
    }
}
#[test]
fn a_journal_whose_first_row_is_a_first_drop_projects_a_drop_without_heartbeat() {
    let dir = tempdir().unwrap();
    let aid = valid_aid();
    let path = journal_with_rows(
        dir.path(),
        &[
            (
                1,
                &event(
                    "0010",
                    &aid,
                    "first-drop",
                    "spaces/default/intents",
                    "write-audit-log",
                    "ENOSPC",
                ),
            ),
            (
                2,
                &event("0011", &aid, "dropped", "null", "null", "EISDIR again"),
            ),
        ],
    );
    let mut updater = HookHealthReadModelUpdater::open(&path).unwrap();
    updater.update_now().unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    let row: (Option<String>, i64, i64, Option<String>) = db
        .query_row(
            "SELECT heartbeat,seq_nr,drops,latest_drop FROM read_hook_health WHERE id=?1",
            [&aid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(row, (None, 2, 2, Some("EISDIR again".into())));
    let health_dir = dir
        .path()
        .join("spaces/default/intents/.aidlc-hooks-health");
    assert!(
        !health_dir.join("write-audit-log.last").exists(),
        "heartbeat がないので .last は書かない"
    );
    let drops = std::fs::read_to_string(health_dir.join("write-audit-log.drops")).unwrap();
    assert_eq!(
        drops,
        "2026-09-08T02:00:00Z\tENOSPC\n2026-09-08T02:00:00Z\tEISDIR again\n"
    );
    // 再投影は drop 履歴を重複追記せず、利用者が切り詰めた分だけを補う。
    updater.update_now().unwrap();
    assert_eq!(
        std::fs::read_to_string(health_dir.join("write-audit-log.drops")).unwrap(),
        drops
    );
    std::fs::write(
        health_dir.join("write-audit-log.drops"),
        "2026-09-08T02:00:00Z\tENOSPC\n",
    )
    .unwrap();
    updater.update_now().unwrap();
    assert_eq!(
        std::fs::read_to_string(health_dir.join("write-audit-log.drops")).unwrap(),
        drops
    );
}
#[test]
fn a_read_table_that_requires_heartbeat_is_rebuilt_before_projecting() {
    let dir = tempdir().unwrap();
    let aid = valid_aid();
    let path = journal_with_rows(
        dir.path(),
        &[(
            1,
            &event(
                "0020",
                &aid,
                "first-drop",
                "spaces/default/intents",
                "write-audit-log",
                "ENOSPC",
            ),
        )],
    );
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE read_hook_health(id TEXT PRIMARY KEY,target TEXT NOT NULL,hook TEXT NOT NULL,heartbeat TEXT NOT NULL,seq_nr INTEGER NOT NULL,drops INTEGER NOT NULL,latest_drop TEXT)").unwrap();
    drop(db);
    HookHealthReadModelUpdater::open(&path)
        .unwrap()
        .update_now()
        .unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    let notnull: i64 = db
        .query_row(
            "SELECT \"notnull\" FROM pragma_table_info('read_hook_health') WHERE name='heartbeat'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(notnull, 0, "heartbeat が NULL を許す形へ作り直される");
    let heartbeat: Option<String> = db
        .query_row(
            "SELECT heartbeat FROM read_hook_health WHERE id=?1",
            [&aid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(heartbeat, None);
    let checkpoint: i64 = db
        .query_row(
            "SELECT last_seq FROM hook_health_projection_checkpoint WHERE projection='hook-health'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(checkpoint, 1);
}
#[test]
fn opening_a_missing_shared_db_is_refused_instead_of_creating_one() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("absent.sqlite");
    let error = HookHealthReadModelUpdater::open(&path).unwrap_err();
    assert_eq!(
        cause_of(&error),
        Some(("hook-health".into(), None, CorruptCause::InvariantViolation))
    );
    assert!(!path.exists());
}
#[test]
fn corrupt_hook_health_rows_are_refused_with_their_cause() {
    let aid = valid_aid();
    let ok_target = "spaces/default/intents";
    let cases: Vec<(
        &str,
        Vec<(i64, String)>,
        (String, Option<usize>, CorruptCause),
    )> = vec![
        (
            "payload が JSON でない",
            vec![(1, "{not json".to_string())],
            ("broken".into(), None, CorruptCause::UndecodablePayload),
        ),
        (
            "イベント ID が UUIDv7 でない",
            vec![(
                1,
                event(
                    "zzzz",
                    &aid,
                    "started",
                    ok_target,
                    "write-audit-log",
                    "null",
                ),
            )],
            (aid.clone(), None, CorruptCause::UndecodablePayload),
        ),
        (
            "集約 ID が hook-health: 形式でない",
            vec![(
                1,
                event(
                    "0030",
                    "hook-health:short",
                    "started",
                    ok_target,
                    "write-audit-log",
                    "null",
                ),
            )],
            (
                "hook-health:short".into(),
                None,
                CorruptCause::UndecodablePayload,
            ),
        ),
        (
            "未知の kind",
            vec![(
                1,
                event("0031", &aid, "paused", ok_target, "write-audit-log", "null"),
            )],
            ("hook-health".into(), None, CorruptCause::UndecodablePayload),
        ),
        (
            "started に target がない",
            vec![(
                1,
                event("0032", &aid, "started", "null", "write-audit-log", "null"),
            )],
            ("hook-health".into(), None, CorruptCause::UndecodablePayload),
        ),
        (
            "started の target が spaces/ で始まらない",
            vec![(
                1,
                event(
                    "0033",
                    &aid,
                    "started",
                    "records/default",
                    "write-audit-log",
                    "null",
                ),
            )],
            ("hook-health".into(), None, CorruptCause::UndecodablePayload),
        ),
        (
            "started に hook がない",
            vec![(1, event("0034", &aid, "started", ok_target, "null", "null"))],
            ("hook-health".into(), None, CorruptCause::UndecodablePayload),
        ),
        (
            "started の hook 名が小文字ケバブでない",
            vec![(
                1,
                event("0035", &aid, "started", ok_target, "Write_Audit", "null"),
            )],
            ("hook-health".into(), None, CorruptCause::UndecodablePayload),
        ),
        (
            "first-drop に target がない",
            vec![(
                1,
                event(
                    "0036",
                    &aid,
                    "first-drop",
                    "null",
                    "write-audit-log",
                    "ENOSPC",
                ),
            )],
            ("hook-health".into(), None, CorruptCause::UndecodablePayload),
        ),
        (
            "first-drop の target が不正",
            vec![(
                1,
                event(
                    "0037",
                    &aid,
                    "first-drop",
                    "spaces/default/other",
                    "write-audit-log",
                    "ENOSPC",
                ),
            )],
            ("hook-health".into(), None, CorruptCause::UndecodablePayload),
        ),
        (
            "first-drop に hook がない",
            vec![(
                1,
                event("0038", &aid, "first-drop", ok_target, "null", "ENOSPC"),
            )],
            ("hook-health".into(), None, CorruptCause::UndecodablePayload),
        ),
        (
            "first-drop の hook 名が不正",
            vec![(
                1,
                event("0039", &aid, "first-drop", ok_target, "9bad", "ENOSPC"),
            )],
            ("hook-health".into(), None, CorruptCause::UndecodablePayload),
        ),
        (
            "first-drop に reason がない",
            vec![(
                1,
                event(
                    "003a",
                    &aid,
                    "first-drop",
                    ok_target,
                    "write-audit-log",
                    "null",
                ),
            )],
            ("hook-health".into(), None, CorruptCause::UndecodablePayload),
        ),
        (
            "first-drop の reason が空白だけ",
            vec![(
                1,
                event(
                    "003b",
                    &aid,
                    "first-drop",
                    ok_target,
                    "write-audit-log",
                    "   ",
                ),
            )],
            ("hook-health".into(), None, CorruptCause::UndecodablePayload),
        ),
        (
            "dropped に reason がない",
            vec![
                (
                    1,
                    event(
                        "003c",
                        &aid,
                        "started",
                        ok_target,
                        "write-audit-log",
                        "null",
                    ),
                ),
                (2, event("003d", &aid, "dropped", "null", "null", "null")),
            ],
            ("hook-health".into(), None, CorruptCause::UndecodablePayload),
        ),
        (
            "先頭が heartbeat（開始でも初回 drop でもない）",
            vec![(1, event("003e", &aid, "heartbeat", "null", "null", "null"))],
            (aid.clone(), Some(1), CorruptCause::InvariantViolation),
        ),
        (
            "履歴が seq 1 から始まらない",
            vec![(
                2,
                event(
                    "003f",
                    &aid,
                    "started",
                    ok_target,
                    "write-audit-log",
                    "null",
                ),
            )],
            (aid.clone(), Some(2), CorruptCause::InvariantViolation),
        ),
        (
            "seq_nr が負",
            vec![(
                -1,
                event(
                    "0040",
                    &aid,
                    "started",
                    ok_target,
                    "write-audit-log",
                    "null",
                ),
            )],
            ("hook-health".into(), None, CorruptCause::InvariantViolation),
        ),
    ];
    for (label, rows, expected) in cases {
        let dir = tempdir().unwrap();
        let borrowed: Vec<(i64, &str)> = rows.iter().map(|(s, p)| (*s, p.as_str())).collect();
        let path = journal_with_rows(dir.path(), &borrowed);
        let error = HookHealthReadModelUpdater::open(&path)
            .unwrap()
            .update_now()
            .unwrap_err();
        assert_eq!(cause_of(&error), Some(expected), "{label}");
        let db = rusqlite::Connection::open(&path).unwrap();
        let projected: i64 = db
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='read_hook_health'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(projected, 0, "{label}: 拒否した履歴から読取表を作らない");
    }
}
#[test]
fn a_row_whose_aggregate_column_disagrees_with_its_payload_is_refused() {
    let dir = tempdir().unwrap();
    let aid = valid_aid();
    let path = dir.path().join("aidlc-runtime.sqlite");
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE journal(id INTEGER PRIMARY KEY AUTOINCREMENT,pkey TEXT NOT NULL,skey TEXT NOT NULL,aid TEXT NOT NULL,seq_nr INTEGER NOT NULL,payload BLOB NOT NULL,occurred_at INTEGER NOT NULL,manifest TEXT NOT NULL)").unwrap();
    let other = HookHealthId::for_hook(
        &HookHealthTarget::new(SpaceName::parse("default").unwrap(), None),
        &HookName::parse("session-start").unwrap(),
    )
    .to_string();
    db.execute(
        "INSERT INTO journal(pkey,skey,aid,seq_nr,payload,occurred_at,manifest) VALUES('hook-test','row-0',?1,1,?2,0,'hook-health-event/1')",
        params![other, event("0050", &aid, "started", "spaces/default/intents", "write-audit-log", "null").into_bytes()],
    )
    .unwrap();
    drop(db);
    let error = HookHealthReadModelUpdater::open(&path)
        .unwrap()
        .update_now()
        .unwrap_err();
    assert_eq!(
        cause_of(&error),
        Some((other, Some(1), CorruptCause::InvariantViolation))
    );
}
#[test]
fn a_row_whose_columns_cannot_be_read_is_refused_as_undecodable() {
    let dir = tempdir().unwrap();
    let aid = valid_aid();
    let path = journal_with_rows(dir.path(), &[]);
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute(
        "INSERT INTO journal(pkey,skey,aid,seq_nr,payload,occurred_at,manifest) VALUES('hook-test','row-0',?1,'one',?2,0,'hook-health-event/1')",
        params![aid, event("0060", &aid, "started", "spaces/default/intents", "write-audit-log", "null").into_bytes()],
    )
    .unwrap();
    drop(db);
    let error = HookHealthReadModelUpdater::open(&path)
        .unwrap()
        .update_now()
        .unwrap_err();
    assert_eq!(
        cause_of(&error),
        Some(("hook-health".into(), None, CorruptCause::UndecodablePayload))
    );
}
#[test]
fn a_drop_history_that_cannot_be_appended_is_refused_after_the_rows_are_projected() {
    let dir = tempdir().unwrap();
    let aid = valid_aid();
    let path = journal_with_rows(
        dir.path(),
        &[(
            1,
            &event(
                "0070",
                &aid,
                "first-drop",
                "spaces/default/intents",
                "write-audit-log",
                "ENOSPC",
            ),
        )],
    );
    let health_dir = dir
        .path()
        .join("spaces/default/intents/.aidlc-hooks-health");
    std::fs::create_dir_all(health_dir.join("write-audit-log.drops")).unwrap();
    let error = HookHealthReadModelUpdater::open(&path)
        .unwrap()
        .update_now()
        .unwrap_err();
    assert_eq!(
        cause_of(&error),
        Some((aid.clone(), None, CorruptCause::InvariantViolation))
    );
    let db = rusqlite::Connection::open(&path).unwrap();
    let drops: i64 = db
        .query_row(
            "SELECT drops FROM read_hook_health WHERE id=?1",
            [&aid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        drops, 1,
        "読取表は確定済みで、公開ファイルの失敗だけが報告される"
    );
}

/// 本家の `journal` 表を持たない共有 DB は、読取表を作らずに拒否される。
#[test]
fn a_shared_db_without_the_journal_table_is_refused_before_any_read_table_is_created() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("aidlc-runtime.sqlite");
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute_batch("CREATE TABLE unrelated(x INTEGER)")
        .unwrap();
    let error = HookHealthReadModelUpdater::open(&path)
        .unwrap()
        .update_now()
        .unwrap_err();
    assert_eq!(
        cause_of(&error),
        Some(("hook-health".into(), None, CorruptCause::InvariantViolation))
    );
    let db = rusqlite::Connection::open(&path).unwrap();
    let tables: i64 = db
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name IN ('read_hook_health','hook_health_projection_checkpoint')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        tables, 0,
        "拒否した投影は読取表もチェックポイント表も作らない"
    );
}

/// 書けない共有 DB（OS が書込を禁じたファイル）では、読取表の作り直し・作成・全消去の
/// どれも失敗として上がり、成功に丸めない。
#[cfg(unix)]
#[test]
fn a_read_only_shared_db_refuses_every_write_step_it_reaches() {
    use std::os::unix::fs::PermissionsExt as _;
    let aid = valid_aid();
    let row = event(
        "0080",
        &aid,
        "started",
        "spaces/default/intents",
        "write-audit-log",
        "null",
    );
    // (読取表の事前状態, 期待される失敗段)
    for (prepare, step) in [
        (None, "CREATE TABLE IF NOT EXISTS"),
        (
            Some(
                "CREATE TABLE read_hook_health(id TEXT PRIMARY KEY,target TEXT NOT NULL,hook TEXT NOT NULL,heartbeat TEXT,seq_nr INTEGER NOT NULL,drops INTEGER NOT NULL,latest_drop TEXT)",
            ),
            "DELETE",
        ),
        (
            Some(
                "CREATE TABLE read_hook_health(id TEXT PRIMARY KEY,target TEXT NOT NULL,hook TEXT NOT NULL,heartbeat TEXT NOT NULL,seq_nr INTEGER NOT NULL,drops INTEGER NOT NULL,latest_drop TEXT)",
            ),
            "DROP TABLE (heartbeat NOT NULL の旧表)",
        ),
    ] {
        let dir = tempdir().unwrap();
        let path = journal_with_rows(dir.path(), &[(1, &row)]);
        if let Some(ddl) = prepare {
            rusqlite::Connection::open(&path)
                .unwrap()
                .execute_batch(ddl)
                .unwrap();
        }
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444)).unwrap();
        let error = HookHealthReadModelUpdater::open(&path)
            .unwrap()
            .update_now()
            .unwrap_err();
        assert_eq!(
            cause_of(&error),
            Some(("hook-health".into(), None, CorruptCause::InvariantViolation)),
            "{step}"
        );
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        let db = rusqlite::Connection::open(&path).unwrap();
        let checkpoints: i64 = db
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='hook_health_projection_checkpoint'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(checkpoints, 0, "{step}: チェックポイントは進まない");
        let heartbeat = dir
            .path()
            .join("spaces/default/intents/.aidlc-hooks-health/write-audit-log.last");
        assert!(!heartbeat.exists(), "{step}: 公開ファイルも書かない");
    }
}

/// 形の違う読取表・チェックポイント表に当たった投影は、同じ Tx ごと巻き戻る —
/// 「行だけ新しくチェックポイントは古い」断面を残さない。
#[test]
fn a_drifted_read_table_or_checkpoint_table_rolls_the_projection_back() {
    let aid = valid_aid();
    let row = event(
        "0090",
        &aid,
        "started",
        "spaces/default/intents",
        "write-audit-log",
        "null",
    );
    for (ddl, step) in [
        (
            "CREATE TABLE read_hook_health(id TEXT PRIMARY KEY,target TEXT NOT NULL,hook TEXT NOT NULL,heartbeat TEXT,seq_nr INTEGER NOT NULL,drops INTEGER NOT NULL)",
            "列の足りない読取表への INSERT",
        ),
        (
            "CREATE TABLE hook_health_projection_checkpoint(projection TEXT,last_seq INTEGER NOT NULL)",
            "主キーの無いチェックポイント表への UPSERT",
        ),
        (
            "CREATE VIEW read_hook_health AS SELECT 'x' AS id,'t' AS target,'h' AS hook,NULL AS heartbeat,0 AS seq_nr,0 AS drops,NULL AS latest_drop",
            "読取表の名前を持つビューへの DELETE",
        ),
    ] {
        let dir = tempdir().unwrap();
        let path = journal_with_rows(dir.path(), &[(1, &row)]);
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(ddl)
            .unwrap();
        let error = HookHealthReadModelUpdater::open(&path)
            .unwrap()
            .update_now()
            .unwrap_err();
        assert_eq!(
            cause_of(&error),
            Some(("hook-health".into(), None, CorruptCause::InvariantViolation)),
            "{step}"
        );
        let db = rusqlite::Connection::open(&path).unwrap();
        let rows: i64 = db
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='read_hook_health'",
                [],
                |r| r.get(0),
            )
            .and_then(|present: i64| {
                if present == 0 {
                    Ok(0)
                } else {
                    db.query_row("SELECT count(*) FROM read_hook_health", [], |r| r.get(0))
                }
            })
            .unwrap();
        assert_eq!(rows, 0, "{step}: 行は 1 つも確定しない");
        let checkpoint: Option<i64> = db
            .query_row(
                "SELECT last_seq FROM hook_health_projection_checkpoint WHERE projection='hook-health'",
                [],
                |r| r.get(0),
            )
            .ok();
        assert_eq!(checkpoint, None, "{step}: チェックポイントは進まない");
    }
}

/// 公開先が作れない・書けないときは、読取表を確定させたうえで公開の失敗だけを報告する。
#[test]
fn an_unwritable_publication_target_is_reported_after_the_rows_are_projected() {
    let aid = valid_aid();
    let row = event(
        "00a0",
        &aid,
        "started",
        "spaces/default/intents",
        "write-audit-log",
        "null",
    );
    // 対象ディレクトリの位置にファイルがある → `.aidlc-hooks-health` を作れない。
    {
        let dir = tempdir().unwrap();
        let path = journal_with_rows(dir.path(), &[(1, &row)]);
        std::fs::create_dir_all(dir.path().join("spaces/default")).unwrap();
        std::fs::write(
            dir.path().join("spaces/default/intents"),
            b"not a directory",
        )
        .unwrap();
        let error = HookHealthReadModelUpdater::open(&path)
            .unwrap()
            .update_now()
            .unwrap_err();
        assert_eq!(
            cause_of(&error),
            Some((aid.clone(), None, CorruptCause::InvariantViolation))
        );
        let db = rusqlite::Connection::open(&path).unwrap();
        let seq: i64 = db
            .query_row(
                "SELECT seq_nr FROM read_hook_health WHERE id=?1",
                [&aid],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(seq, 1, "読取表は確定済み");
    }
    // heartbeat の公開ファイルの位置にディレクトリがある → 書けない (I/O の失敗をパス付きで)。
    {
        let dir = tempdir().unwrap();
        let path = journal_with_rows(dir.path(), &[(1, &row)]);
        let heartbeat = dir
            .path()
            .join("spaces/default/intents/.aidlc-hooks-health/write-audit-log.last");
        std::fs::create_dir_all(&heartbeat).unwrap();
        let error = HookHealthReadModelUpdater::open(&path)
            .unwrap()
            .update_now()
            .unwrap_err();
        match error {
            JournalReadError::Io { kind, path: at } => {
                assert_eq!(at, Some(heartbeat));
                assert_ne!(kind, std::io::ErrorKind::NotFound);
            }
            other => panic!("Io を期待した: {other:?}"),
        }
    }
}
