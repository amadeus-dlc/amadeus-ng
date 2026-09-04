//! `JournalReaderImpl` の契約 (BR1.4) — 全集約横断の順序読取と投影チェックポイント。
//!
//! この面は本家 event-store-adapter-rs に無い (ADR-010 決定 4)。本家の `journal` 表を
//! 同じ DB ファイルへの**別接続**から `rowid` 順に読み、チェックポイントは我々の表に持つ。
//! したがって SQLite バックエンドにしか存在しない。
//!
//! 本家スキーマそのものへのピン留め (ガードテスト) は `journal_reader_impl.rs` の
//! インラインテストが持つ。ここが見るのは読み方の約束である。
//!
//! ジャーナル行を書くのは**本家のイベントストア**であり、コマンド側の Repository ではない
//! (`tests/support/mod.rs` 冒頭の理由を参照)。

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
// ジャーナル行の payload は**契約 JSON ではなくワイヤ形式そのもの**であり、行を用意する
// テストは本家のシリアライザと同じ素の serde で書く (BR1.7 の射程外)。
#![allow(clippy::disallowed_methods)]

mod support;

use core_command_domain::orchestration::{
    IntentExecution, IntentExecutionEvent, IntentExecutionEventId, Parked,
};
use core_command_domain::workflow_definition::StageSlug;
use core_command_domain::workspace::{SpaceName, StorePath};
use core_read_model_updater::orchestration::{
    CorruptCause, GlobalSeqNr, IntentExecutionEventDto, JournalBatch, JournalEntry,
    JournalReadError, JournalReader, JournalReaderImpl, ProjectionName, WorkflowDefinitionEventDto,
};
use core_read_model_updater::read_tables::ReadTables;
use rusqlite::Connection;
use tempfile::TempDir;

/// b40 のテスト用固定イベント識別子。
fn event_id() -> IntentExecutionEventId {
    IntentExecutionEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002").expect("UUIDv7")
}

use support::{
    DEFINITION_MANIFEST, JournalWriter, MANIFEST, UpstreamStore, at, defined_event, definition_id,
    execution_id, intent, open_store, other_execution_id, redefined_event, seed, seed_definition,
    seed_intent,
};

/// 行の中身に依存しない試験が前進と一緒に渡す構造化リードモデル (全表 0 行)。
fn empty_tables() -> ReadTables {
    ReadTables::project(&JournalBatch::empty()).expect("空も投影できる")
}

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
        Fixture { _dir: dir, path }
    }

    /// ジャーナル行の書き手 — 本家のイベントストアそのもの (コマンド側の Repository では
    /// ない。`support` モジュールの冒頭を参照)。
    fn store(&self) -> UpstreamStore {
        open_store(&self.path)
    }

    fn journal_reader(&self) -> JournalReaderImpl {
        JournalReaderImpl::open(&self.path).expect("Reader は開ける")
    }

    fn raw(&self) -> Connection {
        Connection::open(self.path.as_path()).expect("生の接続")
    }
}

fn projection() -> ProjectionName {
    ProjectionName::parse("state-file").expect("投影名は kebab")
}

// ---------------------------------------------------------------------------
// 差分読取 (BR1.4)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_journal_reads_every_event_in_global_order() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;

    let journal_reader = fixture.journal_reader();
    let batch = journal_reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("差分読取");
    let rows = batch.executions();
    assert_eq!(rows.len(), 4);
    let globals: Vec<u64> = rows
        .iter()
        .map(|entry| entry.global_seq().to_u64())
        .collect();
    let mut sorted = globals.clone();
    sorted.sort_unstable();
    assert_eq!(globals, sorted, "global 通番の昇順");
    assert_eq!(
        batch.scanned_to(),
        globals.last().copied().map(GlobalSeqNr::new),
        "走査済み位置は最終行"
    );
    let seqs: Vec<usize> = rows.iter().map(JournalEntry::seq_nr).collect();
    assert_eq!(seqs, [1, 2, 3, 4], "欠落なく順に読める");
    assert!(
        rows.iter()
            .all(|entry| entry.execution_id() == &execution_id()),
        "集約識別子が境界を越えて残る"
    );
    assert!(
        rows.iter().all(|entry| entry.occurred_at() == &at()),
        "発生時刻はドメイン供給値のまま往復する"
    );
}

#[tokio::test]
async fn a_row_of_the_intent_stream_is_consumed_as_an_intent() {
    // intent 自身のジャーナル (issue #50) は同じストアファイルの同じ journal 表に同居する。
    // 読取はこの行を**消費する** (issue #56) — 誕生の材料を検査付き再構成した intent として
    // 返し、実行のイベント列には混ぜない。走査済み位置は intent 行も数える。
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;
    seed_intent(&fixture.path).await;

    let journal_reader = fixture.journal_reader();
    let batch = journal_reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("intent 行は消費できる");
    assert_eq!(
        batch.executions().len(),
        4,
        "実行のイベント列には混ざらない"
    );
    assert_eq!(batch.intents(), &[intent()], "誕生の材料が集約値で返る");
    assert_eq!(
        batch.scanned_to(),
        Some(GlobalSeqNr::new(5)),
        "走査済み位置は intent 行も数える (チェックポイントが前進できる)"
    );
}

