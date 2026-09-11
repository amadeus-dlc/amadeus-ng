//! `JournalReaderImpl` の同居ストリーム（セッション監査・成果物監査・定義・intent・実行）の
//! 復号契約 — 健全な行は自分の列へ振り分けられ、壊れた行は成功に丸めず `Corrupt` で止まる。
//!
//! 行は本家の `journal` 表へ直接書く（書く側の Repository を通さない理由は
//! `tests/support/mod.rs` 冒頭を参照）。

// テストコードでは unwrap / expect / panic を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::needless_pass_by_value
)]
// ジャーナル行の payload は**契約 JSON ではなくワイヤ形式そのもの**であり、行を用意する
// テストは本家のシリアライザと同じ素の serde で書く (BR1.7 の射程外)。
#![allow(clippy::disallowed_methods)]

mod support;

use core_command_domain::orchestration::{
    ArtifactPaths, GateOpened, IntentExecution, IntentExecutionEvent, IntentExecutionEventId,
};
use core_command_domain::workflow_definition::StageSlug;
use core_command_domain::workspace::{
    ArtifactAuditId, HookHealthTarget, SessionAuditId, SpaceName, StorePath,
};
use core_read_model_updater::orchestration::{
    CorruptCause, GlobalSeqNr, IntentExecutionEventDto, JournalReadError, JournalReader,
    JournalReaderImpl, ProjectionName, WorkflowDefinitionEventDto,
};
use core_read_model_updater::read_tables::ReadTables;
use rusqlite::Connection;
use serde_json::Value;
use tempfile::TempDir;

use support::{
    DEFINITION_MANIFEST, MANIFEST, at, defined_event, execution_id, intent, open_store,
    redefined_event, seed, seed_definition, seed_intent,
};

/// 一時ディレクトリ配下の 1 つのストアファイル。
struct Fixture {
    _dir: TempDir,
    path: StorePath,
}

impl Fixture {
    fn new() -> Fixture {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let path = StorePath::for_space(&dir.path().join("aidlc"), &SpaceName::default());
        std::fs::create_dir_all(path.as_path().parent().expect("親 dir を持つ"))
            .expect("intents/ を先に作る");
        // 本家の `journal` 表は本家のストアに作らせる（行だけをこの側が差し込む）。
        drop(open_store(&path));
        Fixture { _dir: dir, path }
    }

    fn raw(&self) -> Connection {
        Connection::open(self.path.as_path()).expect("生の接続")
    }

    fn journal_reader(&self) -> JournalReaderImpl {
        JournalReaderImpl::open(&self.path).expect("Reader は開ける")
    }

    /// `journal` 表へ 1 行を直接差し込む（本家の封筒を経由しない）。
    fn insert_row(&self, aid: &str, seq_nr: i64, payload: &[u8], manifest: &str) {
        self.raw()
            .execute(
                "INSERT INTO journal (pkey, skey, aid, seq_nr, payload, occurred_at, manifest)
                 VALUES ('manual', ?1, ?2, ?3, ?4, 0, ?5)",
                rusqlite::params![
                    format!("manual-{aid}-{seq_nr}-{}", payload.len()),
                    aid,
                    seq_nr,
                    payload,
                    manifest
                ],
            )
            .expect("行を差し込む");
    }

    async fn read_all(
        &self,
    ) -> Result<core_read_model_updater::orchestration::JournalBatch, JournalReadError> {
        self.journal_reader().events_after(GlobalSeqNr::ZERO).await
    }
}

fn projection() -> ProjectionName {
    ProjectionName::parse("state-file").expect("投影名は kebab")
}

fn target() -> HookHealthTarget {
    HookHealthTarget::parse("spaces/default/intents").unwrap()
}

const SESSION_MANIFEST: &str = "session-audit-event/1";
const ARTIFACT_MANIFEST: &str = "artifact-audit-event/1";
const SESSION_EVENT: &str = "0191aaaa-bbbb-7ccc-9ddd-eeeeffff0101";
const OBSERVATION: &str = "0191aaaa-bbbb-7ccc-9ddd-eeeeffff0102";
const ARTIFACT_EVENT: &str = "0191aaaa-bbbb-7ccc-9ddd-eeeeffff0103";

