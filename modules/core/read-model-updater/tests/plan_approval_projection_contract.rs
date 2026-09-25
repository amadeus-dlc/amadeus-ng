//! ワークスペース共有の承認ストアを RMU が読み、操作行・回答行・受領ファイルへ投影する契約。
//! 壊れた行・通番の欠落・チェックポイントの不整合は成功に丸めず拒否する。
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
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    CodeGenerationAuthority, IntentId, PlanApprovalEvidence, PlanApprovalOperationId,
    PlanApprovalRuntime, PlanChallenge, PlanSession, PlanTarget,
};
use core_command_domain::workspace::StorePath;
use core_read_model_updater::orchestration::{
    CorruptCause, JournalReadError, PlanApprovalJournalEntry, PlanApprovalJournalReaderImpl,
    PlanApprovalReadModelUpdater, ReadModelUpdater,
};
use core_read_model_updater::read_tables::PlanApprovalTables;
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

const EVENT_ID: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff";

struct Fixture {
    root: tempfile::TempDir,
    store: StorePath,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let store = StorePath::for_runtime(&root.path().join("aidlc"));
        std::fs::create_dir_all(store.as_path().parent().unwrap()).unwrap();
        Connection::open(store.as_path())
            .unwrap()
            .execute_batch("CREATE TABLE journal(id INTEGER PRIMARY KEY AUTOINCREMENT,pkey TEXT NOT NULL,skey TEXT NOT NULL,aid TEXT NOT NULL,seq_nr INTEGER NOT NULL,payload BLOB NOT NULL,occurred_at INTEGER NOT NULL,manifest TEXT NOT NULL)")
            .unwrap();
        Self { root, store }
    }
    fn raw(&self) -> Connection {
        Connection::open(self.store.as_path()).unwrap()
    }
    fn aidlc(&self) -> std::path::PathBuf {
        self.root.path().join("aidlc")
    }
    fn plan_dir(&self) -> std::path::PathBuf {
        self.aidlc().join(".aidlc-sessions").join("plan-approval")
    }
    fn insert(&self, seq: i64, aid: &str, manifest: &str, payload: &[u8]) {
        self.raw().execute("INSERT INTO journal(pkey,skey,aid,seq_nr,payload,occurred_at,manifest) VALUES('plan',?1,?2,?3,?4,?5,?6)",params![format!("{aid}-{seq}"),aid,seq,payload.to_vec(),1_757_300_000_000_000_000_i64+seq,manifest]).unwrap();
    }
    fn row(&self, seq: i64, payload: &Value) {
        self.insert(
            seq,
            "workspace",
            "plan-approval-event/1",
            payload.to_string().as_bytes(),
        );
    }
    fn project(&self) -> Result<(), JournalReadError> {
        let reader = PlanApprovalJournalReaderImpl::open(&self.store)?;
        block_on(PlanApprovalReadModelUpdater::new(reader).update_read_models())
    }
    fn checkpoint(&self) -> Option<(i64, Option<String>)> {
        self.raw()
            .query_row(
                "SELECT last_seq, anchor_event_id FROM amadeus_plan_projection_checkpoint WHERE projection='plan-approval'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok()
    }
    fn operations(&self) -> Vec<(String, String, String)> {
        let db = self.raw();
        let mut st = db
            .prepare(
                "SELECT operation_id,status,kind FROM read_plan_operation ORDER BY operation_id",
            )
            .unwrap();
        st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }
}