#[tokio::test]
async fn the_rows_of_the_definition_stream_are_consumed_as_definition_events() {
    // 定義のジャーナル (2026-08-31 の ES 転換) は同じストアファイルの同じ journal 表に
    // 同居する第 3 のストリームである。読取はこの行を**消費する** (b39) — 誕生と改訂を
    // ドメインイベントへ戻し、実行・intent の列には混ぜない。走査済み位置も定義行を数える。
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;
    seed_intent(&fixture.path).await;
    seed_definition(&fixture.path).await;

    let journal_reader = fixture.journal_reader();
    let batch = journal_reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("定義の行は消費できる");

    assert_eq!(batch.executions().len(), 4, "実行の列には混ざらない");
    assert_eq!(batch.intents(), &[intent()], "intent の列にも混ざらない");

    let definitions = batch.definitions();
    assert_eq!(definitions.len(), 2, "誕生と改訂の 2 行");
    assert_eq!(
        definitions[0].event(),
        &defined_event(),
        "先頭は誕生 (`Defined`)"
    );
    assert_eq!(definitions[0].seq_nr(), 1);
    assert_eq!(
        definitions[1].event(),
        &redefined_event(),
        "続くのは改訂 (`Redefined`)"
    );
    assert_eq!(definitions[1].seq_nr(), 2);
    assert!(
        definitions
            .iter()
            .all(|entry| entry.definition_id() == &definition_id()),
        "改訂は識別子を運ばないので、定義 id は行の `aid` 由来である"
    );
    assert!(
        definitions[0].global_seq() < definitions[1].global_seq(),
        "書いた順 (rowid 昇順) で返る"
    );
    assert!(
        definitions.iter().all(|entry| entry.occurred_at() == &at()),
        "発生時刻はドメイン供給値のまま往復する"
    );
    assert_eq!(
        batch.scanned_to(),
        Some(GlobalSeqNr::new(7)),
        "走査済み位置は定義行も数える (チェックポイントが前進できる)"
    );
}

#[tokio::test]
async fn a_definition_row_whose_birth_record_names_another_lineage_is_corrupt() {
    // 誕生記録だけは payload にも系譜 ID を持つ。行の `aid` と食い違う行はどちらかが嘘を
    // ついている — 解釈せず止める (intent 行と同じ規律)。
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;

    let payload =
        serde_json::to_vec(&WorkflowDefinitionEventDto::of(&defined_event())).expect("直列化");
    fixture
        .raw()
        .execute(
            "INSERT INTO journal (pkey, skey, aid, seq_nr, payload, occurred_at, manifest)
             VALUES ('WorkflowDefinition-0', 'WorkflowDefinition-kiro-1', 'kiro', 1, ?1, 0, ?2)",
            rusqlite::params![payload, DEFINITION_MANIFEST],
        )
        .expect("別系譜を名乗る定義行を差し込む");

    assert_eq!(
        fixture
            .journal_reader()
            .events_after(GlobalSeqNr::ZERO)
            .await
            .expect_err("系譜が食い違う行は解釈しない"),
        JournalReadError::Corrupt {
            aggregate_id: "kiro".to_string(),
            seq_nr: Some(1),
            cause: CorruptCause::InvariantViolation,
        }
    );
}

#[tokio::test]
async fn an_execution_row_whose_birth_record_names_another_execution_is_corrupt() {
    // 誕生記録は payload にも実行 id を持つ。行の `aid` と食い違う行はどちらかが嘘を
    // ついている — 定義行・intent 行と同じ規律で、解釈せず止める。
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;

    // payload は `execution_id()` の誕生記録、行の `aid` は `other_execution_id()`。
    let (_, started) = IntentExecution::start(execution_id(), &intent(), at());
    let payload = serde_json::to_vec(&IntentExecutionEventDto::of(&started)).expect("直列化");
    fixture
        .raw()
        .execute(
            "INSERT INTO journal (pkey, skey, aid, seq_nr, payload, occurred_at, manifest)
             VALUES ('IntentExecution-9', ?1, ?2, 1, ?3, 0, ?4)",
            rusqlite::params![
                format!("IntentExecution-{}-1", other_execution_id().as_str()),
                other_execution_id().as_str(),
                payload,
                MANIFEST
            ],
        )
        .expect("別の実行を名乗る誕生行を差し込む");

    assert_eq!(
        fixture
            .journal_reader()
            .events_after(GlobalSeqNr::ZERO)
            .await
            .expect_err("実行 id が食い違う行は解釈しない"),
        JournalReadError::Corrupt {
            aggregate_id: other_execution_id().as_str().to_string(),
            seq_nr: Some(1),
            cause: CorruptCause::InvariantViolation,
        }
    );
}

#[tokio::test]
async fn a_later_execution_row_whose_payload_names_another_execution_is_corrupt() {
    // b40 以降は genesis 以外の変種も `aggregate_id` を運ぶので、照合は**全変種**に効く。
    // 誕生後の行 (`Parked`) を別の実行の `aid` に差しても、payload が自分の実行を名乗って
    // いる限り食い違いとして止まる。
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;

    let parked = IntentExecutionEvent::Parked(Parked::new(
        event_id(),
        execution_id(),
        StageSlug::parse("intent-capture").expect("slug"),
    ));
    let payload = serde_json::to_vec(&IntentExecutionEventDto::of(&parked)).expect("直列化");
    fixture
        .raw()
        .execute(
            "INSERT INTO journal (pkey, skey, aid, seq_nr, payload, occurred_at, manifest)
             VALUES ('IntentExecution-9', ?1, ?2, 2, ?3, 0, ?4)",
            rusqlite::params![
                format!("IntentExecution-{}-2", other_execution_id().as_str()),
                other_execution_id().as_str(),
                payload,
                MANIFEST
            ],
        )
        .expect("別の実行を名乗る park 行を差し込む");

    assert_eq!(
        fixture
            .journal_reader()
            .events_after(GlobalSeqNr::ZERO)
            .await
            .expect_err("実行 id が食い違う行は解釈しない"),
        JournalReadError::Corrupt {
            aggregate_id: other_execution_id().as_str().to_string(),
            seq_nr: Some(2),
            cause: CorruptCause::InvariantViolation,
        }
    );
}

