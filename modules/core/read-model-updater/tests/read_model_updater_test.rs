//! 取得ループ（`ReadModelUpdater`）の契約 — checkpoint → 差分読取 → 投影 → 書込 → 前進。
//!
//! 読み手はフェイクである。実 `JournalReaderImpl` の読み方は
//! `journal_reader_impl_test.rs` が固定しており、ここが見るのは**ループの約束**（空差分の
//! 扱い・書いてから進める順序・再生成の冪等）だからである。フェイクなら、まだ投影規則の
//! 裁定が降りていないイベントを混ぜずに、ループだけを孤立させて観測できる。

// テストコードでは unwrap / expect / panic を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    Created, GateOpened, Intent, IntentEventId, IntentExecutionEvent, IntentExecutionEventId,
    IntentExecutionId, IntentId, StageDisplay, StageEntry, StageRevised, StartRequest, Started,
    WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
    WorkflowDefinitionId,
};
use core_read_model_updater::orchestration::{
    GlobalSeqNr, JournalBatch, JournalEntry, JournalReadError, JournalReader, ProjectionName,
    ProjectionTargets, ReadModelUpdater,
};
use core_read_model_updater::read_tables::ReadTables;
use tempfile::TempDir;

/// b40 のテスト用固定イベント識別子 (同じ材料から組んだイベントを同値に保つため)。
fn event_id() -> IntentExecutionEventId {
    IntentExecutionEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002").expect("UUIDv7")
}

/// b40 のテスト用集約識別子 (行の `aid` と payload の `aggregate_id` を揃える)。
fn execution_id() -> IntentExecutionId {
    IntentExecutionId::parse(EXECUTION).expect("UUIDv7")
}

/// b40 のテスト用固定イベント識別子 (intent 面)。
fn intent_event_id() -> IntentEventId {
    IntentEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0001").expect("UUIDv7")
}

/// 状態ファイルの出発点（投影が触る行だけを持つ最小の本文）。
const STATE: &str = "\
## Project Information
- **Scope**: classic

## Stage Progress
- [-] practices-discovery — EXECUTE
";

const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

/// テストの実行識別子 (ジャーナル行の集約キー)。
const EXECUTION: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";

fn at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-08-21T09:14:07Z")
        .expect("固定の ISO 8601")
        .with_timezone(&Utc)
}

fn slug(value: &str) -> StageSlug {
    StageSlug::parse(value).expect("テストの slug は文法内")
}

/// ジャーナル 1 行。
///
/// **横断通番 (`global`) と集約内通番 (`seq_nr`) は別物である** — 前者はジャーナル全体の
/// 追記順、後者はその集約の歴史の何番目かで、誕生記録は必ず 1 から始まる。同じ値にすると
/// 集約を再生できない歴史になる（構造化投影核が `apply_event` の通番検査で落ちる）ので、
/// フィクスチャでも 2 つを分けて渡す。
fn entry(global: u64, seq_nr: usize, event: IntentExecutionEvent) -> JournalEntry {
    JournalEntry::new(
        GlobalSeqNr::new(global),
        IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
        seq_nr,
        at(),
        event,
    )
}

/// intent の誕生の材料（取得ループはここから計画を引く — issue #56）。
fn genesis_intent() -> Intent {
    let stage = |name: &str, number: &str, agent: &str| {
        StageEntry::new(
            slug(name),
            PhaseId::Inception,
            PlanAction::Execute,
            false,
            StageDisplay::new(
                StageNumber::parse(number).expect("番号"),
                "Practices Discovery",
                agent,
            )
            .expect("単一行"),
        )
    };
    Intent::from((
        Created::new(
            intent_event_id(),
            IntentId::parse(INTENT).expect("UUIDv7"),
            WorkflowDefinitionId::parse("claude").expect("定義 id"),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision"),
            StartRequest::new("classic", "build it"),
            vec![stage(
                "practices-discovery",
                "2.2",
                "aidlc-pipeline-deploy-agent",
            )],
            WorkspaceScan::new(
                BrownfieldGreenfield::Greenfield,
                "Unknown",
                "Unknown",
                "Unknown",
            )
            .expect("単一行"),
        ),
        at(),
    ))
}