fn corrupt(error: &JournalReadError) -> Option<CorruptCause> {
    match error {
        JournalReadError::Corrupt {
            aggregate_id,
            cause,
            ..
        } if aggregate_id == "workspace" => Some(*cause),
        _ => None,
    }
}
fn event(index: u16, payload: Value) -> Value {
    json!({"id": format!("{EVENT_ID}{index:04x}"), "aggregate_id": "workspace", "payload": payload})
}
fn op(index: u16) -> String {
    format!("{EVENT_ID}{index:04x}")
}
fn evidence() -> PlanApprovalEvidence {
    PlanApprovalEvidence::new(
        CodeGenerationAuthority::new(
            &PlanTarget::stage_level(),
            &IntentId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
            format!("sha256:{}", "a".repeat(64)),
            "WORKFLOW_STARTED:2026-09-08T01:00:00Z#1".to_string(),
            "b".repeat(64),
            2,
        )
        .unwrap(),
        format!("sha256:{}", "c".repeat(64)),
        "questions.md".to_string(),
        "d".repeat(64),
        "e".repeat(64),
    )
    .unwrap()
}
fn evidence_json() -> Value {
    json!({
        "authority": {"unit": null, "intent_id": "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000", "directive_epoch": format!("sha256:{}", "a".repeat(64)), "run_floor": "WORKFLOW_STARTED:2026-09-08T01:00:00Z#1", "source_floor": "b".repeat(64), "marker_revision": 2},
        "fingerprint": format!("sha256:{}", "c".repeat(64)),
        "questions_file": "questions.md",
        "questions_sha256": "d".repeat(64),
        "prompt_sha256": "e".repeat(64)
    })
}
fn challenge() -> PlanChallenge {
    PlanChallenge::issue(
        evidence(),
        PlanSession::new("session".to_string()).unwrap(),
        ["Approve Plan".to_string(), "Request Changes".to_string()],
        false,
    )
}
fn challenge_issued(index: u16, occurrence: u16, response: Option<Value>) -> Value {
    event(
        index,
        json!({"type": "ChallengeIssued", "value": {
            "id": op(occurrence),
            "challenge": {"id": challenge().id(), "evidence": evidence_json(), "session": "session", "options": ["Approve Plan", "Request Changes"], "require_exact": false},
            "response": response
        }}),
    )
}
fn answer_input_json(choice: &str) -> Value {
    answer_input_with_session(choice, "session")
}
fn answer_input_with_session(choice: &str, session: &str) -> Value {
    json!({"space": "default", "execution_id": "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0001", "stage": "code-generation", "evidence": evidence_json(), "session": session, "choice": choice, "source": "b".repeat(64)})
}

#[test]
fn an_empty_store_projects_empty_tables_and_no_directory() {
    let fixture = Fixture::new();
    fixture.project().unwrap();
    assert_eq!(fixture.checkpoint(), Some((0, None)));
    assert!(fixture.operations().is_empty());
    assert!(
        !fixture.plan_dir().exists(),
        "投影対象が無ければディレクトリを作らない"
    );
    let tables = PlanApprovalTables::project(&[]).unwrap();
    assert!(tables.rows().is_empty() && tables.files().is_empty());
    assert_eq!(tables.sequence(), 0);
}

#[test]
fn a_legacy_operation_table_without_the_kind_column_is_migrated_on_open() {
    let fixture = Fixture::new();
    fixture.raw().execute_batch("CREATE TABLE read_plan_operation(operation_id TEXT PRIMARY KEY, status TEXT NOT NULL, space TEXT, execution_id TEXT, as_of INTEGER NOT NULL)").unwrap();
    PlanApprovalJournalReaderImpl::open(&fixture.store).unwrap();
    let kind: i64 = fixture
        .raw()
        .query_row(
            "SELECT count(*) FROM pragma_table_info('read_plan_operation') WHERE name='kind'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(kind, 1);
}

#[test]
fn opening_a_missing_store_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let store = StorePath::for_runtime(&root.path().join("aidlc"));
    assert!(PlanApprovalJournalReaderImpl::open(&store).is_err());
}

