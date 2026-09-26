//! 構造化面の更新器の契約 — ジャーナル由来の `read_*` 20 表と処理したシーケンス番号。
//!
//! 形は「ジャーナルを読む → 投影 (`ReadTables::project`) → 表の DAO で書く」である
//! (`coding-rules/read-model-updater-structure.md`)。ここでは実ストア (本家のイベントストアが
//! 書いた履歴) の上で次を固定する (Issue #153 の PR4):
//!
//! - 処理したシーケンス番号 (ジャーナル上の位置) の次から再開し、新しい事実が無ければ何も
//!   書かない (共有面の記録の世代も動かない)
//! - 保存したアンカーがジャーナルの行と食い違えば、読み進めずに止める
//! - 途中で失敗すれば、20 表・共有面の記録・番号のどれも動かない (1 つの IMMEDIATE
//!   トランザクションで確定するため)
//! - 別の接続が書込ロックを握っている間は、IMMEDIATE で待ってから書く (#134)
//! - 開く段は、表が揃っていれば書込ロックを取らない (構造化面の更新器もジャーナルの読み手も)
//! - 共有面の記録のダイジェストは、分ける前 (`read_tables::content_digest`) と同じ形式で計算する
//!
//! 20 表の DDL と書込の表駆動の単体試験は `src/orchestration/structured_surface_tables_tests.rs`、
//! 前進とアンカー照合の単体試験は `src/orchestration/structured_surface.rs` にある。