/// 実行開始の事実（genesis の材料 = 実行 id・intent id・解決済み計画）。
fn genesis() -> IntentExecutionEvent {
    let intent = genesis_intent();
    IntentExecutionEvent::Started(Started::new(
        event_id(),
        IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
        intent.id().clone(),
        intent.stages().to_vec(),
    ))
}

/// intent の誕生記録（global 1 — 実行のどの行よりも先に書かれている）。
fn intents() -> Vec<(u64, Intent)> {
    vec![(1, genesis_intent())]
}

/// genesis + 投影規則が確定しているイベント 2 件（global 2〜4。1 は intent の誕生記録）。
///
/// チェックポイントは genesis の直後（2）から始める — 取得ループが見るのは差分だけであり、
/// genesis の状態面（新規スキャフォールド）は本 Bolt の射程外だからである。genesis と
/// 誕生記録は**計画の供給元**としてジャーナルに残っている必要がある。
fn journal() -> Vec<JournalEntry> {
    vec![
        entry(2, 1, genesis()),
        entry(
            3,
            2,
            IntentExecutionEvent::GateOpened(GateOpened::new(
                event_id(),
                execution_id(),
                slug("practices-discovery"),
                Vec::new(),
            )),
        ),
        entry(
            4,
            3,
            IntentExecutionEvent::StageRevised(StageRevised::new(
                event_id(),
                execution_id(),
                slug("practices-discovery"),
            )),
        ),
    ]
}

/// ループだけを孤立させるための読み手。
#[derive(Debug, Default)]
struct FakeReader {
    journal: Vec<JournalEntry>,
    intents: Vec<(u64, Intent)>,
    checkpoints: BTreeMap<ProjectionName, GlobalSeqNr>,
    /// 最後に受け取った構造化リードモデル (前進と同じ呼出で届く — 系統 (2))。
    ///
    /// 共有ハンドルなのは**テストのスパイだから**である。`ReadModelUpdater` は読み手を
    /// 所有したまま返す口を持たない — 「テストのために表現を公開する」ことを
    /// `coding-rules/abstract-data-type.md` が禁じているので、観測する側の器をテストが
    /// 持つ。設計上の内部可変性ではない (`advance_checkpoint` は `&mut self` のまま)。
    tables: Rc<RefCell<Option<ReadTables>>>,
    /// 1 回目の読取の**後**に届く行 (書込との競合の再現)。2 回目以降の読取から見える。
    ///
    /// 実物では別プロセスの書き手が入れる行であり、取得ループが 2 度読むなら 2 度目に
    /// 現れる。ここではそれをフェイクで決定的に起こす。
    late_row: Rc<RefCell<Option<JournalEntry>>>,
    reads: Rc<RefCell<usize>>,
}

impl JournalReader for FakeReader {
    async fn events_after(&self, after: GlobalSeqNr) -> Result<JournalBatch, JournalReadError> {
        let reads = {
            let mut counter = self.reads.borrow_mut();
            let seen = *counter;
            *counter += 1;
            seen
        };
        let mut rows = self.journal.clone();
        if reads >= 1
            && let Some(row) = self.late_row.borrow().clone()
        {
            rows.push(row);
        }
        let executions: Vec<JournalEntry> = rows
            .iter()
            .filter(|entry| entry.global_seq() > after)
            .cloned()
            .collect();
        let intents: Vec<(u64, Intent)> = self
            .intents
            .iter()
            .filter(|(global, _)| GlobalSeqNr::new(*global) > after)
            .cloned()
            .collect();
        let scanned_to = executions
            .last()
            .map(JournalEntry::global_seq)
            .into_iter()
            .chain(intents.iter().map(|(global, _)| GlobalSeqNr::new(*global)))
            .max();
        Ok(JournalBatch::new(
            executions,
            intents.into_iter().map(|(_, intent)| intent).collect(),
            Vec::new(),
            scanned_to,
        ))
    }