#[test]
fn a_challenge_with_its_response_and_answers_project_operation_rows_and_files() {
    let fixture = Fixture::new();
    fixture.row(1, &event(1, json!({"type": "Created"})));
    fixture.row(
        2,
        &challenge_issued(
            2,
            0x10,
            Some(json!({"id": op(0x11), "occurrence_id": op(0x10), "choice": "Approve Plan", "response_sha256": "f".repeat(64)})),
        ),
    );
    // 受領のキーは対象と発行エポックで決まるので、中断・失効で消える受領を先に置き、
    // 最後に残る承認受領と生成受領を後ろに置く。
    let approve = |index: u16, id: u16, session: &str| {
        event(
            index,
            json!({"type": "AnswerRecorded", "value": {
                "id": op(id), "input": answer_input_with_session("Approve Plan", session), "occurrence_id": op(0x10), "challenge_id": challenge().id(), "response_id": op(0x11),
                "receipt": {"evidence": evidence_json(), "session": session, "challenge_id": challenge().id(), "certified_source": "b".repeat(64), "status": "approved"},
                "state": "pending", "error": null
            }}),
        )
    };
    let generation = |index: u16, id: u16, session: &str| {
        event(
            index,
            json!({"type": "GenerationRequested", "value": {
                "id": op(id),
                "receipt": {"evidence": evidence_json(), "session": session, "challenge_id": challenge().id(), "certified_source": "b".repeat(64), "status": "generation"},
                "state": "pending"
            }}),
        )
    };
    fixture.row(3, &approve(3, 0x22, "session-3"));
    fixture.row(
        4,
        &event(
            4,
            json!({"type": "AnswerAborted", "value": {"operation_id": op(0x22)}}),
        ),
    );
    fixture.row(5, &generation(5, 0x31, "session-2"));
    fixture.row(
        6,
        &event(
            6,
            json!({"type": "GenerationRevoked", "value": {"operation_id": op(0x31)}}),
        ),
    );
    fixture.row(
        7,
        &event(7, json!({"type": "AnswerRecorded", "value": {
            "id": op(0x21), "input": answer_input_json("Request Changes"), "occurrence_id": op(0x10), "challenge_id": challenge().id(), "response_id": op(0x11),
            "receipt": null, "state": "pending", "error": null
        }})),
    );
    fixture.row(
        8,
        &event(
            8,
            json!({"type": "AnswerCompleted", "value": {"operation_id": op(0x21)}}),
        ),
    );
    fixture.row(9, &approve(9, 0x20, "session"));
    fixture.row(
        10,
        &event(
            10,
            json!({"type": "AnswerCompleted", "value": {"operation_id": op(0x20)}}),
        ),
    );
    fixture.row(11, &generation(11, 0x30, "session"));
    fixture.row(
        12,
        &event(
            12,
            json!({"type": "GenerationCertified", "value": {"operation_id": op(0x30)}}),
        ),
    );
    // 前回の投影で残った古いファイルは掃除される。
    std::fs::create_dir_all(fixture.plan_dir()).unwrap();
    std::fs::write(fixture.plan_dir().join("receipt-stale.json"), "{}").unwrap();
    fixture.project().unwrap();
    assert_eq!(fixture.checkpoint(), Some((12, Some(op(12)))));
    assert!(!fixture.plan_dir().join("receipt-stale.json").exists());
    let db = fixture.raw();
    let answers: Vec<(String, String, Option<String>, Option<String>)> = {
        let mut st = db.prepare("SELECT operation_id,status,emitted,error FROM read_plan_answer ORDER BY operation_id").unwrap();
        st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    };
    assert_eq!(
        answers,
        vec![
            (op(0x20), "recorded".into(), Some("PLAN_APPROVAL_RECORDED".into()), None),
            (op(0x21), "recorded".into(), Some("QUESTION_ANSWERED".into()), None),
            (
                op(0x22),
                "aborted".into(),
                None,
                Some("Plan Approval source changed during receipt certification; present the current plan again".into()),
            ),
        ]
    );
    let generations: Vec<(String, String)> = {
        let mut st = db
            .prepare("SELECT operation_id,status FROM read_plan_generation ORDER BY operation_id")
            .unwrap();
        st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    };
    assert_eq!(
        generations,
        vec![
            (op(0x30), "generation".into()),
            (op(0x31), "revoked".into())
        ]
    );
    let files: Vec<String> = std::fs::read_dir(fixture.plan_dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        !files.is_empty(),
        "承認済み回答の受領ファイルが公開される: {files:?}"
    );
    // 同じ履歴の再投影は同じ結果に落ち着き、内容の同じファイルを書き直さない。
    let before = std::fs::metadata(fixture.plan_dir().join(&files[0]))
        .unwrap()
        .modified()
        .unwrap();
    fixture.project().unwrap();
    assert_eq!(
        std::fs::metadata(fixture.plan_dir().join(&files[0]))
            .unwrap()
            .modified()
            .unwrap(),
        before
    );
}