#[tokio::test]
async fn a_revision_row_whose_lineage_disagrees_with_the_row_is_corrupt() {
    // b40 以降、改訂も系譜 ID を `aggregate_id` として運ぶ (イベントはエンティティ) ので、
    // 行の `aid` と照合する相手ができた。別系譜の `aid` に差した改訂行はどちらかが嘘を
    // ついている — 解釈せず `Corrupt` で止める (誕生行・intent 行と同じ規律)。かつては
    // 「改訂は照合相手が無いので行の `aid` が正」という片肺だった。
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;

    let payload =
        serde_json::to_vec(&WorkflowDefinitionEventDto::of(&redefined_event())).expect("直列化");
    fixture
        .raw()
        .execute(
            "INSERT INTO journal (pkey, skey, aid, seq_nr, payload, occurred_at, manifest)
             VALUES ('WorkflowDefinition-0', 'WorkflowDefinition-kiro-2', 'kiro', 2, ?1, 0, ?2)",
            rusqlite::params![payload, DEFINITION_MANIFEST],
        )
        .expect("別系譜の改訂行を差し込む");

    let failure = fixture
        .journal_reader()
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect_err("行と payload が別の系譜を語る");
    assert!(
        matches!(
            failure,
            JournalReadError::Corrupt {
                cause: CorruptCause::InvariantViolation,
                ..
            }
        ),
        "照合の食い違いは破損として止まる: {failure:?}"
    );
}

#[tokio::test]
async fn an_undecodable_intent_row_is_corrupt() {
    // intent 行も検査を通る — 形にならない payload は読み飛ばさず破損として止める。
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;

    fixture
        .raw()
        .execute(
            "INSERT INTO journal (pkey, skey, aid, seq_nr, payload, occurred_at, manifest)              VALUES ('Intent-0', 'Intent-018f3b2c-4d5e-7f60-8abc-def012345678-1',              '018f3b2c-4d5e-7f60-8abc-def012345678', 1, X'7B7D', 0, 'intent-event/1')",
            [],
        )
        .expect("intent ストリームの行を差し込む");

    let journal_reader = fixture.journal_reader();
    let err = journal_reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect_err("形にならない intent 行は破損");
    assert!(matches!(
        err,
        JournalReadError::Corrupt {
            cause: CorruptCause::UndecodablePayload,
            ..
        }
    ));
}

#[tokio::test]
async fn the_journal_reads_only_the_difference() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;

    let journal_reader = fixture.journal_reader();
    let all = journal_reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("全件");
    let second = all.executions().get(1).expect("2 件目").global_seq();
    let batch = journal_reader.events_after(second).await.expect("差分");
    assert_eq!(batch.executions().len(), 2);
    assert!(
        batch
            .executions()
            .iter()
            .all(|entry| entry.global_seq() > second)
    );
}

#[tokio::test]
async fn a_renumbered_journal_is_refused_by_the_anchor() {
    // rowid の振り直し (仕様が許す再番号付け) が起きると、保存済みチェックポイントの
    // アンカー (aid, seq_nr) が journal の同 rowid と食い違う。差分読取の欠落・重複という
    // 静かな破損ではなく、明示エラーで止まることを検証する。
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;
    drop(store);

    let mut journal_reader = fixture.journal_reader();
    let before = journal_reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("全件");
    let third = before.executions().get(2).expect("3 件目").global_seq();
    journal_reader
        .advance_checkpoint(&projection(), third, &empty_tables())
        .await
        .expect("チェックポイント前進");
    drop(journal_reader);

    // SQLite の仕様は「INTEGER PRIMARY KEY の無い表の rowid を VACUUM が変えてよい」と
    // している (現行 3.51 は隙間があっても保持する — 下の VACUUM 釘留めテスト参照)。
    // ここでは仕様が許す振り直しそのもの (先頭行の削除 + 後続の繰り上げ) を直接再現する。
    let conn = fixture.raw();
    conn.execute("DELETE FROM journal WHERE rowid = 1", [])
        .expect("先頭行を消して隙間を作る");
    for old_rowid in 2i64..=4 {
        conn.execute(
            "UPDATE journal SET rowid = ?1 WHERE rowid = ?2",
            rusqlite::params![old_rowid - 1, old_rowid],
        )
        .expect("rowid を繰り上げる");
    }
    drop(conn);

    let journal_reader = fixture.journal_reader();
    let error = journal_reader
        .checkpoint(&projection())
        .await
        .expect_err("アンカー照合で止まる");
    assert!(
        matches!(
            error,
            JournalReadError::Corrupt {
                cause: CorruptCause::CheckpointAnchorMismatch,
                ..
            }
        ),
        "実際: {error:?}"
    );
}

#[tokio::test]
async fn a_truncated_journal_behind_the_checkpoint_is_refused() {
    // チェックポイントが指す行そのものが消えている場合もアンカー照合で止まる。
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;
    drop(store);

    let mut journal_reader = fixture.journal_reader();
    let all = journal_reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("全件");
    let last = all.executions().last().expect("末尾").global_seq();
    journal_reader
        .advance_checkpoint(&projection(), last, &empty_tables())
        .await
        .expect("チェックポイント前進");
    drop(journal_reader);

    fixture
        .raw()
        .execute("DELETE FROM journal WHERE rowid >= 4", [])
        .expect("末尾を切り落とす");

    let journal_reader = fixture.journal_reader();
    let error = journal_reader
        .checkpoint(&projection())
        .await
        .expect_err("指し先が無い");
    assert!(
        matches!(
            error,
            JournalReadError::Corrupt {
                cause: CorruptCause::CheckpointAnchorMismatch,
                ..
            }
        ),
        "実際: {error:?}"
    );
}