    async fn checkpoint(
        &self,
        projection: &ProjectionName,
    ) -> Result<GlobalSeqNr, JournalReadError> {
        Ok(self
            .checkpoints
            .get(projection)
            .copied()
            .unwrap_or(GlobalSeqNr::ZERO))
    }

    async fn advance_checkpoint(
        &mut self,
        projection: &ProjectionName,
        to: GlobalSeqNr,
        tables: &ReadTables,
    ) -> Result<(), JournalReadError> {
        let current = self
            .checkpoints
            .get(projection)
            .copied()
            .unwrap_or(GlobalSeqNr::ZERO);
        if to < current {
            return Err(JournalReadError::CheckpointRegression {
                projection: projection.clone(),
                current,
                requested: to,
            });
        }
        self.checkpoints.insert(projection.clone(), to);
        *self.tables.borrow_mut() = Some(tables.clone());
        Ok(())
    }
}

fn projection() -> ProjectionName {
    ProjectionName::parse("state-file").expect("投影名は kebab")
}

/// 一時ディレクトリ上の書込先 2 面。
struct Fixture {
    _dir: TempDir,
    state_file: PathBuf,
    audit_shard: PathBuf,
}

impl Fixture {
    fn new() -> Fixture {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let state_file = dir.path().join("aidlc-state.md");
        std::fs::write(&state_file, STATE).expect("出発点を置く");
        let audit_shard = dir.path().join("audit/host-abcd1234.md");
        Fixture {
            _dir: dir,
            state_file,
            audit_shard,
        }
    }

    fn targets(&self) -> ProjectionTargets {
        ProjectionTargets::new(self.state_file.clone(), self.audit_shard.clone())
    }

    fn updater(
        &self,
        journal: Vec<JournalEntry>,
        intents: Vec<(u64, Intent)>,
    ) -> ReadModelUpdater<FakeReader> {
        self.spied_updater(journal, intents).0
    }

    /// 読み手が受け取った構造化リードモデルを覗ける形で組む。
    fn spied_updater(
        &self,
        journal: Vec<JournalEntry>,
        intents: Vec<(u64, Intent)>,
    ) -> (
        ReadModelUpdater<FakeReader>,
        Rc<RefCell<Option<ReadTables>>>,
    ) {
        // genesis を投影せずに済むよう、チェックポイントはその直後から始める。
        let mut checkpoints = BTreeMap::new();
        if !journal.is_empty() {
            checkpoints.insert(projection(), GlobalSeqNr::new(2));
        }
        let spy = Rc::new(RefCell::new(None));
        let updater = ReadModelUpdater::new(
            FakeReader {
                journal,
                intents,
                checkpoints,
                tables: Rc::clone(&spy),
                late_row: Rc::new(RefCell::new(None)),
                reads: Rc::new(RefCell::new(0)),
            },
            projection(),
            self.targets(),
        );
        (updater, spy)
    }

    /// 1 回目の読取の後に 1 行だけ届く読み手で組む (書込との競合の再現)。
    fn racing_updater(
        &self,
        journal: Vec<JournalEntry>,
        intents: Vec<(u64, Intent)>,
        late_row: JournalEntry,
    ) -> (
        ReadModelUpdater<FakeReader>,
        Rc<RefCell<Option<ReadTables>>>,
    ) {
        let mut checkpoints = BTreeMap::new();
        if !journal.is_empty() {
            checkpoints.insert(projection(), GlobalSeqNr::new(2));
        }
        let spy = Rc::new(RefCell::new(None));
        let updater = ReadModelUpdater::new(
            FakeReader {
                journal,
                intents,
                checkpoints,
                tables: Rc::clone(&spy),
                late_row: Rc::new(RefCell::new(Some(late_row))),
                reads: Rc::new(RefCell::new(0)),
            },
            projection(),
            self.targets(),
        );
        (updater, spy)
    }