/// 健全なセッション監査行（書く側 `SessionAuditEventDto` と同じ綴り）。
fn session_row(aggregate: &str) -> Value {
    serde_json::json!({
        "id": SESSION_EVENT,
        "observation_id": OBSERVATION,
        "aggregate_id": aggregate,
        "target": "spaces/default/intents",
        "record": {"kind": "SESSION_ENDED", "fields": [["Reason", "clear"]]}
    })
}

/// 健全な成果物監査行（書く側 `ArtifactAuditEventDto` と同じ綴り）。
fn artifact_row(aggregate: &str) -> Value {
    serde_json::json!({
        "id": ARTIFACT_EVENT,
        "aggregate_id": aggregate,
        "target": "spaces/default/intents",
        "tool": "Write",
        "file": "ideation/intent-capture/intent.md",
        "context": "intent-capture",
        "created": true
    })
}

fn corrupt(error: JournalReadError) -> (String, Option<usize>, CorruptCause) {
    match error {
        JournalReadError::Corrupt {
            aggregate_id,
            seq_nr,
            cause,
        } => (aggregate_id, seq_nr, cause),
        other => panic!("Corrupt を期待した: {other:?}"),
    }
}

#[tokio::test]
async fn session_and_artifact_rows_are_consumed_into_their_own_streams() {
    let fixture = Fixture::new();
    let session_id = SessionAuditId::for_target(&target()).to_string();
    let artifact_id = ArtifactAuditId::for_target(&target()).to_string();
    fixture.insert_row(
        &session_id,
        1,
        session_row(&session_id).to_string().as_bytes(),
        SESSION_MANIFEST,
    );
    fixture.insert_row(
        &artifact_id,
        1,
        artifact_row(&artifact_id).to_string().as_bytes(),
        ARTIFACT_MANIFEST,
    );
    let batch = fixture.read_all().await.expect("健全な 2 行");
    assert!(batch.executions().is_empty() && batch.intents().is_empty());
    let session = &batch.sessions()[0];
    assert_eq!(session.seq_nr(), 1);
    assert_eq!(session.global_seq(), GlobalSeqNr::new(1));
    assert_eq!(session.event().record().kind().as_str(), "SESSION_ENDED");
    assert_eq!(session.event().observation_id().as_str(), OBSERVATION);
    let artifact = &batch.artifacts()[0];
    assert_eq!(artifact.seq_nr(), 1);
    assert_eq!(artifact.global_seq(), GlobalSeqNr::new(2));
    assert_eq!(
        artifact.event().observation().file(),
        "ideation/intent-capture/intent.md"
    );
    assert!(artifact.event().observation().created());
    // 監査だけの履歴からも構造化面の行が導ける。
    let tables = ReadTables::project_audit_only(&batch).expect("監査だけの投影");
    assert_eq!(tables.session_audits().len(), 1);
    assert_eq!(tables.session_audits()[0].kind(), "SESSION_ENDED");
    assert_eq!(tables.artifact_audits().len(), 1);
    assert_eq!(tables.artifact_audits()[0].tool(), "Write");
}