#[tokio::test]
async fn a_vacuum_rebuild_does_not_move_the_cursor() {
    // journal は削除ゼロの純追記 (DELETE は本家 v2.0.0 でも snapshot 表にしか無い) なので、
    // rowid は隙間の無い連番 1..N であり、VACUUM の再構築でも値が保たれる。ここはその前提を
    // 実挙動で釘留めする回帰テスト — 破れたらアンカー照合 (上の 2 テスト) が実行時に
    // 明示エラーで止める。
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;
    drop(store);

    let mut journal_reader = fixture.journal_reader();
    let before = journal_reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("全件");
    let third = before.executions().get(2).expect("3 件目").global_seq();
    journal_reader
        .advance_checkpoint(&projection(), third, &empty_tables())
        .await
        .expect("チェックポイント前進");
    drop(journal_reader); // VACUUM は排他を要するため他接続を全部閉じてから実行する

    fixture
        .raw()
        .execute_batch("VACUUM")
        .expect("VACUUM は通る");

    let journal_reader = fixture.journal_reader();
    let after = journal_reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("全件");
    let keys = |rows: &[JournalEntry]| -> Vec<u64> {
        rows.iter()
            .map(|entry| entry.global_seq().to_u64())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        keys(after.executions()),
        keys(before.executions()),
        "rowid は VACUUM 前と同一"
    );
    let saved = journal_reader
        .checkpoint(&projection())
        .await
        .expect("保存済み");
    assert_eq!(saved, third, "チェックポイントも生きている");
    let resumed = journal_reader.events_after(saved).await.expect("差分");
    assert_eq!(
        keys(resumed.executions()),
        keys(before.executions())[3..].to_vec(),
        "続行が欠落も重複もしない"
    );
}

#[tokio::test]
async fn the_journal_interleaves_two_aggregates_in_commit_order() {
    // 本家のイベントストアは集約単位でしか読めない。横断のカーソルが「コミット順」で
    // あることこそ、この実装を自前で持つ理由である (ADR-010 決定 4)。
    let fixture = Fixture::new();
    let mut store = fixture.store();

    let mut first = JournalWriter::start(&mut store, execution_id()).await;
    JournalWriter::start(&mut store, other_execution_id()).await;
    first
        .advance(&mut store, |aggregate| {
            // 誕生 = 初期化完了済み (issue #76) なので、genesis の次に打てるのは
            // カーソル (索引 1 のゲート付きステージ) の開放である。
            aggregate.open_gate(&intent(), vec!["intent.md".to_string()], at())
        })
        .await;

    let journal_reader = fixture.journal_reader();
    let rows = journal_reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("全件");
    assert_eq!(
        rows.executions()
            .iter()
            .map(|entry| (
                entry.global_seq().to_u64(),
                entry.execution_id().as_str().to_string()
            ))
            .collect::<Vec<_>>(),
        [
            (1, support::EXECUTION.to_string()),
            (2, support::OTHER_INTENT.to_string()),
            (3, support::EXECUTION.to_string()),
        ],
        "書いた順に 1 本の列へ並ぶ"
    );
}

#[tokio::test]
async fn the_reader_observes_writes_made_after_it_was_opened() {
    // Reader は同じファイルへの**生きた接続**である (写しではない)。
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let mut writer = JournalWriter::start(&mut store, execution_id()).await;

    let journal_reader = fixture.journal_reader();
    writer
        .advance(&mut store, |aggregate| {
            // 誕生 = 初期化完了済み (issue #76) なので、genesis の次に打てるのは
            // カーソル (索引 1 のゲート付きステージ) の開放である。
            aggregate.open_gate(&intent(), vec!["intent.md".to_string()], at())
        })
        .await;

    let rows = journal_reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("全件");
    assert_eq!(rows.executions().len(), 2);
}

#[tokio::test]
async fn a_tampered_journal_payload_is_corrupt() {
    // 行を壊せるのは生の SQL だけである (実装に破壊用のフックを開けない — BR2.8)。
    let fixture = Fixture::new();
    let mut store = fixture.store();
    JournalWriter::start(&mut store, execution_id()).await;
    fixture
        .raw()
        .execute("UPDATE journal SET payload = X'7B6E6F74206A736F6E'", [])
        .expect("payload を壊す");

    let journal_reader = fixture.journal_reader();
    let err = journal_reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect_err("復号できない");
    assert_eq!(
        err,
        JournalReadError::Corrupt {
            aggregate_id: support::EXECUTION.to_string(),
            seq_nr: None,
            cause: CorruptCause::UndecodablePayload,
        }
    );
}

// ---------------------------------------------------------------------------
// チェックポイント (BR1.4)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn an_unregistered_projection_reads_as_zero() {
    let fixture = Fixture::new();
    let _store = fixture.store();
    let journal_reader = fixture.journal_reader();
    assert_eq!(
        journal_reader
            .checkpoint(&projection())
            .await
            .expect("読取"),
        GlobalSeqNr::ZERO
    );
}

#[tokio::test]
async fn the_checkpoint_advances_and_repeats_are_noops() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;

    let mut journal_reader = fixture.journal_reader();
    let rows = journal_reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("全件");
    let last = rows.executions().last().expect("4 件ある").global_seq();

    journal_reader
        .advance_checkpoint(&projection(), last, &empty_tables())
        .await
        .expect("前進");
    assert_eq!(
        journal_reader
            .checkpoint(&projection())
            .await
            .expect("読取"),
        last
    );

    journal_reader
        .advance_checkpoint(&projection(), last, &empty_tables())
        .await
        .expect("同値は no-op");
    assert_eq!(
        journal_reader
            .checkpoint(&projection())
            .await
            .expect("読取"),
        last
    );
    assert_eq!(
        journal_reader
            .events_after(last)
            .await
            .expect("差分")
            .scanned_to(),
        None,
        "追いついた投影に差分は無い"
    );
}