#[test]
fn a_cleared_history_removes_the_projection_directory() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(fixture.plan_dir()).unwrap();
    std::fs::write(fixture.plan_dir().join("receipt-old.json"), "{}").unwrap();
    fixture.project().unwrap();
    assert!(
        !fixture.plan_dir().exists(),
        "公開物が無くなればディレクトリごと消す"
    );
}

#[test]
fn a_projection_target_that_is_not_a_plain_directory_is_refused() {
    let sessions_is_file = Fixture::new();
    std::fs::write(
        sessions_is_file.aidlc().join(".aidlc-sessions"),
        "not a dir",
    )
    .unwrap();
    match sessions_is_file.project().unwrap_err() {
        JournalReadError::Io { kind, path } => {
            assert_eq!(kind, std::io::ErrorKind::InvalidData);
            assert_eq!(path, Some(sessions_is_file.aidlc().join(".aidlc-sessions")));
        }
        other => panic!("Io を期待した: {other:?}"),
    }
    let nested_dir = Fixture::new();
    std::fs::create_dir_all(nested_dir.plan_dir().join("subdir")).unwrap();
    match nested_dir.project().unwrap_err() {
        JournalReadError::Io { kind, path } => {
            assert_eq!(kind, std::io::ErrorKind::InvalidData);
            assert_eq!(path, Some(nested_dir.plan_dir().join("subdir")));
        }
        other => panic!("Io を期待した: {other:?}"),
    }
}

