//! 停止制御（workflow-continuation）ストリームを RMU が読み、進捗回数の読取表と
//! `.aidlc-stop-hook/block-count.json` へ投影する契約。壊れた履歴は拒否する。
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
// ジャーナル行の payload は**契約 JSON ではなくワイヤ形式そのもの**であり、行を用意する
// テストは本家のシリアライザと同じ素の serde で書く (BR1.7 の射程外)。
#![allow(clippy::disallowed_methods)]
use core_command_domain::orchestration::{IntentExecutionId, WorkflowContinuationId};
use core_read_model_updater::orchestration::{
    JournalReadError, ReadModelUpdater, WorkflowContinuationReadModelUpdater,
};
use rusqlite::{Connection, params};
use serde_json::{Value, json};

/// 共通契約の境界は非同期なので、同期のテストは current_thread ランタイムで待つ
/// (この投影器の内部は同期 I/O だけである)。
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("current_thread ランタイムを組める")
        .block_on(future)
}

const EXECUTION: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";
const OTHER_EXECUTION: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0001";

struct Fixture {
    root: tempfile::TempDir,
    path: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("aidlc-runtime.sqlite");
        Connection::open(&path)
            .unwrap()
            .execute_batch("CREATE TABLE journal(id INTEGER PRIMARY KEY AUTOINCREMENT,pkey TEXT NOT NULL,skey TEXT NOT NULL,aid TEXT NOT NULL,seq_nr INTEGER NOT NULL,payload BLOB NOT NULL,occurred_at INTEGER NOT NULL,manifest TEXT NOT NULL)")
            .unwrap();
        Self { root, path }
    }
    fn id(&self) -> WorkflowContinuationId {
        let _ = &self.root;
        WorkflowContinuationId::for_execution(&IntentExecutionId::parse(EXECUTION).unwrap())
    }
    fn record(&self) -> std::path::PathBuf {
        let record = self.root.path().join("record");
        std::fs::create_dir_all(&record).unwrap();
        record
    }
    fn row(&self, seq: i64, payload: &[u8]) {
        Connection::open(&self.path).unwrap().execute(
            "INSERT INTO journal(pkey,skey,aid,seq_nr,payload,occurred_at,manifest) VALUES('cont',?1,?2,?3,?4,?5,'workflow-continuation-event/1')",
            params![format!("row-{seq}"), self.id().as_str(), seq, payload.to_vec(), 1_757_300_000_000_000_000_i64 + seq],
        ).unwrap();
    }
    fn project(&self) -> Result<(), JournalReadError> {
        let mut updater =
            WorkflowContinuationReadModelUpdater::open(&self.path, self.id(), self.record())?;
        block_on(updater.update_read_models())
    }
    fn result_rows(&self) -> Vec<(String, i64, bool, Option<bool>, bool)> {
        let db = Connection::open(&self.path).unwrap();
        let mut st = db
            .prepare("SELECT id,count,blocked,counter_published,settled FROM read_continuation_result ORDER BY seq_nr")
            .unwrap();
        st.query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect()
    }
}
fn signature() -> String {
    format!("code-generation::{}::{}", "a".repeat(64), "b".repeat(64))
}
fn attempt(index: u16) -> String {
    format!("0190bbbb-cccc-7ddd-9eee-ffff0000{index:04x}")
}
fn event(index: u16, request: Value, count: u64, blocked: bool) -> Value {
    json!({
        "id": format!("0190aaaa-bbbb-7ccc-9ddd-eeeeffff1{index:03x}"),
        "aggregate_id": format!("workflow-continuation:{EXECUTION}"),
        "request": request,
        "count": count,
        "blocked": blocked,
        "publication": null
    })
}
fn request(index: u16, observed_guard: Value) -> Value {
    json!({"id": attempt(index), "signature": signature(), "reentrant": false, "limit": 3, "wait": null, "probe_only": false, "observed_guard": observed_guard})
}
const fn invalid_data(error: &JournalReadError) -> bool {
    matches!(
        error,
        JournalReadError::Io {
            kind: std::io::ErrorKind::InvalidData,
            ..
        }
    )
}

#[test]
fn a_blocked_attempt_is_projected_with_its_published_counter() {
    let fixture = Fixture::new();
    fixture.row(
        1,
        event(1, request(1, Value::Null), 1, true)
            .to_string()
            .as_bytes(),
    );
    fixture.project().unwrap();
    assert_eq!(
        fixture.result_rows(),
        vec![(attempt(1), 1, true, Some(true), false)]
    );
    let counter =
        std::fs::read_to_string(fixture.record().join(".aidlc-stop-hook/block-count.json"))
            .unwrap();
    assert_eq!(
        counter,
        format!("{{\"signature\":\"{}\",\"count\":1}}", signature())
    );
    let checkpoint: i64 = Connection::open(&fixture.path)
        .unwrap()
        .query_row(
            "SELECT seq_nr FROM continuation_projection_checkpoint WHERE id=?1",
            [fixture.id().as_str()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(checkpoint, 1);
}

#[test]
fn an_empty_stream_projects_nothing() {
    let fixture = Fixture::new();
    fixture.project().unwrap();
    assert!(fixture.result_rows().is_empty());
    assert!(!fixture.record().join(".aidlc-stop-hook").exists());
}

#[test]
fn corrupt_continuation_rows_are_refused_as_invalid_data() {
    let cases: Vec<(&str, Vec<(i64, Vec<u8>)>)> = vec![
        ("payload が JSON でない", vec![(1, b"{".to_vec())]),
        (
            "別の停止制御の事実が混ざる",
            vec![(1, {
                let mut row = event(1, request(1, Value::Null), 1, true);
                row["aggregate_id"] = format!("workflow-continuation:{OTHER_EXECUTION}").into();
                row.to_string().into_bytes()
            })],
        ),
        (
            "履歴が通番 1 から始まらない",
            vec![(
                2,
                event(1, request(1, Value::Null), 1, true)
                    .to_string()
                    .into_bytes(),
            )],
        ),
        (
            "要求 ID が UUIDv7 でない",
            vec![(1, {
                let mut row = event(1, request(1, Value::Null), 1, true);
                row["request"]["id"] = "x".into();
                row.to_string().into_bytes()
            })],
        ),
        (
            "観測した guard が署名なしで回数を持つ",
            vec![(
                1,
                event(
                    1,
                    request(
                        1,
                        json!({"signature": null, "count": 2, "initialized": true}),
                    ),
                    3,
                    false,
                )
                .to_string()
                .into_bytes(),
            )],
        ),
        (
            "事実の回数が guard から導いた回数と一致しない",
            vec![(
                1,
                event(1, request(1, Value::Null), 2, true)
                    .to_string()
                    .into_bytes(),
            )],
        ),
        (
            "wait の綴りが未知",
            vec![(1, {
                let mut row = event(1, request(1, Value::Null), 1, true);
                row["request"]["wait"] = "forever".into();
                row.to_string().into_bytes()
            })],
        ),
        (
            "通番が負",
            vec![(
                -1,
                event(1, request(1, Value::Null), 1, true)
                    .to_string()
                    .into_bytes(),
            )],
        ),
    ];
    for (label, rows) in cases {
        let fixture = Fixture::new();
        for (seq, payload) in rows {
            fixture.row(seq, &payload);
        }
        let outcome = fixture.project();
        assert!(
            outcome.as_ref().is_err_and(invalid_data),
            "{label}: {outcome:?}"
        );
        assert!(
            fixture.result_rows().is_empty(),
            "{label}: 拒否した履歴から行を作らない"
        );
    }
}
