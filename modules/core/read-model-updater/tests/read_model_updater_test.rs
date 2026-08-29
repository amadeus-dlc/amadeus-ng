//! 取得ループ（`ReadModelUpdater`）の契約 — checkpoint → 差分読取 → 投影 → 書込 → 前進。
//!
//! 読み手はフェイクである。実 `JournalReaderImpl` の読み方は
//! `journal_reader_impl_test.rs` が固定しており、ここが見るのは**ループの約束**（空差分の
//! 扱い・書いてから進める順序・再生成の冪等）だからである。フェイクなら、まだ投影規則の
//! 裁定が降りていないイベントを混ぜずに、ループだけを孤立させて観測できる。

// テストコードでは unwrap / expect / panic を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    GateOpened, Intent, IntentExecutionEvent, IntentExecutionId, IntentId, StageDisplay,
    StageEntry, StageRevised, StartRequest, Started, WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
    WorkflowDefinitionId,
};
use core_read_model_updater::orchestration::{
    GlobalSeqNr, JournalEntry, JournalReadError, JournalReader, ProjectionName, ProjectionTargets,
    ReadModelUpdater,
};
use tempfile::TempDir;

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

fn entry(global: u64, event: IntentExecutionEvent) -> JournalEntry {
    JournalEntry::new(
        GlobalSeqNr::new(global),
        IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
        global as usize,
        at(),
        event,
    )
}

/// 表示属性を運ぶ genesis（取得ループはここから計画を引く）。
fn genesis() -> IntentExecutionEvent {
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
    IntentExecutionEvent::Started(Started::new(
        Intent::from_material(
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
        )
        .expect("合成計画は Intent の不変条件を満たす"),
    ))
}

/// genesis + 投影規則が確定しているイベント 2 件。
///
/// チェックポイントは genesis の直後（1）から始める — 取得ループが見るのは差分だけであり、
/// genesis の状態面（新規スキャフォールド）は本 Bolt の射程外だからである。genesis 自身は
/// **計画の供給元**としてジャーナルに残っている必要がある。
fn journal() -> Vec<JournalEntry> {
    vec![
        entry(1, genesis()),
        entry(
            2,
            IntentExecutionEvent::GateOpened(GateOpened::new(
                slug("practices-discovery"),
                Vec::new(),
            )),
        ),
        entry(
            3,
            IntentExecutionEvent::StageRevised(StageRevised::new(slug("practices-discovery"))),
        ),
    ]
}

/// ループだけを孤立させるための読み手。
#[derive(Debug, Default)]
struct FakeReader {
    journal: Vec<JournalEntry>,
    checkpoints: BTreeMap<ProjectionName, GlobalSeqNr>,
}

impl JournalReader for FakeReader {
    async fn events_after(
        &self,
        after: GlobalSeqNr,
    ) -> Result<Vec<JournalEntry>, JournalReadError> {
        Ok(self
            .journal
            .iter()
            .filter(|entry| entry.global_seq() > after)
            .cloned()
            .collect())
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

    fn updater(&self, journal: Vec<JournalEntry>) -> ReadModelUpdater<FakeReader> {
        // genesis を投影せずに済むよう、チェックポイントはその直後から始める。
        let mut checkpoints = BTreeMap::new();
        if !journal.is_empty() {
            checkpoints.insert(projection(), GlobalSeqNr::new(1));
        }
        ReadModelUpdater::new(
            FakeReader {
                journal,
                checkpoints,
            },
            projection(),
            self.targets(),
        )
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
    let mut updater = fixture.updater(journal());

    let reached = updater.catch_up().await.expect("キャッチアップ");
    assert_eq!(reached, GlobalSeqNr::new(3), "末尾まで進む");

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
async fn a_second_catch_up_has_nothing_to_do_and_touches_nothing() {
    let fixture = Fixture::new();
    let mut updater = fixture.updater(journal());

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
    let mut updater = fixture.updater(Vec::new());

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
        let mut updater = fixture.updater(journal());
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
    let mut updater = fixture.updater(journal());

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
    let mut updater = fixture.updater(journal());

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
    let updater = fixture.updater(Vec::new());
    assert_eq!(updater.targets().state_file(), fixture.state_file);
    assert_eq!(updater.targets().audit_shard(), fixture.audit_shard);
}