#[test]
fn corrupt_approval_rows_are_refused() {
    let cases: Vec<(&str, Vec<(i64, &str, &str, Vec<u8>)>, CorruptCause)> = vec![
        (
            "先頭の通番が 1 でない",
            vec![(2, "workspace", "plan-approval-event/1", event(1, json!({"type": "Created"})).to_string().into_bytes())],
            CorruptCause::InvariantViolation,
        ),
        (
            "通番が 0",
            vec![(0, "workspace", "plan-approval-event/1", event(1, json!({"type": "Created"})).to_string().into_bytes())],
            CorruptCause::InvariantViolation,
        ),
        (
            "manifest が承認ストリームでない",
            vec![(1, "workspace", "hook-health-event/1", event(1, json!({"type": "Created"})).to_string().into_bytes())],
            CorruptCause::InvariantViolation,
        ),
        (
            "通番が飛ぶ",
            vec![
                (1, "workspace", "plan-approval-event/1", event(1, json!({"type": "Created"})).to_string().into_bytes()),
                (3, "workspace", "plan-approval-event/1", event(3, json!({"type": "Created"})).to_string().into_bytes()),
            ],
            CorruptCause::InvariantViolation,
        ),
        (
            "payload が JSON でない",
            vec![(1, "workspace", "plan-approval-event/1", b"{".to_vec())],
            CorruptCause::UndecodablePayload,
        ),
        (
            "集約 ID が workspace でない",
            vec![(1, "workspace", "plan-approval-event/1", json!({"id": op(1), "aggregate_id": "elsewhere", "payload": {"type": "Created"}}).to_string().into_bytes())],
            CorruptCause::UndecodablePayload,
        ),
        (
            "イベント ID が文法外",
            vec![(1, "workspace", "plan-approval-event/1", json!({"id": "x", "aggregate_id": "workspace", "payload": {"type": "Created"}}).to_string().into_bytes())],
            CorruptCause::UndecodablePayload,
        ),
        (
            "誕生で始まらない",
            vec![(1, "workspace", "plan-approval-event/1", event(1, json!({"type": "AnswerCompleted", "value": {"operation_id": op(9)}})).to_string().into_bytes())],
            CorruptCause::InvariantViolation,
        ),
        (
            "挑戦 ID が内容から導いた値と一致しない",
            vec![
                (1, "workspace", "plan-approval-event/1", event(1, json!({"type": "Created"})).to_string().into_bytes()),
                (2, "workspace", "plan-approval-event/1", {
                    let mut row = challenge_issued(2, 0x10, None);
                    row["payload"]["value"]["challenge"]["id"] = format!("sha256:{}", "0".repeat(64)).into();
                    row.to_string().into_bytes()
                }),
            ],
            CorruptCause::UndecodablePayload,
        ),
        (
            "応答が別の発行回を指す",
            vec![
                (1, "workspace", "plan-approval-event/1", event(1, json!({"type": "Created"})).to_string().into_bytes()),
                (2, "workspace", "plan-approval-event/1", challenge_issued(2, 0x10, Some(json!({"id": op(0x11), "occurrence_id": op(0x12), "choice": "Approve Plan", "response_sha256": "f".repeat(64)}))).to_string().into_bytes()),
            ],
            CorruptCause::UndecodablePayload,
        ),
        (
            "応答の要約が SHA-256 でない",
            vec![
                (1, "workspace", "plan-approval-event/1", event(1, json!({"type": "Created"})).to_string().into_bytes()),
                (2, "workspace", "plan-approval-event/1", challenge_issued(2, 0x10, Some(json!({"id": op(0x11), "occurrence_id": op(0x10), "choice": "Approve Plan", "response_sha256": "short"}))).to_string().into_bytes()),
            ],
            CorruptCause::UndecodablePayload,
        ),
        (
            "回答の状態と誤りが噛み合わない",
            vec![
                (1, "workspace", "plan-approval-event/1", event(1, json!({"type": "Created"})).to_string().into_bytes()),
                (2, "workspace", "plan-approval-event/1", event(2, json!({"type": "AnswerRecorded", "value": {"id": op(0x20), "input": answer_input_json("Request Changes"), "occurrence_id": op(0x10), "challenge_id": challenge().id(), "response_id": op(0x11), "receipt": null, "state": "aborted", "error": null}})).to_string().into_bytes()),
            ],
            CorruptCause::UndecodablePayload,
        ),
        (
            "世代の状態が未知の綴り",
            vec![
                (1, "workspace", "plan-approval-event/1", event(1, json!({"type": "Created"})).to_string().into_bytes()),
                (2, "workspace", "plan-approval-event/1", event(2, json!({"type": "GenerationRequested", "value": {"id": op(0x30), "receipt": {"evidence": evidence_json(), "session": "session", "challenge_id": challenge().id(), "certified_source": "b".repeat(64), "status": "generation"}, "state": "done"}})).to_string().into_bytes()),
            ],
            CorruptCause::UndecodablePayload,
        ),
        (
            "受領の状態が未知の綴り",
            vec![
                (1, "workspace", "plan-approval-event/1", event(1, json!({"type": "Created"})).to_string().into_bytes()),
                (2, "workspace", "plan-approval-event/1", event(2, json!({"type": "GenerationRequested", "value": {"id": op(0x30), "receipt": {"evidence": evidence_json(), "session": "session", "challenge_id": challenge().id(), "certified_source": "b".repeat(64), "status": "sealed"}, "state": "pending"}})).to_string().into_bytes()),
            ],
            CorruptCause::UndecodablePayload,
        ),
    ];
    for (label, rows, expected) in cases {
        let fixture = Fixture::new();
        for (seq, aid, manifest, payload) in rows {
            fixture.insert(seq, aid, manifest, &payload);
        }
        let error = fixture.project().unwrap_err();
        assert_eq!(corrupt(&error), Some(expected), "{label}: {error:?}");
        assert_eq!(
            fixture.checkpoint(),
            None,
            "{label}: 拒否した履歴で checkpoint を進めない"
        );
    }
}

#[test]
fn a_checkpoint_ahead_of_the_journal_or_with_a_foreign_anchor_is_refused() {
    let ahead = Fixture::new();
    ahead.row(1, &event(1, json!({"type": "Created"})));
    PlanApprovalJournalReaderImpl::open(&ahead.store).unwrap();
    ahead
        .raw()
        .execute(
            "INSERT INTO amadeus_plan_projection_checkpoint VALUES('plan-approval', 5, NULL)",
            [],
        )
        .unwrap();
    assert_eq!(
        corrupt(&ahead.project().unwrap_err()),
        Some(CorruptCause::CheckpointAnchorMismatch)
    );
    let foreign = Fixture::new();
    foreign.row(1, &event(1, json!({"type": "Created"})));
    PlanApprovalJournalReaderImpl::open(&foreign.store).unwrap();
    foreign
        .raw()
        .execute(
            "INSERT INTO amadeus_plan_projection_checkpoint VALUES('plan-approval', 1, ?1)",
            [op(0x99)],
        )
        .unwrap();
    assert_eq!(
        corrupt(&foreign.project().unwrap_err()),
        Some(CorruptCause::CheckpointAnchorMismatch)
    );
    // 同じアンカーなら進める。
    let same = Fixture::new();
    same.row(1, &event(1, json!({"type": "Created"})));
    PlanApprovalJournalReaderImpl::open(&same.store).unwrap();
    same.raw()
        .execute(
            "INSERT INTO amadeus_plan_projection_checkpoint VALUES('plan-approval', 1, ?1)",
            [op(1)],
        )
        .unwrap();
    same.project().unwrap();
    assert_eq!(same.checkpoint(), Some((1, Some(op(1)))));
}