#[tokio::test]
async fn the_checkpoint_survives_reopening_the_store() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;
    {
        let mut journal_reader = fixture.journal_reader();
        journal_reader
            .advance_checkpoint(&projection(), GlobalSeqNr::new(3), &empty_tables())
            .await
            .expect("前進");
    }

    let journal_reader = fixture.journal_reader();
    assert_eq!(
        journal_reader
            .checkpoint(&projection())
            .await
            .expect("読取"),
        GlobalSeqNr::new(3),
        "自前の表はファイルに残る"
    );
}

#[tokio::test]
async fn a_checkpoint_regression_is_refused() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;

    let mut journal_reader = fixture.journal_reader();
    journal_reader
        .advance_checkpoint(&projection(), GlobalSeqNr::new(3), &empty_tables())
        .await
        .expect("前進");
    let err = journal_reader
        .advance_checkpoint(&projection(), GlobalSeqNr::new(2), &empty_tables())
        .await
        .expect_err("後退は拒否");
    assert_eq!(
        err,
        JournalReadError::CheckpointRegression {
            projection: projection(),
            current: GlobalSeqNr::new(3),
            requested: GlobalSeqNr::new(2),
        }
    );
    assert_eq!(
        journal_reader
            .checkpoint(&projection())
            .await
            .expect("読取"),
        GlobalSeqNr::new(3),
        "拒否しても現在値は動かない"
    );
}

#[tokio::test]
async fn two_projections_keep_independent_checkpoints() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;

    let other = ProjectionName::parse("intents-registry").expect("投影名は kebab");
    let mut journal_reader = fixture.journal_reader();
    journal_reader
        .advance_checkpoint(&projection(), GlobalSeqNr::new(4), &empty_tables())
        .await
        .expect("前進");
    assert_eq!(
        journal_reader.checkpoint(&other).await.expect("読取"),
        GlobalSeqNr::ZERO,
        "別の投影は動かない"
    );
    journal_reader
        .advance_checkpoint(&other, GlobalSeqNr::new(2), &empty_tables())
        .await
        .expect("前進");
    assert_eq!(
        journal_reader
            .checkpoint(&projection())
            .await
            .expect("読取"),
        GlobalSeqNr::new(4)
    );
    assert_eq!(
        journal_reader.checkpoint(&other).await.expect("読取"),
        GlobalSeqNr::new(2)
    );
}

// ---------------------------------------------------------------------------
// 構造化リードモデル (`read_*` 表) — 行の往復と、前進との原子性 (b39 / 裁定 §3)
// ---------------------------------------------------------------------------

/// 実行・intent・定義の 3 ストリームを 1 本の履歴として読み直し、行へ投影する。
async fn seeded_tables(journal_reader: &JournalReaderImpl) -> ReadTables {
    let history = journal_reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("全件");
    ReadTables::project(&history).expect("健全な履歴は投影できる")
}

#[tokio::test]
async fn opening_the_store_twice_leaves_the_read_tables_intact() {
    // DDL は `CREATE TABLE IF NOT EXISTS` — 2 度目の open で落ちないし、行も消さない。
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed_intent(&fixture.path).await;
    seed(&mut store).await;

    let mut journal_reader = fixture.journal_reader();
    let tables = seeded_tables(&journal_reader).await;
    let last = tables.as_of().expect("走査位置");
    journal_reader
        .advance_checkpoint(&projection(), last, &tables)
        .await
        .expect("前進");

    let reopened = fixture.journal_reader();
    assert_eq!(
        reopened.checkpoint(&projection()).await.expect("読取"),
        last
    );
    let rows: i64 = fixture
        .raw()
        .query_row("SELECT COUNT(*) FROM read_execution", [], |row| row.get(0))
        .expect("表は残っている");
    assert_eq!(rows, 1, "2 度目の open は行を消さない");
}