#[tokio::test]
async fn a_session_row_that_cannot_be_decoded_is_corrupt() {
    let session_id = SessionAuditId::for_target(&target()).to_string();
    let other = SessionAuditId::for_target(&HookHealthTarget::new(
        SpaceName::parse("other").unwrap(),
        None,
    ))
    .to_string();
    let mut mismatched = session_row(&session_id);
    mismatched["aggregate_id"] = Value::String(other.clone());
    let mut unknown_kind = session_row(&session_id);
    unknown_kind["record"]["kind"] = "NOT_A_KIND".into();
    let mut bad_key = session_row(&session_id);
    bad_key["record"]["fields"] = serde_json::json!([["Reason", "x"], ["Event", "x"]]);
    let mut raw_newline = session_row(&session_id);
    raw_newline["record"]["fields"] = serde_json::json!([["Reason", "two\nlines"]]);
    let mut duplicate_key = session_row(&session_id);
    duplicate_key["record"]["fields"] = serde_json::json!([["Reason", "a"], ["Reason", "b"]]);
    let mut bad_target = session_row(&session_id);
    bad_target["target"] = "elsewhere".into();
    for (label, payload) in [
        ("JSON でない", b"{not json".to_vec()),
        (
            "行の aid と payload の集約 id が食い違う",
            mismatched.to_string().into_bytes(),
        ),
        ("未知の監査種別", unknown_kind.to_string().into_bytes()),
        ("発行側の所有する項目名", bad_key.to_string().into_bytes()),
        ("改行を含む生の値", raw_newline.to_string().into_bytes()),
        ("重複する項目名", duplicate_key.to_string().into_bytes()),
        ("対象の文法外", bad_target.to_string().into_bytes()),
    ] {
        let fixture = Fixture::new();
        fixture.insert_row(&session_id, 1, &payload, SESSION_MANIFEST);
        let (aid, seq, cause) = corrupt(fixture.read_all().await.expect_err(label));
        assert_eq!(
            (aid.as_str(), seq, cause),
            (session_id.as_str(), None, CorruptCause::UndecodablePayload),
            "{label}"
        );
    }
}

#[tokio::test]
async fn an_artifact_row_that_cannot_be_decoded_is_corrupt() {
    let artifact_id = ArtifactAuditId::for_target(&target()).to_string();
    let other = ArtifactAuditId::for_target(&HookHealthTarget::new(
        SpaceName::parse("other").unwrap(),
        None,
    ))
    .to_string();
    let mut bad_id = artifact_row(&artifact_id);
    bad_id["id"] = "x".into();
    let mut bad_aggregate = artifact_row(&artifact_id);
    bad_aggregate["aggregate_id"] = "artifact-audit:short".into();
    let mut bad_target = artifact_row(&artifact_id);
    bad_target["target"] = "elsewhere".into();
    let mut mismatched = artifact_row(&artifact_id);
    mismatched["aggregate_id"] = Value::String(other);
    for (label, payload, cause) in [
        (
            "JSON でない",
            b"{not json".to_vec(),
            CorruptCause::UndecodablePayload,
        ),
        (
            "イベント id の文法外",
            bad_id.to_string().into_bytes(),
            CorruptCause::UndecodablePayload,
        ),
        (
            "集約 id の文法外",
            bad_aggregate.to_string().into_bytes(),
            CorruptCause::UndecodablePayload,
        ),
        (
            "対象の文法外",
            bad_target.to_string().into_bytes(),
            CorruptCause::UndecodablePayload,
        ),
        (
            "集約 id が対象から導いたものと食い違う",
            mismatched.to_string().into_bytes(),
            CorruptCause::InvariantViolation,
        ),
    ] {
        let fixture = Fixture::new();
        fixture.insert_row(&artifact_id, 1, &payload, ARTIFACT_MANIFEST);
        let (aid, seq, actual) = corrupt(fixture.read_all().await.expect_err(label));
        assert_eq!(
            (aid.as_str(), seq, actual),
            (artifact_id.as_str(), None, cause),
            "{label}"
        );
    }
}

#[tokio::test]
async fn a_negative_sequence_number_is_corrupt_in_every_stream() {
    let session_id = SessionAuditId::for_target(&target()).to_string();
    let artifact_id = ArtifactAuditId::for_target(&target()).to_string();
    let defined = serde_json::to_vec(&WorkflowDefinitionEventDto::of(&defined_event())).unwrap();
    let intent_row =
        serde_json::to_vec(&core_read_model_updater::orchestration::IntentEventDto::of(
            &support::intent_created_event(),
            at(),
        ))
        .unwrap();
    let (_, started) = IntentExecution::start(execution_id(), &intent(), at());
    let execution_row = serde_json::to_vec(&IntentExecutionEventDto::of(&started)).unwrap();
    for (label, aid, payload, manifest) in [
        (
            "セッション監査",
            session_id.clone(),
            session_row(&session_id).to_string().into_bytes(),
            SESSION_MANIFEST,
        ),
        (
            "成果物監査",
            artifact_id.clone(),
            artifact_row(&artifact_id).to_string().into_bytes(),
            ARTIFACT_MANIFEST,
        ),
        (
            "定義",
            support::DEFINITION.to_string(),
            defined.clone(),
            DEFINITION_MANIFEST,
        ),
        (
            "intent",
            support::INTENT.to_string(),
            intent_row.clone(),
            "intent-event/1",
        ),
        (
            "実行",
            support::EXECUTION.to_string(),
            execution_row.clone(),
            MANIFEST,
        ),
    ] {
        let fixture = Fixture::new();
        fixture.insert_row(&aid, -1, &payload, manifest);
        let (actual_aid, seq, cause) = corrupt(fixture.read_all().await.expect_err(label));
        assert_eq!(
            (actual_aid.as_str(), seq, cause),
            (aid.as_str(), None, CorruptCause::InvariantViolation),
            "{label}"
        );
    }
}