    fn state(&self) -> String {
        std::fs::read_to_string(&self.state_file).expect("状態ファイルは読める")
    }

    fn shard(&self) -> String {
        std::fs::read_to_string(&self.audit_shard).unwrap_or_default()
    }
}

#[tokio::test]
async fn catching_up_writes_both_faces_and_advances_the_checkpoint() {
    let fixture = Fixture::new();
    let mut updater = fixture.updater(journal(), intents());

    let reached = updater.catch_up().await.expect("キャッチアップ");
    assert_eq!(reached, GlobalSeqNr::new(4), "末尾まで進む");

    // 状態面: `[-]` → `[?]`（GateOpened）→ `[?]`（StageRevised も同じ位置）
    assert!(
        fixture
            .state()
            .contains("- [?] practices-discovery — EXECUTE"),
        "実際: {}",
        fixture.state()
    );
    // 監査面: 空のシャードだったのでヘッダが先に載り、ブロックが 2 つ並ぶ
    let shard = fixture.shard();
    assert!(shard.starts_with("# AI-DLC Audit Log\n"), "実際: {shard:?}");
    assert_eq!(
        shard
            .lines()
            .filter(|line| line.starts_with("**Event**: STAGE_AWAITING_APPROVAL"))
            .count(),
        2
    );
}

#[tokio::test]
async fn a_row_that_lands_between_the_two_reads_is_drawn_on_both_faces_at_one_position() {
    // 取得ループが 2 度読むなら、その間に書込が入りうる。描く材料を 2 つの読取に跨がって
    // 採ると、Markdown 面は古い断面・構造化面は新しい断面になり、`as_of` がチェックポイント
    // を追い越す — 「行はもう新しいのに、そこまで進んでいない」という嘘の断面が残る。
    // 材料はすべて**同じ 1 回の読取**から採らなければならない。
    let fixture = Fixture::new();
    let late = entry(
        5,
        4,
        IntentExecutionEvent::GateOpened(GateOpened::new(
            event_id(),
            execution_id(),
            slug("practices-discovery"),
            Vec::new(),
        )),
    );
    let (mut updater, spy) = fixture.racing_updater(journal(), intents(), late);

    let reached = updater.catch_up().await.expect("キャッチアップ");
    assert_eq!(
        reached,
        GlobalSeqNr::new(5),
        "遅れて届いた行まで進む (読んだ断面が前進先を決める)"
    );

    let tables = spy.borrow().clone().expect("前進と一緒に届く");
    assert_eq!(
        tables.as_of(),
        Some(reached),
        "行の `as_of` はチェックポイントと一致する (追い越さない)"
    );

    // Markdown 面も同じ断面で描かれている — 遅れて届いた行のぶんまでブロックが並ぶ。
    assert_eq!(
        fixture
            .shard()
            .lines()
            .filter(|line| line.starts_with("**Event**: STAGE_AWAITING_APPROVAL"))
            .count(),
        3,
        "読んだ行はすべて監査面に出る: {}",
        fixture.shard()
    );
}

#[tokio::test]
async fn a_second_catch_up_has_nothing_to_do_and_touches_nothing() {
    let fixture = Fixture::new();
    let mut updater = fixture.updater(journal(), intents());

    let first = updater.catch_up().await.expect("1 回目");
    let state_after_first = fixture.state();
    let shard_after_first = fixture.shard();

    let second = updater.catch_up().await.expect("2 回目");
    assert_eq!(second, first, "チェックポイントは動かない");
    assert_eq!(fixture.state(), state_after_first, "状態面は同一バイト");
    assert_eq!(fixture.shard(), shard_after_first, "監査面も同一バイト");
}