#[tokio::test]
async fn the_rows_come_back_out_of_sqlite_exactly_as_they_were_projected() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed_intent(&fixture.path).await;
    seed_definition(&fixture.path).await;
    seed(&mut store).await;

    let mut journal_reader = fixture.journal_reader();
    let tables = seeded_tables(&journal_reader).await;
    let last = tables.as_of().expect("走査位置");
    journal_reader
        .advance_checkpoint(&projection(), last, &tables)
        .await
        .expect("前進");

    let connection = fixture.raw();
    let as_of = i64::try_from(last.to_u64()).expect("i64 に収まる");

    // read_execution — 実行 1 本の代表列。`scope` を含めるのは、行型が持つ値が DDL と
    // INSERT の両方に届いていることを見るためである (どちらかが欠けると、投影は正しいのに
    // 引き当てた行だけが黙って値を失う)。
    let expected = tables.executions().first().expect("実行が 1 本");
    let (execution_id, scope, status, cursor_slug, autonomy, seq_nr, stamp): (
        String,
        String,
        String,
        Option<String>,
        String,
        i64,
        i64,
    ) = connection
        .query_row(
            "SELECT id, scope, status, cursor_slug, autonomy, seq_nr, as_of
             FROM read_execution",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .expect("行は 1 件");
    assert_eq!(execution_id, expected.id());
    assert_eq!(
        scope,
        expected.scope(),
        "intent から非正規化した scope が届く"
    );
    assert_eq!(status, expected.status());
    assert_eq!(cursor_slug.as_deref(), expected.cursor_slug());
    assert_eq!(autonomy, expected.autonomy());
    assert_eq!(usize::try_from(seq_nr).expect("非負"), expected.seq_nr());
    assert_eq!(stamp, as_of, "as_of は走査済み最終位置");

    // read_next_answer — 4 kind そろっている。
    let kinds: Vec<String> = connection
        .prepare("SELECT request_kind FROM read_next_answer ORDER BY request_kind")
        .expect("準備")
        .query_map([], |row| row.get(0))
        .expect("問い合わせ")
        .collect::<Result<Vec<String>, _>>()
        .expect("収集");
    assert_eq!(kinds, ["bare", "free-text", "reentry", "resume"]);

    // read_definition_stage — 定義ストリームの改訂まで畳んだ結果が行になっている。
    let (slug, phase, gated): (String, String, bool) = connection
        .query_row(
            "SELECT stage_slug, phase, gated FROM read_definition_stage",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("行は 1 件");
    let expected_stage = tables.definition_stages().first().expect("ステージ 1 件");
    assert_eq!(slug, expected_stage.stage_slug());
    assert_eq!(phase, expected_stage.phase());
    assert_eq!(gated, expected_stage.gated());
}

/// ジャーナル由来 15 表の名前と、その表に行が在ることを確かめる引き当て。
///
/// 表ごとの INSERT が失敗を握り潰していないかを見るので、**その表に行が 1 件も無ければ
/// 試験が空振りする**。行数を一緒に持って、空振りをテスト自身が検出できるようにする。
fn read_table_rows(tables: &ReadTables) -> [(&'static str, usize); 15] {
    [
        ("read_definition", tables.definitions().len()),
        ("read_definition_stage", tables.definition_stages().len()),
        ("read_definition_scope", tables.definition_scopes().len()),
        (
            "read_definition_scope_keyword",
            tables.definition_scope_keywords().len(),
        ),
        (
            "read_definition_scope_stage",
            tables.definition_scope_stages().len(),
        ),
        (
            "read_definition_scope_phase_entry",
            tables.definition_scope_phase_entries().len(),
        ),
        ("read_intent", tables.intents().len()),
        ("read_intent_stage", tables.intent_stages().len()),
        ("read_execution", tables.executions().len()),
        ("read_execution_stage", tables.execution_stages().len()),
        ("read_next_answer", tables.next_answers().len()),
        ("read_next_jump", tables.next_jumps().len()),
        ("read_next_jump_phase", tables.next_jump_phases().len()),
        ("read_run_stage", tables.run_stages().len()),
        ("read_scope_change", tables.scope_changes().len()),
    ]
}

#[tokio::test]
async fn the_written_rows_carry_their_primary_key_and_resolve_their_foreign_keys() {
    // 行型が id と FK を持っていても、`INSERT` の列に届いていなければ SQLite の側は空の
    // ままである。ここは**書かれた行**を SQL で見る — 主キーが空でないこと、FK の指す先が
    // 同じスナップショットに実在することの 2 つを確かめる。
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed_intent(&fixture.path).await;
    seed_definition(&fixture.path).await;
    seed(&mut store).await;

    let mut journal_reader = fixture.journal_reader();
    let tables = seeded_tables(&journal_reader).await;
    let last = tables.as_of().expect("走査位置");
    journal_reader
        .advance_checkpoint(&projection(), last, &tables)
        .await
        .expect("前進");

    let connection = fixture.raw();
    for (table, rows) in read_table_rows(&tables) {
        assert!(rows > 0, "{table} に行が無いと試験が空振りする");
        let blank: i64 = connection
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE id IS NULL OR id = ''"),
                [],
                |row| row.get(0),
            )
            .expect("引ける");
        assert_eq!(blank, 0, "{table} に主キーの空の行がある");
    }

    // 親を持つ表は、その親が同じスナップショットに居る。
    for (child, foreign_key, parent) in [
        ("read_definition_stage", "definition_id", "read_definition"),
        ("read_definition_scope", "definition_id", "read_definition"),
        (
            "read_definition_scope_keyword",
            "definition_id",
            "read_definition",
        ),
        (
            "read_definition_scope_stage",
            "definition_id",
            "read_definition",
        ),
        (
            "read_definition_scope_phase_entry",
            "definition_id",
            "read_definition",
        ),
        ("read_run_stage", "definition_id", "read_definition"),
        ("read_intent", "definition_id", "read_definition"),
        ("read_intent_stage", "intent_id", "read_intent"),
        ("read_execution", "intent_id", "read_intent"),
        ("read_execution_stage", "execution_id", "read_execution"),
        ("read_next_answer", "execution_id", "read_execution"),
        ("read_next_jump", "execution_id", "read_execution"),
        ("read_next_jump_phase", "execution_id", "read_execution"),
        ("read_scope_change", "execution_id", "read_execution"),
        ("read_next_answer", "run_stage_id", "read_run_stage"),
    ] {
        let orphans: i64 = connection
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM {child}
                     WHERE {foreign_key} IS NOT NULL
                       AND {foreign_key} NOT IN (SELECT id FROM {parent})"
                ),
                [],
                |row| row.get(0),
            )
            .expect("引ける");
        assert_eq!(orphans, 0, "{child}.{foreign_key} が {parent} に届かない");
    }

    // この fixture の定義グラフは `state-init` 1 ノードだが、実行の計画は 3 ステージを
    // 名乗る (計画は誕生時の内容版に対して解決されるので、その後に痩せた定義とはずれる)。
    // したがって材料の行が無い run-stage の答えが在り、その FK は**届かない値ではなく
    // NULL** でなければならない。届く FK が書かれる側は行の契約テスト
    // (`read_tables_test`) が固定する。
    let run_stage_answers: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM read_next_answer WHERE decision_kind = 'run-stage'",
            [],
            |row| row.get(0),
        )
        .expect("引ける");
    assert!(
        run_stage_answers > 0,
        "run-stage の答えが 1 行も無く、FK の検査が空振りする"
    );
    let dangling: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM read_next_answer
             WHERE decision_kind = 'run-stage' AND run_stage_id IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .expect("引ける");
    assert_eq!(
        dangling, 0,
        "材料の行が無いのに FK が書かれている (この定義に run-stage の行は無い)"
    );
}