#[test]
fn the_tables_refuse_a_history_whose_sequence_is_broken() {
    let at: DateTime<Utc> = "2026-09-08T01:00:00Z".parse().unwrap();
    let (mut runtime, created) = PlanApprovalRuntime::create(at);
    let occurrence = PlanApprovalOperationId::generate();
    let issued = runtime
        .issue_challenge(occurrence, challenge(), at)
        .unwrap();
    let is_corrupt = |entries: &[PlanApprovalJournalEntry]| {
        matches!(
            PlanApprovalTables::project(entries),
            Err(JournalReadError::Corrupt {
                cause: CorruptCause::InvariantViolation,
                ..
            })
        )
    };
    assert!(
        is_corrupt(&[PlanApprovalJournalEntry::new(2, at, created.clone())]),
        "誕生の通番が 1 でない"
    );
    assert!(
        is_corrupt(&[PlanApprovalJournalEntry::new(1, at, issued.clone())]),
        "誕生で始まらない"
    );
    assert!(
        is_corrupt(&[
            PlanApprovalJournalEntry::new(1, at, created),
            PlanApprovalJournalEntry::new(3, at, issued),
        ]),
        "通番が飛ぶ"
    );
}

/// 承認済み回答 1 件の履歴（受領ファイルを 1 つ公開する最小の形）。
fn seed_one_approved_answer(fixture: &Fixture) {
    fixture.row(1, &event(1, json!({"type": "Created"})));
    fixture.row(
        2,
        &challenge_issued(
            2,
            0x10,
            Some(json!({"id": op(0x11), "occurrence_id": op(0x10), "choice": "Approve Plan", "response_sha256": "f".repeat(64)})),
        ),
    );
    fixture.row(
        3,
        &event(3, json!({"type": "AnswerRecorded", "value": {
            "id": op(0x20), "input": answer_input_json("Approve Plan"), "occurrence_id": op(0x10), "challenge_id": challenge().id(), "response_id": op(0x11),
            "receipt": {"evidence": evidence_json(), "session": "session", "challenge_id": challenge().id(), "certified_source": "b".repeat(64), "status": "approved"},
            "state": "pending", "error": null
        }})),
    );
    fixture.row(
        4,
        &event(
            4,
            json!({"type": "AnswerCompleted", "value": {"operation_id": op(0x20)}}),
        ),
    );
}

fn io_of(error: JournalReadError) -> (std::io::ErrorKind, Option<std::path::PathBuf>) {
    match error {
        JournalReadError::Io { kind, path } => (kind, path),
        other => panic!("Io を期待した: {other:?}"),
    }
}