#[tokio::test]
async fn an_empty_journal_writes_nothing_at_all() {
    let fixture = Fixture::new();
    let mut updater = fixture.updater(Vec::new(), Vec::new());

    let reached = updater.catch_up().await.expect("キャッチアップ");
    assert_eq!(reached, GlobalSeqNr::ZERO);
    assert_eq!(fixture.state(), STATE, "状態ファイルに触らない");
    assert!(
        !fixture.audit_shard.exists(),
        "書くものが無いのにシャードを生やさない"
    );
}

#[tokio::test]
async fn regenerating_from_zero_twice_yields_identical_bytes() {
    // NFR3 — 同じチェックポイントから何度流しても同一バイト。
    let run = || async {
        let fixture = Fixture::new();
        let mut updater = fixture.updater(journal(), intents());
        updater.catch_up().await.expect("キャッチアップ");
        (fixture.state(), fixture.shard())
    };
    assert_eq!(run().await, run().await);
}

#[tokio::test]
async fn a_projection_failure_leaves_the_checkpoint_where_it_was() {
    // 描けないものに当たったら、書かず・進めず止まる（半端な前進で行を飛ばさない）。
    let fixture = Fixture::new();
    std::fs::write(
        &fixture.state_file,
        "## Stage Progress\n- [ ] other — EXECUTE\n",
    )
    .expect("対象ステージの無い出発点");
    let mut updater = fixture.updater(journal(), intents());

    let error = updater.catch_up().await.expect_err("投影が失敗する");
    assert!(
        error.to_string().starts_with("projection: "),
        "実際: {error}"
    );
    assert!(!fixture.audit_shard.exists(), "監査面へ何も書かない");

    // 次の試行が同じ差分をもう一度読めることが「進んでいない」ことの観測である。
    let retried = updater.catch_up().await.expect_err("同じ失敗を繰り返す");
    assert_eq!(retried.to_string(), error.to_string());
}

#[tokio::test]
async fn a_missing_state_file_is_refused_with_the_verbatim_wording() {
    let fixture = Fixture::new();
    std::fs::remove_file(&fixture.state_file).expect("消す");
    let mut updater = fixture.updater(journal(), intents());

    let error = updater.catch_up().await.expect_err("読めない");
    assert_eq!(
        error.to_string(),
        format!(
            "state file read: State file not found: {}",
            fixture.state_file.display()
        )
    );
}

#[tokio::test]
async fn the_targets_are_carried_as_one_pair() {
    let fixture = Fixture::new();
    let updater = fixture.updater(Vec::new(), Vec::new());
    assert_eq!(updater.targets().state_file(), fixture.state_file);
    assert_eq!(updater.targets().audit_shard(), fixture.audit_shard);
}

#[tokio::test]
async fn an_intent_only_batch_advances_the_checkpoint_without_writing() {
    // intent の行しか無いバッチは書くものが無い — それでもチェックポイントは走査済み位置
    // まで進む（intent 行を毎回再走査しない。issue #56 申し送りの解消）。
    let fixture = Fixture::new();
    let mut updater = fixture.updater(Vec::new(), intents());

    let reached = updater.catch_up().await.expect("キャッチアップ");
    assert_eq!(reached, GlobalSeqNr::new(1), "intent 行の位置まで進む");
    assert_eq!(fixture.state(), STATE, "状態ファイルに触らない");
    assert!(
        !fixture.audit_shard.exists(),
        "書くものが無いのにシャードを生やさない"
    );
}

#[tokio::test]
async fn a_journal_without_a_started_is_plan_unavailable() {
    // 実行のイベントはあるのに `Started` が無い — どの intent の計画かすら分からない
    // (ジャーナルが途中から切り落とされた兆候)。
    let fixture = Fixture::new();
    let journal = vec![entry(
        3,
        2,
        IntentExecutionEvent::GateOpened(GateOpened::new(
            event_id(),
            execution_id(),
            slug("practices-discovery"),
            Vec::new(),
        )),
    )];
    let mut updater = fixture.updater(journal, intents());

    let error = updater.catch_up().await.expect_err("計画の材料が無い");
    assert_eq!(error.to_string(), "plan unavailable");
}

