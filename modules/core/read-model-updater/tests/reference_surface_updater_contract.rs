//! 参照入力由来の単独面の更新器の契約 — steering・テスト契約・計画指紋・Code Generation 開始可否。
//!
//! どれも「読む → 投影 → 表の DAO で書く」形の更新器である
//! (`coding-rules/read-model-updater-structure.md`)。材料が人の編集するファイルや承認入力なので、
//! 冪等の鍵は処理したシーケンス番号ではなく行の `source_digest` である。ここでは実ストア
//! (本家のイベントストアが書いた履歴) の上で次を固定する:
//!
//! - 同じ入力からの更新は行を書き直さない (保存済みの行を横から書き換えて、残ることを見る)
//! - 入力が動けば書き直す
//! - 途中で失敗すれば、どの行も動かない (同じトランザクションで確定するため)
//! - 別の接続が書込ロックを握っている間は、IMMEDIATE で待ってから書く (#134)
//!
//! 計画指紋と開始可否の行は実行と intent の履歴からしか組めないので、その 2 表の DAO の
//! 「書いて読み戻す」試験もここに置く。

// テストコードでは unwrap / expect / panic を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use core_command_domain::orchestration::{
    PlanApprovalDocuments, PlanApprovalInput, PlanReceipts, PlanTarget, TestingSections,
};
use core_command_domain::workspace::{SpaceName, StorePath};
use core_read_model_updater::orchestration::{
    CodeGenerationApprovalDao as _, CodeGenerationApprovalDaoImpl,
    CodeGenerationApprovalReadModelUpdater, CodeGenerationApprovalRow, GlobalSeqNr,
    JournalReadError, JournalReader as _, JournalReaderImpl, PlanFingerprintDao as _,
    PlanFingerprintDaoImpl, PlanFingerprintReadModelUpdater, PlanFingerprintRow,
    ReadModelUpdateError, ReadModelUpdater, SourceStamp, SteeringReadModelUpdater, SteeringSource,
    TestingReadModelUpdater,
};
use core_read_model_updater::read_tables::{CodeGenerationApprovalTables, PlanFingerprintTables};
use rusqlite::Connection;
use tempfile::TempDir;

use support::{execution_id, open_store, seed, seed_intent};

/// ロックを握る側が保持する時間。
const HOLD: Duration = Duration::from_millis(200);

/// 呼び出しがロック待ちを実際に観測したと言える所要時間の下限 (HOLD の半分)。
const MIN_OBSERVED_WAIT: Duration = Duration::from_millis(100);

/// intent と実行 1 本の履歴を持つ実ストアと、memory 層の置き場。
struct Fixture {
    _dir: TempDir,
    path: StorePath,
    memory: PathBuf,
}

impl Fixture {
    async fn seeded() -> Fixture {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let path = StorePath::for_space(&dir.path().join("aidlc"), &SpaceName::default());
        std::fs::create_dir_all(path.as_path().parent().expect("親 dir を持つ"))
            .expect("intents/ を先に作る");
        seed_intent(&path).await;
        seed(&mut open_store(&path)).await;
        let memory = dir.path().join("memory");
        std::fs::create_dir_all(memory.join("phases")).expect("memory 層を作る");
        std::fs::write(
            memory.join("org.md"),
            "# Org\n\nALWAYS keep the audit record.\n",
        )
        .expect("規則を置く");
        let fixture = Fixture {
            _dir: dir,
            path,
            memory,
        };
        // 開く段で読み面と参照入力由来の表が揃う (本番と同じ順)。
        drop(fixture.journal_reader());
        fixture
    }

    fn store(&self) -> &Path {
        self.path.as_path()
    }

    fn raw(&self) -> Connection {
        Connection::open(self.store()).expect("生の接続")
    }

    fn journal_reader(&self) -> JournalReaderImpl {
        JournalReaderImpl::open(&self.path).expect("Reader は開ける")
    }

    fn write_rule(&self, relative: &str, text: &str) {
        std::fs::write(self.memory.join(relative), text).expect("規則を書く");
    }

    fn source(&self) -> SteeringSource {
        SteeringSource::new(self.memory.clone())
    }