#[tokio::test]
async fn a_definition_row_is_refused_when_its_id_payload_or_sequence_disagree() {
    let defined = serde_json::to_vec(&WorkflowDefinitionEventDto::of(&defined_event())).unwrap();
    let redefined =
        serde_json::to_vec(&WorkflowDefinitionEventDto::of(&redefined_event())).unwrap();
    let mut bad_slug: Value = serde_json::from_slice(&defined).unwrap();
    // グラフの先頭ノードの slug を文法外にする — DTO は読めるがドメインへは写せない。
    bad_slug["Defined"]["content"]["graph"][0]["slug"] = "Bad Slug".into();
    for (label, aid, seq, payload, expected_seq, cause) in [
        (
            "定義 id が文法外 (空白だけ)",
            "   ".to_string(),
            1,
            defined.clone(),
            Some(1),
            CorruptCause::InvariantViolation,
        ),
        (
            "定義 id が payload の系譜と食い違う",
            "kiro".to_string(),
            1,
            defined.clone(),
            Some(1),
            CorruptCause::InvariantViolation,
        ),
        (
            "payload がドメインへ写せない",
            support::DEFINITION.to_string(),
            1,
            bad_slug.to_string().into_bytes(),
            Some(1),
            CorruptCause::UndecodablePayload,
        ),
        (
            "誕生が通番 1 でない",
            support::DEFINITION.to_string(),
            2,
            defined.clone(),
            Some(2),
            CorruptCause::InvariantViolation,
        ),
        (
            "改訂が通番 1 を名乗る",
            support::DEFINITION.to_string(),
            1,
            redefined.clone(),
            Some(1),
            CorruptCause::InvariantViolation,
        ),
    ] {
        let fixture = Fixture::new();
        fixture.insert_row(&aid, seq, &payload, DEFINITION_MANIFEST);
        let (actual_aid, actual_seq, actual) = corrupt(fixture.read_all().await.expect_err(label));
        assert_eq!(
            (actual_aid.as_str(), actual_seq, actual),
            (aid.as_str(), expected_seq, cause),
            "{label}"
        );
    }
}

#[tokio::test]
async fn an_execution_row_is_refused_when_its_id_manifest_or_sequence_disagree() {
    let (_, started) = IntentExecution::start(execution_id(), &intent(), at());
    let started_row = serde_json::to_vec(&IntentExecutionEventDto::of(&started)).unwrap();
    let opened = IntentExecutionEvent::GateOpened(GateOpened::new(
        IntentExecutionEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002").unwrap(),
        execution_id(),
        StageSlug::parse("intent-capture").unwrap(),
        ArtifactPaths::new(vec!["intent.md".to_string()]),
    ));
    let opened_row = serde_json::to_vec(&IntentExecutionEventDto::of(&opened)).unwrap();
    for (label, aid, seq, payload, manifest, cause) in [
        (
            "実行 id が文法外",
            "not-an-execution".to_string(),
            1,
            started_row.clone(),
            MANIFEST,
            CorruptCause::InvariantViolation,
        ),
        (
            "未知の型判別子",
            support::EXECUTION.to_string(),
            1,
            started_row.clone(),
            "something-else/1",
            CorruptCause::UndecodablePayload,
        ),
        (
            "誕生が通番 1 でない",
            support::EXECUTION.to_string(),
            2,
            started_row.clone(),
            MANIFEST,
            CorruptCause::InvariantViolation,
        ),
        (
            "誕生でない行が通番 1 を名乗る",
            support::EXECUTION.to_string(),
            1,
            opened_row.clone(),
            MANIFEST,
            CorruptCause::InvariantViolation,
        ),
    ] {
        let fixture = Fixture::new();
        fixture.insert_row(&aid, seq, &payload, manifest);
        let (actual_aid, actual_seq, actual) = corrupt(fixture.read_all().await.expect_err(label));
        assert_eq!(
            (actual_aid.as_str(), actual_seq, actual),
            (aid.as_str(), Some(seq as usize), cause),
            "{label}"
        );
    }
}