#[tokio::test]
async fn executions_of_two_different_intents_are_refused_as_mixed() {
    // この取得ループは単一 intent の状態ファイル 1 面へ描く — 別 intent の実行を同じ計画で
    // 描かない (intent ごとの振り分けは U7 の駆動設計と対で扱う)。
    let fixture = Fixture::new();
    let other = IntentId::parse("018f3b2c-4d5e-7f60-8abc-def012345678").expect("UUIDv7");
    let journal = vec![
        entry(2, 1, genesis()),
        entry(
            3,
            1,
            IntentExecutionEvent::Started(Started::new(
                event_id(),
                IntentExecutionId::parse("0190cccc-dddd-7eee-8fff-000011112222").expect("UUIDv7"),
                other,
                genesis_intent().stages().to_vec(),
            )),
        ),
        entry(
            4,
            2,
            IntentExecutionEvent::GateOpened(GateOpened::new(
                event_id(),
                execution_id(),
                slug("practices-discovery"),
                Vec::new(),
            )),
        ),
    ];
    let mut updater = fixture.updater(journal, intents());

    let error = updater.catch_up().await.expect_err("混在は拒否");
    assert_eq!(error.to_string(), "mixed intents");
    assert_eq!(fixture.state(), STATE, "状態ファイルに触らない");
}

#[tokio::test]
async fn a_started_without_its_created_is_plan_unavailable() {
    // `Started` は intent の識別子しか運ばない — 指された誕生記録がジャーナルに無ければ
    // 計画は組めない（ジャーナルが途中から切り落とされた兆候）。
    let fixture = Fixture::new();
    let mut updater = fixture.updater(journal(), Vec::new());

    let error = updater.catch_up().await.expect_err("計画の材料が無い");
    assert_eq!(error.to_string(), "plan unavailable");
}

// ---------------------------------------------------------------------------
// 構造化面 (系統 (2)) — 行は前進と同じ呼出で読み手へ渡る (b39 / 裁定 §3)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_structured_rows_reach_the_reader_with_the_advance() {
    let fixture = Fixture::new();
    let (mut updater, spy) = fixture.spied_updater(journal(), intents());

    let reached = updater.catch_up().await.expect("キャッチアップ");

    let received = spy.borrow();
    let tables = received.as_ref().expect("前進と一緒に行が届く");
    assert_eq!(
        tables.as_of(),
        Some(reached),
        "行の as_of は前進後のチェックポイントと同じ位置を指す"
    );
    // 差分はチェックポイント 2 以降だが、行は**全履歴**から作られている（誕生記録を含む）。
    assert_eq!(tables.executions().len(), 1);
    assert_eq!(tables.intents().len(), 1);
    assert_eq!(
        tables.next_answers().len(),
        4,
        "1 実行につき 4 つの要求の形すべてに答えが在る"
    );
}

#[tokio::test]
async fn an_empty_journal_hands_the_reader_no_rows_at_all() {
    // 差分が空なら前進そのものが起きない — 行も渡らない。
    let fixture = Fixture::new();
    let (mut updater, spy) = fixture.spied_updater(Vec::new(), Vec::new());

    updater.catch_up().await.expect("キャッチアップ");

    assert!(spy.borrow().is_none(), "前進が起きないので行も渡らない");
}

#[tokio::test]
async fn a_second_catch_up_leaves_the_rows_as_the_first_one_left_them() {
    let fixture = Fixture::new();
    let (mut updater, spy) = fixture.spied_updater(journal(), intents());

    updater.catch_up().await.expect("1 回目");
    let after_first = spy.borrow().clone().expect("1 回目で届く");

    updater.catch_up().await.expect("2 回目");
    assert_eq!(
        spy.borrow().as_ref(),
        Some(&after_first),
        "差分が無ければ行も書き直さない"
    );
}