#[cfg(unix)]
fn chmod(path: &std::path::Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

/// 受領ファイル 1 つを公開した直後の状態（ファイル名を返す）。
fn published_receipt(fixture: &Fixture) -> std::path::PathBuf {
    seed_one_approved_answer(fixture);
    fixture.project().unwrap();
    let mut files: Vec<_> = std::fs::read_dir(fixture.plan_dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1, "受領ファイルは 1 つ: {files:?}");
    files.pop().unwrap()
}

/// 公開先を OS が拒む（読めない・列挙できない・消せない・作れない・書けない）ときは、
/// 拒まれたパスと I/O の種類をそのまま報告し、成功に丸めない。
#[cfg(unix)]
#[test]
fn a_publication_target_the_os_refuses_is_reported_with_its_path() {
    // 1. 公開先の根 (aidlc/) を辿れない → `.aidlc-sessions` の stat が拒まれる。
    {
        let fixture = Fixture::new();
        seed_one_approved_answer(&fixture);
        let reader = PlanApprovalJournalReaderImpl::open(&fixture.store).unwrap();
        chmod(&fixture.aidlc(), 0o000);
        let outcome = block_on(PlanApprovalReadModelUpdater::new(reader).update_read_models());
        chmod(&fixture.aidlc(), 0o755);
        let (kind, path) = io_of(outcome.unwrap_err());
        assert_eq!(kind, std::io::ErrorKind::PermissionDenied);
        assert_eq!(path, Some(fixture.aidlc().join(".aidlc-sessions")));
        assert_eq!(fixture.checkpoint(), None, "チェックポイントは進まない");
    }
    // 2. 公開ディレクトリを列挙できない。
    {
        let fixture = Fixture::new();
        seed_one_approved_answer(&fixture);
        std::fs::create_dir_all(fixture.plan_dir()).unwrap();
        chmod(&fixture.plan_dir(), 0o000);
        let outcome = fixture.project();
        chmod(&fixture.plan_dir(), 0o755);
        let (kind, path) = io_of(outcome.unwrap_err());
        assert_eq!(kind, std::io::ErrorKind::PermissionDenied);
        assert_eq!(path, Some(fixture.plan_dir()));
    }
    // 3. 古いファイルを消せない。
    {
        let fixture = Fixture::new();
        seed_one_approved_answer(&fixture);
        std::fs::create_dir_all(fixture.plan_dir()).unwrap();
        let stale = fixture.plan_dir().join("receipt-stale.json");
        std::fs::write(&stale, "{}").unwrap();
        chmod(&fixture.plan_dir(), 0o555);
        let outcome = fixture.project();
        chmod(&fixture.plan_dir(), 0o755);
        let (kind, path) = io_of(outcome.unwrap_err());
        assert_eq!(kind, std::io::ErrorKind::PermissionDenied);
        assert_eq!(path, Some(stale.clone()));
        assert!(stale.exists());
    }
    // 4. 公開ディレクトリを作れない (`.aidlc-sessions` が書けない)。
    {
        let fixture = Fixture::new();
        seed_one_approved_answer(&fixture);
        let sessions = fixture.aidlc().join(".aidlc-sessions");
        std::fs::create_dir_all(&sessions).unwrap();
        chmod(&sessions, 0o555);
        let outcome = fixture.project();
        chmod(&sessions, 0o755);
        let (kind, path) = io_of(outcome.unwrap_err());
        assert_eq!(kind, std::io::ErrorKind::PermissionDenied);
        assert_eq!(path, Some(fixture.plan_dir()));
    }
    // 5. 空になった公開ディレクトリを消せない。
    {
        let fixture = Fixture::new();
        let sessions = fixture.aidlc().join(".aidlc-sessions");
        std::fs::create_dir_all(fixture.plan_dir()).unwrap();
        chmod(&sessions, 0o555);
        let outcome = fixture.project();
        chmod(&sessions, 0o755);
        let (kind, path) = io_of(outcome.unwrap_err());
        assert_eq!(kind, std::io::ErrorKind::PermissionDenied);
        assert_eq!(path, Some(fixture.plan_dir()));
    }
    // 6. 既存の受領ファイルを読めない。
    {
        let fixture = Fixture::new();
        let receipt = published_receipt(&fixture);
        chmod(&receipt, 0o000);
        let outcome = fixture.project();
        chmod(&receipt, 0o644);
        let (kind, path) = io_of(outcome.unwrap_err());
        assert_eq!(kind, std::io::ErrorKind::PermissionDenied);
        assert_eq!(path, Some(receipt));
    }
    // 7. 受領ファイルを書けない (ディレクトリが書けず、ファイルも無い)。
    {
        let fixture = Fixture::new();
        let receipt = published_receipt(&fixture);
        std::fs::remove_file(&receipt).unwrap();
        chmod(&fixture.plan_dir(), 0o555);
        let outcome = fixture.project();
        chmod(&fixture.plan_dir(), 0o755);
        let (kind, path) = io_of(outcome.unwrap_err());
        assert_eq!(kind, std::io::ErrorKind::PermissionDenied);
        assert_eq!(path, Some(receipt));
    }
}

/// 同名で中身の違う受領ファイルは、保存済みの内容へ書き直される。
#[test]
fn a_receipt_whose_bytes_drifted_is_rewritten_from_the_history() {
    let fixture = Fixture::new();
    let receipt = published_receipt(&fixture);
    let expected = std::fs::read(&receipt).unwrap();
    std::fs::write(&receipt, b"{\"tampered\":true}").unwrap();
    fixture.project().unwrap();
    assert_eq!(std::fs::read(&receipt).unwrap(), expected);
}

/// 応答準備・失効準備・応答観測・失効解決の行は、識別子・space・セッション・選択の文法外を
/// 復号不能として拒否し、checkpoint を進めない。
#[test]
fn corrupt_response_and_invalidation_rows_are_refused_as_undecodable() {
    let prepared = |space: &str, execution: &str, session: &str, choice: Value| {
        json!({"type": "ResponsePrepared", "value": {
            "id": op(0x40), "space": space, "execution_id": execution, "occurrence_id": op(0x10),
            "session": session, "response": "A", "choice": choice
        }})
    };
    let invalidation = |space: &str, execution: &str| {
        json!({"type": "InvalidationPrepared", "value": {
            "id": op(0x50), "space": space, "execution_id": execution
        }})
    };
    let observed = |observation: &str, session: &str, choice: Value| {
        json!({"type": "ResponseObserved", "value": {
            "observation_id": observation, "occurrence_id": op(0x10), "session": session,
            "response_sha256": "f".repeat(64), "choice": choice
        }})
    };
    let execution = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0001";
    let cases: Vec<(&str, Value)> = vec![
        (
            "応答準備の space が文法外",
            prepared("../x", execution, "session", Value::Null),
        ),
        (
            "応答準備の実行 id が文法外",
            prepared("default", "x", "session", Value::Null),
        ),
        (
            "応答準備のセッションが空",
            prepared("default", execution, "", Value::Null),
        ),
        (
            "応答準備の選択が未知の綴り",
            prepared("default", execution, "session", "Maybe".into()),
        ),
        ("失効準備の space が文法外", invalidation("../x", execution)),
        ("失効準備の実行 id が文法外", invalidation("default", "x")),
        (
            "応答観測の観測 id が文法外",
            observed("x", "session", Value::Null),
        ),
        (
            "応答観測のセッションが空",
            observed(&op(0x60), "", Value::Null),
        ),
        (
            "応答観測の選択が未知の綴り",
            observed(&op(0x60), "session", "Maybe".into()),
        ),
        (
            "失効解決の操作 id が文法外",
            json!({"type": "InvalidationResolved", "value": {"operation_id": "x", "published": true}}),
        ),
        ("集約 id が workspace の綴りでない", Value::Null),
    ];
    for (label, payload) in cases {
        let fixture = Fixture::new();
        fixture.row(1, &event(1, json!({"type": "Created"})));
        if payload.is_null() {
            fixture.insert(
                2,
                "workspace",
                "plan-approval-event/1",
                json!({"id": op(2), "aggregate_id": "Workspace", "payload": {"type": "Created"}})
                    .to_string()
                    .as_bytes(),
            );
        } else {
            fixture.row(2, &event(2, payload));
        }
        let error = fixture.project().unwrap_err();
        assert_eq!(
            corrupt(&error),
            Some(CorruptCause::UndecodablePayload),
            "{label}: {error:?}"
        );
        assert_eq!(fixture.checkpoint(), None, "{label}");
    }
    // 健全な応答準備・失効準備・応答観測・失効解決は受理される。
    let fixture = Fixture::new();
    fixture.row(1, &event(1, json!({"type": "Created"})));
    fixture.row(2, &challenge_issued(2, 0x10, None));
    fixture.row(
        3,
        &event(
            3,
            prepared("default", execution, "session", "Approve Plan".into()),
        ),
    );
    fixture.row(4, &event(4, invalidation("default", execution)));
    fixture.row(
        5,
        &event(5, observed(&op(0x60), "session", "Request Changes".into())),
    );
    fixture.row(
        6,
        &event(6, json!({"type": "InvalidationResolved", "value": {"operation_id": op(0x50), "published": false}})),
    );
    fixture.project().unwrap();
    assert_eq!(fixture.checkpoint(), Some((6, Some(op(6)))));
}