/// 正のチェックポイントにアンカーが無い行は、直接改変の兆候として拒否される。
#[tokio::test]
async fn a_positive_checkpoint_without_its_anchor_is_refused() {
    let fixture = Fixture::new();
    let mut store = open_store(&fixture.path);
    seed_intent(&fixture.path).await;
    seed(&mut store).await;
    drop(store);
    let mut journal_reader = fixture.journal_reader();
    let history = journal_reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("全件");
    let last = history.scanned_to().expect("走査位置");
    let tables = ReadTables::project(&history).expect("投影");
    journal_reader
        .advance_checkpoint(&projection(), last, &tables)
        .await
        .expect("前進");
    drop(journal_reader);
    fixture
        .raw()
        .execute(
            "UPDATE amadeus_projection_checkpoint SET anchor_aid = NULL, anchor_seq_nr = NULL",
            [],
        )
        .expect("アンカーを消す");
    let (aid, seq, cause) = corrupt(
        fixture
            .journal_reader()
            .checkpoint(&projection())
            .await
            .expect_err("アンカーの無い正のチェックポイント"),
    );
    assert_eq!(
        (aid.as_str(), seq, cause),
        ("-", None, CorruptCause::CheckpointAnchorMismatch)
    );
}

/// 版の違うストアの作り直しで、行は読めるが表を描けない歴史（誕生の欠けた実行）に
/// 当たったら、版を上げずに止める。
#[tokio::test]
async fn a_rebuild_whose_history_cannot_be_projected_stops_without_bumping_the_version() {
    let fixture = Fixture::new();
    let mut store = open_store(&fixture.path);
    seed_intent(&fixture.path).await;
    seed(&mut store).await;
    drop(store);
    let mut journal_reader = fixture.journal_reader();
    let history = journal_reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("全件");
    let last = history.scanned_to().expect("走査位置");
    let tables = ReadTables::project(&history).expect("投影");
    journal_reader
        .advance_checkpoint(&projection(), last, &tables)
        .await
        .expect("前進");
    drop(journal_reader);
    // 版を戻し、誕生の行だけを消す — 各行は復号できるが、集約は起こせない。
    let raw = fixture.raw();
    raw.execute_batch("PRAGMA user_version = 0")
        .expect("版を戻す");
    raw.execute(
        "DELETE FROM journal WHERE aid = ?1 AND seq_nr = 1",
        [support::EXECUTION],
    )
    .expect("誕生の行を消す");
    drop(raw);
    let (aid, seq, cause) = corrupt(
        JournalReaderImpl::open(&fixture.path).expect_err("誕生の欠けた歴史は描き直せない"),
    );
    assert_eq!(
        (aid.as_str(), seq, cause),
        ("-", None, CorruptCause::InvariantViolation)
    );
    let version: i64 = fixture
        .raw()
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("版");
    assert_eq!(version, 0, "描き直せていないなら版も上げない");
}