// テストコードでは unwrap / expect / panic を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use std::path::Path;
use std::sync::mpsc::{Sender, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use core_command_domain::workspace::{SpaceName, StorePath};
use core_read_model_updater::orchestration::{
    CorruptCause, GlobalSeqNr, JournalReadError, JournalReader as _, JournalReaderImpl,
    ProjectionName, ReadModelUpdateError, ReadModelUpdater as _, StructuredReadModelUpdater,
};
use core_read_model_updater::read_tables::ReadTables;
use rusqlite::Connection;
use tempfile::TempDir;

use support::{JournalWriter, open_store, other_execution_id, seed, seed_definition, seed_intent};

/// ロックを握る側が保持する時間。
const HOLD: Duration = Duration::from_millis(200);

/// 呼び出しがロック待ちを実際に観測したと言える所要時間の下限 (HOLD の半分)。
const MIN_OBSERVED_WAIT: Duration = Duration::from_millis(100);

/// 開く段が書込ロックを待ったと見なす所要時間 (busy timeout 5000ms より十分短い)。
const PROMPT_OPEN: Duration = Duration::from_millis(1000);

/// 構造化面の 20 表 (内容の照合に含める並び)。
const TABLES: [&str; 20] = [
    "read_session_audit",
    "read_artifact_audit",
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
];

/// ダイジェストの材料 (表 → 行 → 値の (格納型の印, バイト列))。
type DigestMaterial = Vec<Vec<Vec<(u8, Vec<u8>)>>>;

/// 定義・intent・実行 1 本の履歴を持つ実ストア。
struct Fixture {
    _dir: TempDir,
    path: StorePath,
}

impl Fixture {
    async fn seeded() -> Fixture {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let path = StorePath::for_space(&dir.path().join("aidlc"), &SpaceName::default());
        std::fs::create_dir_all(path.as_path().parent().expect("親 dir を持つ"))
            .expect("intents/ を先に作る");
        seed_intent(&path).await;
        seed_definition(&path).await;
        seed(&mut open_store(&path)).await;
        Fixture { _dir: dir, path }
    }

    fn store(&self) -> &Path {
        self.path.as_path()
    }

    fn raw(&self) -> Connection {
        Connection::open(self.store()).expect("生の接続")
    }

    fn updater(&self) -> StructuredReadModelUpdater {
        StructuredReadModelUpdater::open(self.store())
            .expect("開ける")
            .for_projection(projection())
    }

    async fn update(&self) -> Result<(), ReadModelUpdateError> {
        self.updater().update_read_models().await
    }

    fn checkpoint(&self) -> Result<GlobalSeqNr, ReadModelUpdateError> {
        self.updater().checkpoint(&projection())
    }

    /// ジャーナルの最後の位置と、全履歴の投影。
    async fn history(&self) -> (GlobalSeqNr, ReadTables) {
        let history = JournalReaderImpl::open(&self.path)
            .expect("Reader は開ける")
            .events_after(GlobalSeqNr::ZERO)
            .await
            .expect("全件");
        (
            history.scanned_to().expect("履歴あり"),
            ReadTables::project(&history).expect("投影"),
        )
    }

    /// 実行をもう 1 本始める (新しい事実を足す)。
    async fn another_execution(&self) {
        let mut store = open_store(&self.path);
        drop(JournalWriter::start(&mut store, other_execution_id()).await);
    }

    /// 共有面の記録 (位置・世代・変換・ダイジェスト・照合済みか)。
    fn head(&self) -> (i64, i64, String, String, bool) {
        self.raw()
            .query_row(
                "SELECT position, generation, revision, content_digest, verified
                 FROM amadeus_read_model_head WHERE singleton = 1",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("記録")
    }

    /// 20 表の全行 (表ごとに主キーの昇順、値は文字列へ写す)。
    fn surface(&self) -> Vec<Vec<String>> {
        let raw = self.raw();
        TABLES
            .iter()
            .map(|table| {
                let mut statement = raw
                    .prepare(&format!("SELECT * FROM {table} ORDER BY id"))
                    .expect("文");
                let columns = statement.column_count();
                statement
                    .query_map([], |row| {
                        (0..columns)
                            .map(|column| {
                                row.get::<_, rusqlite::types::Value>(column)
                                    .map(|value| format!("{value:?}"))
                            })
                            .collect::<Result<Vec<_>, _>>()
                            .map(|values| values.join("|"))
                    })
                    .expect("引ける")
                    .map(Result::unwrap)
                    .collect()
            })
            .collect()
    }

    fn count(&self, table: &str) -> i64 {
        self.raw()
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("数えられる")
    }
}

fn projection() -> ProjectionName {
    ProjectionName::parse("structured").expect("投影名は kebab")
}

// ---- 読む → 投影 → 表の DAO で書く ----

#[tokio::test]
async fn the_first_update_writes_the_projected_rows_and_saves_the_position() {
    let fixture = Fixture::seeded().await;
    let (last, tables) = fixture.history().await;

    fixture.update().await.unwrap();

    assert_eq!(fixture.checkpoint(), Ok(last), "処理したシーケンス番号");
    assert_eq!(
        usize::try_from(fixture.count("read_execution")).unwrap(),
        tables.executions().len()
    );
    assert_eq!(
        usize::try_from(fixture.count("read_run_stage")).unwrap(),
        tables.run_stages().len()
    );
    assert_eq!(
        usize::try_from(fixture.count("read_next_answer")).unwrap(),
        tables.next_answers().len()
    );
    let (position, _, _, _, verified) = fixture.head();
    assert_eq!(u64::try_from(position).unwrap(), last.to_u64());
    assert!(verified, "書いた内容で共有面の記録を作り直す");
    let as_of: Vec<i64> = fixture
        .raw()
        .prepare("SELECT DISTINCT as_of FROM read_execution")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        as_of,
        [i64::try_from(last.to_u64()).unwrap()],
        "全行が同じ断面を名乗る"
    );
}

#[tokio::test]
async fn a_restart_resumes_after_the_saved_position_and_a_repeat_writes_nothing() {
    let fixture = Fixture::seeded().await;
    fixture.update().await.unwrap();
    let head = fixture.head();
    let surface = fixture.surface();

    // 新しい事実が無ければ、再起動した更新器も何も書かない (記録の世代も動かない)。
    fixture.update().await.unwrap();
    assert_eq!(fixture.head(), head, "繰り返しは何も書かない");
    assert_eq!(fixture.surface(), surface);

    // 番号より後に事実が増えれば、その次から読み進めて番号も進める。
    fixture.another_execution().await;
    let (last, _) = fixture.history().await;
    fixture.update().await.unwrap();
    assert_eq!(fixture.checkpoint(), Ok(last));
    assert_eq!(fixture.count("read_execution"), 2, "新しい実行の行が増える");
    assert_eq!(fixture.head().1, head.1 + 1, "記録の世代が 1 つ進む");
}

#[tokio::test]
async fn a_renumbered_journal_is_refused_by_the_anchor() {
    // 保存したアンカー (位置の行の集約 ID・通番) がジャーナルの同じ位置の行と食い違えば、
    // 位置の振り直し・改変の兆候として読み進めない。照合はチェックポイントの表とジャーナルを
    // またぐので、DAO ではなく更新器 (構造化面の手順) が持つ。
    let fixture = Fixture::seeded().await;
    fixture.update().await.unwrap();
    let (last, _) = fixture.history().await;
    let surface = fixture.surface();
    fixture
        .raw()
        .execute(
            "UPDATE journal SET seq_nr = seq_nr + 100 WHERE rowid = ?1",
            [i64::try_from(last.to_u64()).unwrap()],
        )
        .expect("位置の行を書き換える");
    fixture.another_execution().await;

    let error = fixture.update().await.unwrap_err();
    assert!(
        matches!(
            error,
            ReadModelUpdateError::Read(JournalReadError::Corrupt {
                cause: CorruptCause::CheckpointAnchorMismatch,
                ..
            })
        ),
        "{error:?}"
    );
    assert_eq!(fixture.surface(), surface, "読み進めずに止める");
    // 公開の側 (Markdown 面) が読む番号も、同じ照合を通る。
    assert!(matches!(
        JournalReaderImpl::open(&fixture.path)
            .unwrap()
            .checkpoint(&projection())
            .await,
        Err(JournalReadError::Corrupt {
            cause: CorruptCause::CheckpointAnchorMismatch,
            ..
        })
    ));
}

#[tokio::test]
async fn an_update_that_fails_midway_moves_no_table_no_record_and_no_position() {
    let fixture = Fixture::seeded().await;
    fixture.update().await.unwrap();
    let position = fixture.checkpoint().unwrap();
    let head = fixture.head();
    let surface = fixture.surface();
    fixture.another_execution().await;
    // 20 表の差し替えの終盤 (最後に書く表の 1 つ) で落ちるようにする — それより前の表の
    // 削除・書込は同じトランザクションの中で済んでいる。
    fixture
        .raw()
        .execute_batch(
            "CREATE TRIGGER refuse_scope_change BEFORE INSERT ON read_scope_change
             BEGIN SELECT RAISE(ABORT, 'refused'); END;",
        )
        .unwrap();

    let error = fixture.update().await.unwrap_err();
    assert!(
        matches!(
            error,
            ReadModelUpdateError::Read(JournalReadError::Io { .. })
        ),
        "{error:?}"
    );
    assert_eq!(fixture.checkpoint(), Ok(position), "番号は動かない");
    assert_eq!(fixture.head(), head, "共有面の記録も動かない");
    assert_eq!(fixture.surface(), surface, "消した行も戻る");

    // 原因を取り除けば、次の更新が保存した番号の次から描き直す。
    fixture
        .raw()
        .execute_batch("DROP TRIGGER refuse_scope_change")
        .unwrap();
    fixture.update().await.unwrap();
    assert!(fixture.checkpoint().unwrap() > position);
}

// ---- 書込ロック (#134) ----

/// 別の接続で書込ロック (`BEGIN IMMEDIATE`) を握り、合図から HOLD だけ保持して手放す。
///
/// ホルダは HOLD を「主スレッドが更新を呼ぶ直前」の合図から数える。呼び出しの所要時間が
/// HOLD の半分以上なら、ロック待ちを実際に観測したと言える (PR1 の自己診断と同じ立て付け)。
struct LockHolder {
    handle: JoinHandle<()>,
    calling: Sender<()>,
}

impl LockHolder {
    fn hold(path: &Path) -> LockHolder {
        let (locked_sender, locked_receiver) = channel::<()>();
        let (calling, calling_receiver) = channel::<()>();
        let holder_path = path.to_path_buf();
        let handle = std::thread::spawn(move || {
            let connection = Connection::open(&holder_path).unwrap();
            connection.execute_batch("BEGIN IMMEDIATE").unwrap();
            locked_sender.send(()).unwrap();
            if calling_receiver.recv().is_ok() {
                std::thread::sleep(HOLD);
            }
            connection.execute_batch("END").unwrap();
        });
        locked_receiver.recv().unwrap();
        LockHolder { handle, calling }
    }

    /// 更新を呼ぶ直前の合図を送り、呼び出しの所要時間と結果を返す。
    async fn measure<F>(self, update: F) -> (Duration, Result<(), ReadModelUpdateError>)
    where
        F: Future<Output = Result<(), ReadModelUpdateError>>,
    {
        self.calling.send(()).unwrap();
        let started = Instant::now();
        let result = update.await;
        let waited = started.elapsed();
        drop(self.calling);
        self.handle.join().unwrap();
        (waited, result)
    }
}

/// 別の接続で書込ロックを握り続け、手放す合図を待つ。
struct HeldLock {
    handle: JoinHandle<()>,
    release: Sender<()>,
}

impl HeldLock {
    fn hold(path: &Path) -> HeldLock {
        let (locked_sender, locked_receiver) = channel::<()>();
        let (release, release_receiver) = channel::<()>();
        let holder_path = path.to_path_buf();
        let handle = std::thread::spawn(move || {
            let connection = Connection::open(&holder_path).unwrap();
            connection.execute_batch("BEGIN IMMEDIATE").unwrap();
            locked_sender.send(()).unwrap();
            let _ = release_receiver.recv();
            connection.execute_batch("END").unwrap();
        });
        locked_receiver.recv().unwrap();
        HeldLock { handle, release }
    }

    fn release(self) {
        drop(self.release);
        self.handle.join().unwrap();
    }
}

#[tokio::test]
async fn the_update_waits_for_a_write_lock_held_by_another_connection() {
    // #134 の教訓の回帰。前進は保存済みの番号と共有面の記録を読んでから書く。DEFERRED で
    // 始めると、別の接続が書込ロックを握っている間は、読んだ後の書込昇格が busy timeout を
    // 待たずに即 `SQLITE_BUSY` (= `WouldBlock`) になる。IMMEDIATE なら最初に書込ロックを
    // 待ち、解放後に書く。ホルダが握る時間 (HOLD = 200ms) は busy timeout (5000ms) より十分
    // 短いので、IMMEDIATE なら解放を待って成功する。
    let fixture = Fixture::seeded().await;
    let mut updater = fixture.updater();
    let (last, _) = fixture.history().await;

    let holder = LockHolder::hold(fixture.store());
    let (waited, result) = holder.measure(updater.update_read_models()).await;

    assert_eq!(result, Ok(()), "書込ロックの解放を待って書く");
    assert!(
        waited >= MIN_OBSERVED_WAIT,
        "ロック待ちを観測していない (所要 {waited:?})。ホルダが呼び出しより先に放した"
    );
    assert_eq!(fixture.checkpoint(), Ok(last));
}

#[tokio::test]
async fn opening_does_not_wait_for_a_write_lock_when_the_tables_exist() {
    // 読取だけの動詞も開く段を通る。表が揃っていて版も現行なら、構造化面の更新器の開く段も
    // ジャーナルの読み手の開く段も書込ロックを取らない (以前は共有面の記録の
    // `INSERT OR IGNORE` が、ジャーナルの読み手を開くたびに書込ロックを取っていた)。
    let fixture = Fixture::seeded().await;
    fixture.update().await.unwrap();
    // ジャーナルの読み手がまだ抱えている公開計画の表 (PR5 で移す) も一度作らせておく。
    drop(JournalReaderImpl::open(&fixture.path).unwrap());
    let held = HeldLock::hold(fixture.store());

    let started = Instant::now();
    let structured = StructuredReadModelUpdater::open(fixture.store()).map(drop);
    let journal = JournalReaderImpl::open_with_busy_timeout(&fixture.path, PROMPT_OPEN).map(drop);
    let waited = started.elapsed();
    held.release();

    assert_eq!(structured, Ok(()));
    assert_eq!(journal, Ok(()));
    assert!(
        waited < PROMPT_OPEN,
        "開く段が書込ロックを待った (所要 {waited:?})"
    );
}

#[tokio::test]
async fn opening_creates_the_tables_again_when_one_is_missing() {
    let fixture = Fixture::seeded().await;
    fixture.update().await.unwrap();
    fixture
        .raw()
        .execute_batch("DROP TABLE read_next_jump; DROP TABLE read_pipeline_progress")
        .unwrap();
    drop(StructuredReadModelUpdater::open(fixture.store()).unwrap());
    assert_eq!(fixture.count("read_next_jump"), 0, "空の表として在る");
    assert_eq!(
        fixture.count("read_pipeline_progress"),
        0,
        "参照入力由来の表も揃える (クエリ側は面ごとの更新器が走る前にも引く)"
    );
}

// ---- 共有面の記録 ----

#[tokio::test]
async fn a_shared_surface_from_an_old_transform_is_rebuilt_before_anything_else() {
    // 共有面の点検 (旧 `JournalReader::prepare_read_model`)。旧い変換で描かれた面は、投影名を
    // 束ねない更新でも現在の全履歴から描き直す。開く段だけでは描き直さない。
    let fixture = Fixture::seeded().await;
    fixture.update().await.unwrap();
    let (_, generation, _, digest, _) = fixture.head();
    fixture
        .raw()
        .execute_batch(
            "DELETE FROM read_intent;
             UPDATE amadeus_read_model_head SET revision = 'old-transform'",
        )
        .unwrap();

    let mut updater = StructuredReadModelUpdater::open(fixture.store()).unwrap();
    assert_eq!(
        fixture.count("read_intent"),
        0,
        "開く段だけでは描き直さない"
    );
    updater.update_read_models().await.unwrap();

    assert_eq!(fixture.count("read_intent"), 1, "全履歴から描き直す");
    let (_, rebuilt_generation, revision, rebuilt_digest, verified) = fixture.head();
    assert_ne!(revision, "old-transform");
    assert_eq!(rebuilt_generation, generation + 1);
    assert_eq!(rebuilt_digest, digest, "同じ履歴からは同じ内容");
    assert!(verified);
}

#[tokio::test]
async fn the_recorded_digest_follows_the_published_format() {
    // 保存済みのストアの記録は、表ごとの DAO へ分ける前の `read_tables::content_digest` で
    // 計算されている。ここではその計算 (表の並び・値の格納型の印・バイト列) を書き下し、
    // 記録と一致することを確かめる — 形式が変わると既存のストアの記録がすべて食い違い、
    // 公開が止まる。
    let fixture = Fixture::seeded().await;
    fixture.update().await.unwrap();
    let raw = fixture.raw();
    let material: DigestMaterial = TABLES
        .iter()
        .map(|table| {
            let mut statement = raw
                .prepare(&format!("SELECT * FROM {table} ORDER BY id"))
                .unwrap();
            let columns = statement.column_count();
            statement
                .query_map([], |row| {
                    (0..columns)
                        .map(|column| row.get::<_, rusqlite::types::Value>(column))
                        .collect::<Result<Vec<_>, _>>()
                })
                .unwrap()
                .map(|row| {
                    row.unwrap()
                        .into_iter()
                        .map(|value| match value {
                            rusqlite::types::Value::Null => (0_u8, Vec::new()),
                            rusqlite::types::Value::Integer(value) => {
                                (1, value.to_be_bytes().to_vec())
                            }
                            rusqlite::types::Value::Real(value) => {
                                (2, value.to_bits().to_be_bytes().to_vec())
                            }
                            rusqlite::types::Value::Text(value) => (3, value.into_bytes()),
                            rusqlite::types::Value::Blob(value) => (4, value),
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    let names: &[&str] = &TABLES;
    let json = core_infrastructure::canon_json::to_value(&(names, material)).unwrap();
    assert_eq!(
        fixture.head().3,
        core_infrastructure::canon_json::hash_compact(&json).rendered()
    );
}

#[tokio::test]
async fn a_history_behind_the_recorded_position_is_not_rebuilt() {
    // 記録やチェックポイントが走査済みの位置より先を名乗っていれば、歴史が切り落とされた
    // 兆候なので描かずに止める (旧 `rebuild_read_model` の約束)。
    let fixture = Fixture::seeded().await;
    fixture.update().await.unwrap();
    let surface = fixture.surface();
    fixture
        .raw()
        .execute_batch(
            "DELETE FROM journal WHERE rowid = (SELECT MAX(rowid) FROM journal);
             UPDATE amadeus_read_model_head SET revision = 'old-transform'",
        )
        .unwrap();

    let error = StructuredReadModelUpdater::open(fixture.store())
        .unwrap()
        .update_read_models()
        .await
        .unwrap_err();

    assert!(
        matches!(
            error,
            ReadModelUpdateError::Read(JournalReadError::Corrupt {
                cause: CorruptCause::CheckpointAnchorMismatch,
                ..
            })
        ),
        "{error:?}"
    );
    assert_eq!(fixture.surface(), surface, "何も描かない");
}