    fn count(&self, table: &str) -> i64 {
        self.raw()
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    fn text(&self, sql: &str) -> String {
        self.raw().query_row(sql, [], |row| row.get(0)).unwrap()
    }

    fn execute(&self, sql: &str) {
        self.raw().execute_batch(sql).unwrap();
    }
}

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

fn plan_input(plan: &str) -> PlanApprovalInput {
    PlanApprovalInput::new(
        PlanApprovalDocuments::new(
            plan.to_string(),
            String::new(),
            String::new(),
            "questions.md".to_string(),
        ),
        TestingSections::from_documents("", "", ""),
        PlanTarget::stage_level(),
        None,
        None,
    )
}

// ---- 開く段は、表が揃っていれば書込ロックを取らない ----

/// 開く段が書込ロックを待ったと見なす所要時間 (busy timeout 5000ms より十分短い)。
const PROMPT_OPEN: Duration = Duration::from_millis(1000);

/// 別の接続で書込ロック (`BEGIN IMMEDIATE`) を握り続け、手放す合図を待つ。
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
async fn opening_each_updater_does_not_wait_for_a_write_lock_when_its_tables_exist() {
    // 読取だけの動詞も開く段を通る。表が在るのに書込ロックを取りに行くと、別の書き手が
    // いる間は busy timeout まで待たされ、最後は `WouldBlock` で落ちる (CodeRabbit の指摘)。
    let fixture = Fixture::seeded().await;
    let mut reader = fixture.journal_reader();
    let source = fixture.source();
    let execution = execution_id();
    let input = plan_input("# Plan\n");
    let receipts = PlanReceipts::default();
    let held = HeldLock::hold(fixture.store());

    let started = Instant::now();
    let steering = SteeringReadModelUpdater::open(fixture.store(), fixture.source()).map(drop);
    let testing = TestingReadModelUpdater::open(&mut reader, fixture.store(), &source).map(drop);
    let fingerprint =
        PlanFingerprintReadModelUpdater::open(&mut reader, fixture.store(), &execution, &input)
            .map(drop);
    let approval = CodeGenerationApprovalReadModelUpdater::open(
        &mut reader,
        fixture.store(),
        &execution,
        &input,
        &receipts,
    )
    .map(drop);
    let waited = started.elapsed();
    held.release();

    assert_eq!(steering, Ok(()));
    assert_eq!(testing, Ok(()));
    assert_eq!(fingerprint, Ok(()));
    assert_eq!(approval, Ok(()));
    assert!(
        waited < PROMPT_OPEN,
        "開く段が書込ロックを待った (所要 {waited:?})"
    );
}

#[tokio::test]
async fn opening_an_updater_creates_its_table_again_when_it_is_missing() {
    // 読み手は落とす前に開く (開く段の JournalReaderImpl も表を作り直すので、更新器が
    // 自分で作ることを見るには、落とした後に読み手を開かない)。
    let fixture = Fixture::seeded().await;
    let mut reader = fixture.journal_reader();
    fixture.execute("DROP TABLE read_testing_contract; DROP TABLE read_steering_part");
    let source = fixture.source();
    drop(TestingReadModelUpdater::open(&mut reader, fixture.store(), &source).unwrap());
    drop(SteeringReadModelUpdater::open(fixture.store(), fixture.source()).unwrap());
    assert_eq!(
        fixture.count("read_testing_contract"),
        0,
        "空の表として在る"
    );
    assert_eq!(fixture.count("read_steering_part"), 0, "空の表として在る");
}

// ---- steering (`read_steering_plan` / `read_steering_part`) ----

async fn update_steering(fixture: &Fixture) -> Result<(), ReadModelUpdateError> {
    SteeringReadModelUpdater::open(fixture.store(), fixture.source())?
        .update_read_models()
        .await
}

#[tokio::test]
async fn the_first_steering_update_projects_the_memory_layer_it_finds() {
    let fixture = Fixture::seeded().await;
    fixture.write_rule(
        "phases/inception.md",
        "# Inception\n\nALWAYS confirm the scope.\n",
    );
    update_steering(&fixture).await.unwrap();

    assert_eq!(fixture.count("read_steering_plan"), 5, "束は phase の関数");
    assert_eq!(fixture.count("read_steering_part"), 5, "1 部 × 5 フェーズ");
    let (parts, delivered): (i64, String) = fixture
        .raw()
        .query_row(
            "SELECT part_count, delivered_paths FROM read_steering_plan WHERE phase = 'inception'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(parts, 1);
    assert!(
        delivered.contains("org.md") && delivered.contains("phases/inception.md"),
        "実際: {delivered}"
    );
}

#[tokio::test]
async fn an_unchanged_memory_layer_does_not_rewrite_the_steering_rows() {
    // 参照入力を読み直すのは毎回だが、書き替えるのはダイジェストが動いたときだけである。
    let fixture = Fixture::seeded().await;
    update_steering(&fixture).await.unwrap();
    fixture.execute("UPDATE read_steering_plan SET delivered_paths = 'untouched'");
    fixture.execute("UPDATE read_steering_part SET rules_content = 'untouched'");
    update_steering(&fixture).await.unwrap();
    assert_eq!(
        fixture.text("SELECT group_concat(DISTINCT delivered_paths) FROM read_steering_plan"),
        "untouched",
        "同じ参照入力では書き替えない"
    );
    assert_eq!(
        fixture.text("SELECT group_concat(DISTINCT rules_content) FROM read_steering_part"),
        "untouched"
    );
}

#[tokio::test]
async fn an_edited_rule_file_rewrites_both_steering_tables() {
    let fixture = Fixture::seeded().await;
    update_steering(&fixture).await.unwrap();
    let before = fixture.text("SELECT source_digest FROM read_steering_plan LIMIT 1");
    let bundle =
        fixture.text("SELECT bundle_digest FROM read_steering_plan ORDER BY phase LIMIT 1");
    fixture.execute("UPDATE read_steering_part SET rules_content = 'stale'");

    fixture.write_rule("org.md", "# Org\n\n変更した規則\n");
    update_steering(&fixture).await.unwrap();

    assert_ne!(
        fixture.text("SELECT source_digest FROM read_steering_plan LIMIT 1"),
        before
    );
    assert_ne!(
        fixture.text("SELECT bundle_digest FROM read_steering_plan ORDER BY phase LIMIT 1"),
        bundle,
        "束のダイジェストも動く"
    );
    assert_eq!(
        fixture.count("read_steering_part"),
        5,
        "部の表も同じ更新で差し替わる"
    );
    assert!(
        !fixture
            .text("SELECT group_concat(rules_content) FROM read_steering_part")
            .contains("stale")
    );
}

#[tokio::test]
async fn a_rule_file_that_disappears_is_normal_and_shrinks_the_bundle() {
    let fixture = Fixture::seeded().await;
    update_steering(&fixture).await.unwrap();
    std::fs::remove_file(fixture.memory.join("org.md")).unwrap();
    update_steering(&fixture).await.expect("欠損は正常");
    assert_eq!(
        fixture.text("SELECT group_concat(DISTINCT part_count) FROM read_steering_plan"),
        "0",
        "配る規則が 1 本も無い"
    );
    assert_eq!(
        fixture.text("SELECT group_concat(DISTINCT delivered_paths) FROM read_steering_plan"),
        "[]"
    );
    assert_eq!(fixture.count("read_steering_part"), 0);
}

#[tokio::test]
async fn a_rule_file_that_exists_but_cannot_be_read_writes_nothing() {
    // 「在るのに読めない」は blocking である — 規則を落として進むと、届く steering が
    // 静かに痩せる。
    let fixture = Fixture::seeded().await;
    std::fs::write(fixture.memory.join("team.md"), [0x80_u8, 0x81]).unwrap();
    match update_steering(&fixture).await {
        Err(ReadModelUpdateError::SteeringRead { path, kind }) => {
            assert!(path.ends_with("team.md"), "実際: {path}");
            assert_eq!(kind, std::io::ErrorKind::InvalidData);
        }
        other => panic!("読取の失敗として上がる (実際: {other:?})"),
    }
    assert_eq!(fixture.count("read_steering_plan"), 0, "1 行も書かない");
    assert_eq!(fixture.count("read_steering_part"), 0);
}

#[tokio::test]
async fn a_steering_update_that_fails_midway_moves_neither_table() {
    let fixture = Fixture::seeded().await;
    update_steering(&fixture).await.unwrap();
    let digest = fixture.text("SELECT source_digest FROM read_steering_plan LIMIT 1");
    let parts = fixture.text("SELECT group_concat(rules_content) FROM read_steering_part");
    // 計画の表を書いた後、部の表の書込で落ちるようにする (トランザクションの途中の失敗)。
    fixture.execute(
        "CREATE TRIGGER refuse_parts BEFORE INSERT ON read_steering_part
         BEGIN SELECT RAISE(ABORT, 'refused'); END;",
    );
    fixture.write_rule("org.md", "# Org\n\n直した規則\n");
    let error = update_steering(&fixture).await.unwrap_err();
    assert!(
        matches!(
            error,
            ReadModelUpdateError::Read(JournalReadError::Io { .. })
        ),
        "実際: {error:?}"
    );
    assert_eq!(
        fixture.text("SELECT group_concat(DISTINCT source_digest) FROM read_steering_plan"),
        digest,
        "計画の表は確定されていない"
    );
    assert_eq!(
        fixture.text("SELECT group_concat(rules_content) FROM read_steering_part"),
        parts,
        "部の表も元のまま"
    );

    // 原因を取り除けば、次の更新が同じ参照入力を描き直す。
    fixture.execute("DROP TRIGGER refuse_parts");
    update_steering(&fixture).await.unwrap();
    assert_ne!(
        fixture.text("SELECT source_digest FROM read_steering_plan LIMIT 1"),
        digest
    );
}

#[tokio::test]
async fn the_steering_update_waits_for_a_write_lock_held_by_another_connection() {
    // #134 の教訓の回帰。更新器は保存済みの出所を読んでから書く。DEFERRED で始めると、
    // 別の接続が書込ロックを握っている間は、読んだ後の書込昇格が busy timeout を待たずに
    // 即 `SQLITE_BUSY` になる。IMMEDIATE なら最初に書込ロックを待ち、解放後に書く。
    let fixture = Fixture::seeded().await;
    let mut updater = SteeringReadModelUpdater::open(fixture.store(), fixture.source()).unwrap();
    updater.update_read_models().await.unwrap();
    let before = fixture.text("SELECT source_digest FROM read_steering_plan LIMIT 1");
    fixture.write_rule("org.md", "# Org\n\nロック中に直した規則\n");

    let holder = LockHolder::hold(fixture.store());
    let (waited, result) = holder.measure(updater.update_read_models()).await;

    assert_eq!(result, Ok(()), "書込ロックの解放を待って書く");
    assert!(
        waited >= MIN_OBSERVED_WAIT,
        "ロック待ちを観測していない (所要 {waited:?})"
    );
    assert_ne!(
        fixture.text("SELECT source_digest FROM read_steering_plan LIMIT 1"),
        before
    );
}

// ---- テスト契約 (`read_testing_contract`) ----

async fn update_testing(fixture: &Fixture) -> Result<(), ReadModelUpdateError> {
    let mut reader = fixture.journal_reader();
    let source = fixture.source();
    TestingReadModelUpdater::open(&mut reader, fixture.store(), &source)?
        .update_read_models()
        .await
}

#[tokio::test]
async fn the_testing_contract_rows_are_written_for_the_bare_space_and_every_intent() {
    let fixture = Fixture::seeded().await;
    update_testing(&fixture).await.unwrap();
    assert_eq!(
        fixture.count("read_testing_contract"),
        2,
        "依頼前の既定行 + intent 1 本"
    );
    assert_eq!(
        fixture.text("SELECT group_concat(DISTINCT source_digest) FROM read_testing_contract"),
        fixture.text("SELECT source_digest FROM read_testing_contract WHERE id = 'bare-space'"),
        "全行が同じ出所を名乗る"
    );
}

#[tokio::test]
async fn an_unchanged_testing_input_does_not_rewrite_the_rows() {
    let fixture = Fixture::seeded().await;
    update_testing(&fixture).await.unwrap();
    fixture.execute("UPDATE read_testing_contract SET rendered = 'untouched'");
    update_testing(&fixture).await.unwrap();
    assert_eq!(
        fixture.text("SELECT group_concat(DISTINCT rendered) FROM read_testing_contract"),
        "untouched",
        "同じ規則と依頼からは書き替えない"
    );
}

#[tokio::test]
async fn an_edited_testing_rule_rewrites_the_rows() {
    let fixture = Fixture::seeded().await;
    update_testing(&fixture).await.unwrap();
    let before = fixture.text("SELECT source_digest FROM read_testing_contract LIMIT 1");
    fixture.execute("UPDATE read_testing_contract SET rendered = 'stale'");
    fixture.write_rule(
        "org.md",
        "# Org\n\n## Testing Posture\n\n- Methodology: tdd\n",
    );
    update_testing(&fixture).await.unwrap();
    assert_ne!(
        fixture.text("SELECT source_digest FROM read_testing_contract LIMIT 1"),
        before
    );
    assert_eq!(
        fixture.count("read_testing_contract"),
        2,
        "全行を差し替える (古い行を残さない)"
    );
    assert!(
        !fixture
            .text("SELECT group_concat(COALESCE(rendered, '')) FROM read_testing_contract")
            .contains("stale")
    );
}

#[tokio::test]
async fn rows_projected_at_a_later_history_position_are_not_overwritten_by_an_older_cut() {
    // 保存済みの行がより新しい履歴位置 (`as_of`) で作られていれば、古い断面で上書きしない。
    let fixture = Fixture::seeded().await;
    update_testing(&fixture).await.unwrap();
    fixture.execute(
        "UPDATE read_testing_contract SET source_digest = 'later', as_of = 1000000, rendered = 'later'",
    );
    fixture.write_rule(
        "org.md",
        "# Org\n\n## Testing Posture\n\n- Methodology: tdd\n",
    );
    update_testing(&fixture).await.unwrap();
    assert_eq!(
        fixture.text("SELECT group_concat(DISTINCT source_digest) FROM read_testing_contract"),
        "later"
    );
    assert_eq!(
        fixture.text("SELECT group_concat(DISTINCT rendered) FROM read_testing_contract"),
        "later"
    );
}

#[tokio::test]
async fn a_testing_update_that_fails_midway_leaves_every_row_unchanged() {
    let fixture = Fixture::seeded().await;
    update_testing(&fixture).await.unwrap();
    let before = fixture.text(
        "SELECT group_concat(id || ':' || source_digest || ':' || COALESCE(rendered, ''))
         FROM (SELECT * FROM read_testing_contract ORDER BY id)",
    );
    // 既定行を書いた後、intent の行の書込で落ちるようにする (全消しの後の途中の失敗)。
    fixture.execute(
        "CREATE TRIGGER refuse_intents BEFORE INSERT ON read_testing_contract
         WHEN NEW.id <> 'bare-space' BEGIN SELECT RAISE(ABORT, 'refused'); END;",
    );
    fixture.write_rule(
        "org.md",
        "# Org\n\n## Testing Posture\n\n- Methodology: tdd\n",
    );
    let error = update_testing(&fixture).await.unwrap_err();
    assert!(
        matches!(
            error,
            ReadModelUpdateError::Read(JournalReadError::Io { .. })
        ),
        "実際: {error:?}"
    );
    assert_eq!(
        fixture.text(
            "SELECT group_concat(id || ':' || source_digest || ':' || COALESCE(rendered, ''))
             FROM (SELECT * FROM read_testing_contract ORDER BY id)",
        ),
        before,
        "どの行も確定されていない"
    );
}

#[tokio::test]
async fn the_testing_update_waits_for_a_write_lock_held_by_another_connection() {
    let fixture = Fixture::seeded().await;
    update_testing(&fixture).await.unwrap();
    let before = fixture.text("SELECT source_digest FROM read_testing_contract LIMIT 1");
    fixture.write_rule(
        "org.md",
        "# Org\n\n## Testing Posture\n\n- Methodology: tdd\n",
    );
    let mut reader = fixture.journal_reader();
    let source = fixture.source();
    let mut updater = TestingReadModelUpdater::open(&mut reader, fixture.store(), &source).unwrap();

    let holder = LockHolder::hold(fixture.store());
    let (waited, result) = holder.measure(updater.update_read_models()).await;

    assert_eq!(result, Ok(()), "書込ロックの解放を待って書く");
    assert!(
        waited >= MIN_OBSERVED_WAIT,
        "ロック待ちを観測していない (所要 {waited:?})"
    );
    assert_ne!(
        fixture.text("SELECT source_digest FROM read_testing_contract LIMIT 1"),
        before
    );
}

// ---- 計画指紋 (`read_plan_fingerprint`) ----

async fn update_fingerprint(fixture: &Fixture, plan: &str) -> Result<(), ReadModelUpdateError> {
    let mut reader = fixture.journal_reader();
    let execution = execution_id();
    let input = plan_input(plan);
    PlanFingerprintReadModelUpdater::open(&mut reader, fixture.store(), &execution, &input)?
        .update_read_models()
        .await
}

async fn fingerprint_row(fixture: &Fixture, plan: &str) -> PlanFingerprintRow {
    let history = fixture
        .journal_reader()
        .events_after(GlobalSeqNr::ZERO)
        .await
        .unwrap();
    PlanFingerprintTables::project(&history, &execution_id(), &plan_input(plan))
        .unwrap()
        .row()
        .clone()
}

#[tokio::test]
async fn a_saved_plan_fingerprint_row_reads_back_column_for_column() {
    let fixture = Fixture::seeded().await;
    let row = fingerprint_row(&fixture, "# Plan\n\n- [ ] Step 1\n").await;
    let mut connection = fixture.raw();
    let mut transaction = connection.transaction().unwrap();
    PlanFingerprintDaoImpl.save(&mut transaction, &row).unwrap();
    PlanFingerprintDaoImpl.save(&mut transaction, &row).unwrap();
    transaction.commit().unwrap();

    type Columns = (
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        String,
        i64,
    );
    let saved: Vec<Columns> = connection
        .prepare(
            "SELECT id, execution_id, target_id, fingerprint, error, source_digest, as_of
             FROM read_plan_fingerprint",
        )
        .unwrap()
        .query_map([], |record| {
            Ok((
                record.get(0)?,
                record.get(1)?,
                record.get(2)?,
                record.get(3)?,
                record.get(4)?,
                record.get(5)?,
                record.get(6)?,
            ))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        saved,
        [(
            row.id().to_string(),
            row.execution_id().to_string(),
            row.target_id().to_string(),
            row.fingerprint().map(str::to_string),
            row.error().map(str::to_string),
            row.source_digest().to_string(),
            i64::try_from(row.as_of().to_u64()).unwrap(),
        )],
        "同じ主キーを 2 度書いても 1 行"
    );
    assert_eq!(
        PlanFingerprintDaoImpl
            .find_stamp(&connection, row.id())
            .unwrap(),
        Some(SourceStamp::new(
            row.source_digest().to_string(),
            row.as_of()
        ))
    );
}

#[tokio::test]
async fn an_unchanged_plan_input_does_not_rewrite_the_fingerprint_row() {
    let fixture = Fixture::seeded().await;
    update_fingerprint(&fixture, "# Plan\n").await.unwrap();
    fixture.execute("UPDATE read_plan_fingerprint SET error = 'untouched'");
    update_fingerprint(&fixture, "# Plan\n").await.unwrap();
    assert_eq!(
        fixture.text("SELECT error FROM read_plan_fingerprint"),
        "untouched"
    );
}

#[tokio::test]
async fn an_edited_plan_rewrites_the_fingerprint_row() {
    let fixture = Fixture::seeded().await;
    update_fingerprint(&fixture, "# Plan\n").await.unwrap();
    let before = fixture.text("SELECT source_digest FROM read_plan_fingerprint");
    fixture.execute("UPDATE read_plan_fingerprint SET error = 'stale'");
    update_fingerprint(&fixture, "# Plan\n\n- [ ] Step 1\n")
        .await
        .unwrap();
    assert_eq!(fixture.count("read_plan_fingerprint"), 1);
    assert_ne!(
        fixture.text("SELECT source_digest FROM read_plan_fingerprint"),
        before
    );
    assert_ne!(
        fixture.text("SELECT COALESCE(error, '') FROM read_plan_fingerprint"),
        "stale"
    );
}

#[tokio::test]
async fn a_fingerprint_update_that_fails_midway_leaves_the_row_unchanged() {
    let fixture = Fixture::seeded().await;
    update_fingerprint(&fixture, "# Plan\n").await.unwrap();
    let before = fixture.text("SELECT source_digest FROM read_plan_fingerprint");
    // 主キーの行を消した後、書き戻しで落ちるようにする (トランザクションの途中の失敗)。
    fixture.execute(
        "CREATE TRIGGER refuse BEFORE INSERT ON read_plan_fingerprint
         BEGIN SELECT RAISE(ABORT, 'refused'); END;",
    );
    let error = update_fingerprint(&fixture, "# Plan\n\n- [ ] Step 1\n")
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            ReadModelUpdateError::Read(JournalReadError::Io { .. })
        ),
        "実際: {error:?}"
    );
    assert_eq!(fixture.count("read_plan_fingerprint"), 1, "消した行も戻る");
    assert_eq!(
        fixture.text("SELECT source_digest FROM read_plan_fingerprint"),
        before
    );
}

#[tokio::test]
async fn the_fingerprint_update_waits_for_a_write_lock_held_by_another_connection() {
    let fixture = Fixture::seeded().await;
    update_fingerprint(&fixture, "# Plan\n").await.unwrap();
    let before = fixture.text("SELECT source_digest FROM read_plan_fingerprint");
    let mut reader = fixture.journal_reader();
    let execution = execution_id();
    let input = plan_input("# Plan\n\n- [ ] Step 1\n");
    let mut updater =
        PlanFingerprintReadModelUpdater::open(&mut reader, fixture.store(), &execution, &input)
            .unwrap();

    let holder = LockHolder::hold(fixture.store());
    let (waited, result) = holder.measure(updater.update_read_models()).await;

    assert_eq!(result, Ok(()), "書込ロックの解放を待って書く");
    assert!(
        waited >= MIN_OBSERVED_WAIT,
        "ロック待ちを観測していない (所要 {waited:?})"
    );
    assert_ne!(
        fixture.text("SELECT source_digest FROM read_plan_fingerprint"),
        before
    );
}

// ---- Code Generation 開始可否 (`read_code_generation_approval`) ----

async fn update_approval(fixture: &Fixture, plan: &str) -> Result<(), ReadModelUpdateError> {
    let mut reader = fixture.journal_reader();
    let execution = execution_id();
    let input = plan_input(plan);
    let receipts = PlanReceipts::default();
    CodeGenerationApprovalReadModelUpdater::open(
        &mut reader,
        fixture.store(),
        &execution,
        &input,
        &receipts,
    )?
    .update_read_models()
    .await
}

async fn approval_row(fixture: &Fixture, plan: &str) -> CodeGenerationApprovalRow {
    let history = fixture
        .journal_reader()
        .events_after(GlobalSeqNr::ZERO)
        .await
        .unwrap();
    CodeGenerationApprovalTables::project(
        &history,
        &execution_id(),
        &plan_input(plan),
        &PlanReceipts::default(),
    )
    .unwrap()
    .row()
    .clone()
}

#[tokio::test]
async fn a_saved_code_generation_approval_row_reads_back_column_for_column() {
    let fixture = Fixture::seeded().await;
    let row = approval_row(&fixture, "# Plan\n\n- [ ] Step 1\n").await;
    let mut connection = fixture.raw();
    let mut transaction = connection.transaction().unwrap();
    CodeGenerationApprovalDaoImpl
        .save(&mut transaction, &row)
        .unwrap();
    CodeGenerationApprovalDaoImpl
        .save(&mut transaction, &row)
        .unwrap();
    transaction.commit().unwrap();

    type Columns = (
        String,
        String,
        String,
        i64,
        String,
        Option<String>,
        Option<String>,
        String,
        i64,
    );
    let saved: Vec<Columns> = connection
        .prepare(
            "SELECT id, execution_id, target_id, ok, reason, unit, contract_hash, source_digest, as_of
             FROM read_code_generation_approval",
        )
        .unwrap()
        .query_map([], |record| {
            Ok((
                record.get(0)?,
                record.get(1)?,
                record.get(2)?,
                record.get(3)?,
                record.get(4)?,
                record.get(5)?,
                record.get(6)?,
                record.get(7)?,
                record.get(8)?,
            ))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        saved,
        [(
            row.id().to_string(),
            row.execution_id().to_string(),
            row.target_id().to_string(),
            i64::from(row.ok()),
            row.reason().to_string(),
            row.unit().map(str::to_string),
            row.contract_hash().map(str::to_string),
            row.source_digest().to_string(),
            i64::try_from(row.as_of().to_u64()).unwrap(),
        )],
        "同じ主キーを 2 度書いても 1 行"
    );
    assert_eq!(
        CodeGenerationApprovalDaoImpl
            .find_stamp(&connection, row.id())
            .unwrap(),
        Some(SourceStamp::new(
            row.source_digest().to_string(),
            row.as_of()
        ))
    );
}

#[tokio::test]
async fn an_unchanged_approval_input_does_not_rewrite_the_approval_row() {
    let fixture = Fixture::seeded().await;
    update_approval(&fixture, "# Plan\n").await.unwrap();
    fixture.execute("UPDATE read_code_generation_approval SET reason = 'untouched'");
    update_approval(&fixture, "# Plan\n").await.unwrap();
    assert_eq!(
        fixture.text("SELECT reason FROM read_code_generation_approval"),
        "untouched"
    );
}

#[tokio::test]
async fn an_edited_plan_rewrites_the_approval_row() {
    let fixture = Fixture::seeded().await;
    update_approval(&fixture, "# Plan\n").await.unwrap();
    let before = fixture.text("SELECT source_digest FROM read_code_generation_approval");
    fixture.execute("UPDATE read_code_generation_approval SET reason = 'stale'");
    update_approval(&fixture, "# Plan\n\n- [ ] Step 1\n")
        .await
        .unwrap();
    assert_eq!(fixture.count("read_code_generation_approval"), 1);
    assert_ne!(
        fixture.text("SELECT source_digest FROM read_code_generation_approval"),
        before
    );
    assert_ne!(
        fixture.text("SELECT reason FROM read_code_generation_approval"),
        "stale"
    );
}

#[tokio::test]
async fn an_approval_update_that_fails_midway_leaves_the_row_unchanged() {
    let fixture = Fixture::seeded().await;
    update_approval(&fixture, "# Plan\n").await.unwrap();
    let before = fixture.text("SELECT source_digest FROM read_code_generation_approval");
    fixture.execute(
        "CREATE TRIGGER refuse BEFORE INSERT ON read_code_generation_approval
         BEGIN SELECT RAISE(ABORT, 'refused'); END;",
    );
    let error = update_approval(&fixture, "# Plan\n\n- [ ] Step 1\n")
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            ReadModelUpdateError::Read(JournalReadError::Io { .. })
        ),
        "実際: {error:?}"
    );
    assert_eq!(
        fixture.count("read_code_generation_approval"),
        1,
        "消した行も戻る"
    );
    assert_eq!(
        fixture.text("SELECT source_digest FROM read_code_generation_approval"),
        before
    );
}

#[tokio::test]
async fn the_approval_update_waits_for_a_write_lock_held_by_another_connection() {
    let fixture = Fixture::seeded().await;
    update_approval(&fixture, "# Plan\n").await.unwrap();
    let before = fixture.text("SELECT source_digest FROM read_code_generation_approval");
    let mut reader = fixture.journal_reader();
    let execution = execution_id();
    let input = plan_input("# Plan\n\n- [ ] Step 1\n");
    let receipts = PlanReceipts::default();
    let mut updater = CodeGenerationApprovalReadModelUpdater::open(
        &mut reader,
        fixture.store(),
        &execution,
        &input,
        &receipts,
    )
    .unwrap();

    let holder = LockHolder::hold(fixture.store());
    let (waited, result) = holder.measure(updater.update_read_models()).await;

    assert_eq!(result, Ok(()), "書込ロックの解放を待って書く");
    assert!(
        waited >= MIN_OBSERVED_WAIT,
        "ロック待ちを観測していない (所要 {waited:?})"
    );
    assert_ne!(
        fixture.text("SELECT source_digest FROM read_code_generation_approval"),
        before
    );
}