/// 読取表の**列が欠けた**形（`id` と `as_of` は在るので走査位置の照合は通るが INSERT が
/// 解決できない）に当たった全差し替えは、その表の INSERT で失敗し、チェックポイントも行も
/// 動かさない。
#[tokio::test]
async fn a_read_table_missing_its_columns_fails_the_insert_of_that_table() {
    for table in [
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
    ] {
        let fixture = Fixture::new();
        let mut store = open_store(&fixture.path);
        seed_intent(&fixture.path).await;
        seed_definition(&fixture.path).await;
        seed(&mut store).await;
        drop(store);
        let mut journal_reader = fixture.journal_reader();
        let history = journal_reader
            .events_after(GlobalSeqNr::ZERO)
            .await
            .expect("全件");
        let last = history.scanned_to().expect("走査位置");
        let tables = ReadTables::project(&history).expect("投影");
        fixture
            .raw()
            .execute_batch(&format!(
                "DROP TABLE {table}; CREATE TABLE {table} (id TEXT, as_of INTEGER)"
            ))
            .expect("列の欠けた形へ作り替える");
        let error = journal_reader
            .advance_checkpoint(&projection(), last, &tables)
            .await
            .expect_err("列の欠けた表への INSERT は失敗する");
        assert!(
            matches!(error, JournalReadError::Io { .. }),
            "{table}: SQLite の失敗は I/O の失敗として上がる (実際: {error:?})"
        );
        assert_eq!(
            journal_reader
                .checkpoint(&projection())
                .await
                .expect("読取"),
            GlobalSeqNr::ZERO,
            "{table}: 失敗した前進はチェックポイントを動かさない"
        );
        let rows: i64 = fixture
            .raw()
            .query_row("SELECT COUNT(*) FROM read_execution", [], |row| row.get(0))
            .expect("件数");
        assert_eq!(rows, 0, "{table}: 他の表にも行を残さない");
    }
}

/// 参照入力（steering）の読取表の列が欠けていれば、その表の INSERT で失敗として上がる。
#[tokio::test]
async fn a_steering_table_missing_its_columns_fails_the_replacement() {
    use core_read_model_updater::read_tables::{MemoryRules, RuleContent, SteeringTables};
    for table in ["read_steering_plan", "read_steering_part"] {
        let fixture = Fixture::new();
        let mut store = open_store(&fixture.path);
        seed(&mut store).await;
        drop(store);
        let mut reader = fixture.journal_reader();
        let tables = SteeringTables::pack(&MemoryRules::new(
            vec![RuleContent::new(
                "org.md".to_string(),
                "# Organization\nALWAYS keep the audit record.\n".to_string(),
            )],
            std::collections::BTreeMap::new(),
        ))
        .unwrap();
        fixture
            .raw()
            .execute_batch(&format!(
                "DROP TABLE {table}; CREATE TABLE {table} (id TEXT)"
            ))
            .expect("列の欠けた形へ作り替える");
        let error = reader
            .replace_steering(&tables)
            .await
            .expect_err("列の欠けた表への INSERT は失敗する");
        assert!(
            matches!(error, JournalReadError::Io { .. }),
            "{table}: 実際 {error:?}"
        );
        let rows: i64 = fixture
            .raw()
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(rows, 0, "{table}: 失敗した差し替えは行を残さない");
    }
}

/// 古い履歴位置で組んだテスト契約の面は、より新しい面を上書きしない。
#[tokio::test]
async fn an_older_testing_snapshot_does_not_overwrite_a_newer_one() {
    use core_command_domain::orchestration::TestingSections;
    use core_read_model_updater::orchestration::JournalBatch;
    use core_read_model_updater::read_tables::TestingTables;
    let fixture = Fixture::new();
    let mut store = open_store(&fixture.path);
    seed_intent(&fixture.path).await;
    seed(&mut store).await;
    drop(store);
    let mut reader = fixture.journal_reader();
    let full = reader.events_after(GlobalSeqNr::ZERO).await.unwrap();
    let newer = TestingTables::project(
        &full,
        &TestingSections::from_documents("## Testing Posture\n\n- Methodology: tdd\n", "", ""),
    );
    let older = TestingTables::project(
        &JournalBatch::empty(),
        &TestingSections::from_documents("", "", ""),
    );
    assert!(older.as_of() < newer.as_of());
    assert_ne!(older.source_digest(), newer.source_digest());
    reader.replace_testing(&newer).await.unwrap();
    reader.replace_testing(&older).await.unwrap();
    assert_eq!(
        reader.testing_source_digest().await.unwrap(),
        Some(newer.source_digest().to_string()),
        "古い断面は無視され、新しい断面が残る"
    );
    let rows: i64 = fixture
        .raw()
        .query_row("SELECT COUNT(*) FROM read_testing_contract", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(rows, newer.rows().len() as i64);
}