#[tokio::test]
async fn a_read_table_whose_shape_drifted_fails_the_advance_instead_of_writing_partial_rows() {
    // 全差し替えは 15 表への INSERT の並びである。どの 1 本も失敗を握り潰してはならない
    // — 握り潰せば「他の表は新しく、その表だけ空」という嘘の断面が残る。表を 1 つずつ
    // 列の合わない形へ作り替えて、前進ごと失敗し行が 1 つも動かないことを見る。
    for (table, rows) in read_table_rows(&{
        let fixture = Fixture::new();
        let mut store = fixture.store();
        seed_intent(&fixture.path).await;
        seed_definition(&fixture.path).await;
        seed(&mut store).await;
        seeded_tables(&fixture.journal_reader()).await
    }) {
        assert!(rows > 0, "{table} に行が無いと試験が空振りする");

        let fixture = Fixture::new();
        let mut store = fixture.store();
        seed_intent(&fixture.path).await;
        seed_definition(&fixture.path).await;
        seed(&mut store).await;

        let mut journal_reader = fixture.journal_reader();
        let tables = seeded_tables(&journal_reader).await;
        let last = tables.as_of().expect("走査位置");

        // `DELETE` は通るが `INSERT` の列が解決できない形へ作り替える。
        fixture
            .raw()
            .execute_batch(&format!(
                "DROP TABLE {table}; CREATE TABLE {table} (definition_id TEXT)"
            ))
            .expect("表を作り替える");

        let error = journal_reader
            .advance_checkpoint(&projection(), last, &tables)
            .await
            .expect_err("列の合わない表への INSERT は失敗する");
        assert!(
            matches!(error, JournalReadError::Io { .. }),
            "{table}: SQLite の失敗は I/O の失敗として上がる (実際: {error:?})"
        );

        // 同じ Tx なのでチェックポイントも前進していない (裁定 §3)。
        assert_eq!(
            journal_reader
                .checkpoint(&projection())
                .await
                .expect("読取"),
            GlobalSeqNr::ZERO,
            "{table}: 失敗した前進はチェックポイントを動かさない"
        );
    }
}

#[tokio::test]
async fn a_refused_advance_leaves_the_rows_untouched() {
    // 行の差し替えと前進は 1 Tx である — 後退が拒否されたら行も 1 つも動かない (裁定 §3)。
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed_intent(&fixture.path).await;
    seed(&mut store).await;

    let mut journal_reader = fixture.journal_reader();
    let tables = seeded_tables(&journal_reader).await;
    let last = tables.as_of().expect("走査位置");
    journal_reader
        .advance_checkpoint(&projection(), last, &tables)
        .await
        .expect("前進");

    let before: i64 = fixture
        .raw()
        .query_row("SELECT COUNT(*) FROM read_next_jump", [], |row| row.get(0))
        .expect("件数");
    assert!(before > 0, "前進で行が入っている");

    // 後退を、**行が空の**リードモデルと一緒に要求する。拒否されるので行は消えない。
    let err = journal_reader
        .advance_checkpoint(&projection(), GlobalSeqNr::new(1), &empty_tables())
        .await
        .expect_err("後退は拒否");
    assert!(matches!(err, JournalReadError::CheckpointRegression { .. }));

    let after: i64 = fixture
        .raw()
        .query_row("SELECT COUNT(*) FROM read_next_jump", [], |row| row.get(0))
        .expect("件数");
    assert_eq!(after, before, "拒否した Tx は行を 1 つも消していない");
}

#[tokio::test]
async fn a_second_advance_replaces_every_row_instead_of_appending() {
    // 投影は全再計算・全差し替えである — 2 度書いても行は増えない。
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed_intent(&fixture.path).await;
    seed(&mut store).await;

    let mut journal_reader = fixture.journal_reader();
    let tables = seeded_tables(&journal_reader).await;
    let last = tables.as_of().expect("走査位置");
    journal_reader
        .advance_checkpoint(&projection(), last, &tables)
        .await
        .expect("前進");
    let first: i64 = fixture
        .raw()
        .query_row("SELECT COUNT(*) FROM read_execution_stage", [], |row| {
            row.get(0)
        })
        .expect("件数");

    journal_reader
        .advance_checkpoint(&projection(), last, &tables)
        .await
        .expect("同値は no-op だが行は書き直す");
    let second: i64 = fixture
        .raw()
        .query_row("SELECT COUNT(*) FROM read_execution_stage", [], |row| {
            row.get(0)
        })
        .expect("件数");
    assert_eq!(second, first, "差し替えであって追記ではない");
}

// ---------------------------------------------------------------------------
// 読み面スキーマの版管理 (PR #103 レビュー指摘)
// ---------------------------------------------------------------------------

/// `read_next_answer` を b47 以前の形 (`gated INTEGER`) に戻し、版も戻す。
///
/// 実 CLI からは作れない断面なので、ここだけストアへ直接 DDL を打つ。旧スキーマのまま
/// `CREATE TABLE IF NOT EXISTS` が走ると表は旧いまま残り、`INSERT` が `no such column` で
/// 落ちる — それが版管理の無い世界で起きることである。
fn regress_to_the_old_schema(fixture: &Fixture) {
    fixture
        .raw()
        .execute_batch(
            "DROP TABLE read_next_answer;
             CREATE TABLE read_next_answer (
               id            TEXT    PRIMARY KEY,
               execution_id  TEXT    NOT NULL,
               request_kind  TEXT    NOT NULL,
               decision_kind TEXT    NOT NULL,
               stage_index   INTEGER,
               stage_slug    TEXT,
               gated         INTEGER,
               checkbox      TEXT,
               run_stage_id  TEXT,
               as_of         INTEGER NOT NULL
             );
             PRAGMA user_version = 0;",
        )
        .expect("旧スキーマへ戻せる");
}

/// 保存済みの版が違えば、17 表を作り直してジャーナルから描き直す。
#[tokio::test]
async fn a_store_left_on_the_old_read_schema_is_rebuilt_from_the_journal() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed_intent(&fixture.path).await;
    seed_definition(&fixture.path).await;
    seed(&mut store).await;

    // いったん現行スキーマで投影しておく (= チェックポイントが進んだストアにする)。
    let mut journal_reader = fixture.journal_reader();
    let tables = seeded_tables(&journal_reader).await;
    let last = tables.as_of().expect("走査位置");
    journal_reader
        .advance_checkpoint(&projection(), last, &tables)
        .await
        .expect("前進");
    let projected: i64 = fixture
        .raw()
        .query_row("SELECT COUNT(*) FROM read_next_answer", [], |row| {
            row.get(0)
        })
        .expect("件数");
    assert!(projected > 0, "前提: 現行スキーマでは行が入る");
    drop(journal_reader);

    // 旧スキーマへ戻す — この形のまま INSERT すると `no such column: gate` で落ちる。
    regress_to_the_old_schema(&fixture);

    // 取得ループの入口 (= 開く段) が版の差を見て作り直す。
    let journal_reader = fixture.journal_reader();
    drop(journal_reader);

    let raw = fixture.raw();
    let version: i64 = raw
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("版");
    assert_eq!(version, 1, "作り直したら版を記録する");
    let rebuilt: i64 = raw
        .query_row("SELECT COUNT(*) FROM read_next_answer", [], |row| {
            row.get(0)
        })
        .expect("件数");
    assert_eq!(rebuilt, projected, "ジャーナルから同じ行が描き直される");
    let gate: i64 = raw
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('read_next_answer') WHERE name = 'gate'",
            [],
            |row| row.get(0),
        )
        .expect("列");
    assert_eq!(gate, 1, "新しい列名で作り直されている");
}

/// 版が一致していれば 1 表も落とさない (毎回の作り直しは行を捨てる無駄である)。
#[tokio::test]
async fn a_store_already_on_the_current_read_schema_is_not_dropped() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed_intent(&fixture.path).await;
    seed(&mut store).await;

    let mut journal_reader = fixture.journal_reader();
    let tables = seeded_tables(&journal_reader).await;
    let last = tables.as_of().expect("走査位置");
    journal_reader
        .advance_checkpoint(&projection(), last, &tables)
        .await
        .expect("前進");
    drop(journal_reader);
    // 投影が書かない印を 1 行置く — 落として作り直せば消える。
    fixture
        .raw()
        .execute(
            "INSERT INTO read_scope_change(id, execution_id, scope, kind, as_of)
             VALUES ('sentinel', 'e', 's', 'k', 0)",
            [],
        )
        .expect("印を置ける");

    let journal_reader = fixture.journal_reader();
    drop(journal_reader);

    let survived: i64 = fixture
        .raw()
        .query_row(
            "SELECT COUNT(*) FROM read_scope_change WHERE id = 'sentinel'",
            [],
            |row| row.get(0),
        )
        .expect("件数");
    assert_eq!(survived, 1, "版が同じなら表は落とさない");
}

/// 作り直しの途中で歴史が読めなければ、版を上げずに止める。
///
/// 版だけ先に上げてしまうと、次の起動は「もう作り直した」と判断して旧い読み面のまま
/// 進んでしまう。壊れた材料は作り直しでも直らないので、開く段でそのまま倒れる。
#[tokio::test]
async fn a_rebuild_that_cannot_read_the_history_stops_without_bumping_the_version() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed_intent(&fixture.path).await;
    seed(&mut store).await;

    // チェックポイントを進めておく (作り直しの対象になるストアにする)。
    let mut journal_reader = fixture.journal_reader();
    let tables = seeded_tables(&journal_reader).await;
    let last = tables.as_of().expect("走査位置");
    journal_reader
        .advance_checkpoint(&projection(), last, &tables)
        .await
        .expect("前進");
    drop(journal_reader);

    regress_to_the_old_schema(&fixture);
    // 版を戻したうえで歴史も壊す — 作り直しは読取の段で倒れる。
    fixture
        .raw()
        .execute("UPDATE journal SET payload = X'7B6E6F74206A736F6E'", [])
        .expect("payload を壊す");

    let error = JournalReaderImpl::open(&fixture.path).expect_err("壊れた歴史は描き直せない");
    assert!(
        matches!(
            error,
            JournalReadError::Corrupt {
                cause: CorruptCause::UndecodablePayload,
                ..
            }
        ),
        "{error:?}"
    );
    let version: i64 = fixture
        .raw()
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("版");
    assert_eq!(version, 0, "描き直せていないなら版も上げない");
}
