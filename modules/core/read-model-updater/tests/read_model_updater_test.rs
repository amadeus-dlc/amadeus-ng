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
    ArtifactPaths, Created, GateOpened, Intent, IntentEventId, IntentExecutionEvent,
    IntentExecutionEventId, IntentExecutionId, IntentId, PracticesAffirmed, StageDisplay,
    StageEntries, StageEntry, StageRevised, StartRequest, Started, WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
    WorkflowDefinitionId,
};
use core_command_domain::workspace::PromotedSection;
use core_command_domain::workspace::{PromotedSections, RuleLines};
use core_read_model_updater::orchestration::{
    GlobalSeqNr, JournalBatch, JournalEntry, JournalReadError, JournalReader,
    OrchestrationReadModelUpdater, ProjectionName, ProjectionTargets, PublicationBatch,
    ReadModelUpdateError, ReadModelUpdater, SteeringSource, StructuredReadModelUpdater,
};
use core_read_model_updater::read_tables::{ReadTables, SteeringTables};
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

## Current Status
- **Last Updated**: 2026-08-20T00:00:00Z
";

/// メモリ層の正本 2 本（b49 — 昇格の書込先）。
const TEAM_MD: &str = "# Team\n\n## Way of Working\nold way.\n";
const PROJECT_MD: &str = "# Project\n\n## Mandated\n\n## Forbidden\n";

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
            StageEntries::new(vec![stage(
                "practices-discovery",
                "2.2",
                "aidlc-pipeline-deploy-agent",
            )])
            .expect("フィクスチャの計画は不変条件を満たす"),
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
        intent.stages().clone(),
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
                ArtifactPaths::empty(),
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
    plan_fingerprints: BTreeMap<String, core_read_model_updater::read_tables::PlanFingerprintRow>,
    code_generation_approvals:
        BTreeMap<String, core_read_model_updater::read_tables::CodeGenerationApprovalRow>,
    testing: Option<core_read_model_updater::read_tables::TestingTables>,
    publications: Rc<RefCell<BTreeMap<ProjectionName, (PublicationBatch, bool)>>>,
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
    lose_history_after_probe: bool,
    /// 保存済みの steering 面 (差し替えのたびに丸ごと入れ替わる — 実装と同じ約束)。
    steering: Rc<RefCell<Option<SteeringTables>>>,
    /// steering 面を差し替えた回数 (再投影が走ったかどうかの観測点)。
    steering_writes: Rc<RefCell<usize>>,
    /// 同居する成果物監査の行 (global 通番付き)。
    artifacts: Vec<core_read_model_updater::orchestration::ArtifactJournalEntry>,
    /// 同居するセッション監査の行 (global 通番付き)。
    sessions: Vec<core_read_model_updater::orchestration::SessionJournalEntry>,
}

impl JournalReader for FakeReader {
    fn prepare_read_model(&mut self) -> Result<(), ReadModelUpdateError> {
        Ok(())
    }

    async fn pending_publication(
        &self,
        projection: &ProjectionName,
    ) -> Result<Option<PublicationBatch>, JournalReadError> {
        Ok(self
            .publications
            .borrow()
            .get(projection)
            .filter(|(_, committed)| !committed)
            .map(|(batch, _)| batch.clone()))
    }

    async fn events_through(&self, to: GlobalSeqNr) -> Result<JournalBatch, JournalReadError> {
        let rows: Vec<_> = self
            .journal
            .iter()
            .filter(|entry| entry.global_seq() <= to)
            .cloned()
            .collect();
        let intents: Vec<_> = self
            .intents
            .iter()
            .filter(|(position, _)| GlobalSeqNr::new(*position) <= to)
            .cloned()
            .collect();
        let artifacts: Vec<_> = self
            .artifacts
            .iter()
            .filter(|entry| entry.global_seq() <= to)
            .cloned()
            .collect();
        let sessions: Vec<_> = self
            .sessions
            .iter()
            .filter(|entry| entry.global_seq() <= to)
            .cloned()
            .collect();
        let last = rows
            .last()
            .map(JournalEntry::global_seq)
            .into_iter()
            .chain(
                intents
                    .iter()
                    .map(|(position, _)| GlobalSeqNr::new(*position)),
            )
            .chain(artifacts.iter().map(|entry| entry.global_seq()))
            .chain(sessions.iter().map(|entry| entry.global_seq()))
            .max();
        Ok(JournalBatch::new(
            rows,
            intents.into_iter().map(|(_, intent)| intent).collect(),
            Vec::new(),
            last,
        )
        .with_artifacts(artifacts)
        .with_sessions(sessions))
    }

    async fn publish(
        &mut self,
        projection: &ProjectionName,
        candidate: &PublicationBatch,
        tables: &ReadTables,
    ) -> Result<(), ReadModelUpdateError> {
        let batch = candidate.clone();
        self.publications
            .borrow_mut()
            .insert(projection.clone(), (batch.clone(), false));
        batch.apply()?;
        self.advance_checkpoint(projection, batch.to(), tables)
            .await?;
        self.publications
            .borrow_mut()
            .insert(projection.clone(), (batch, true));
        Ok(())
    }
    async fn events_after(&self, after: GlobalSeqNr) -> Result<JournalBatch, JournalReadError> {
        let reads = {
            let mut counter = self.reads.borrow_mut();
            let seen = *counter;
            *counter += 1;
            seen
        };
        if reads > 0 && self.lose_history_after_probe {
            return Ok(JournalBatch::empty());
        }
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
        let artifacts: Vec<_> = self
            .artifacts
            .iter()
            .filter(|entry| entry.global_seq() > after)
            .cloned()
            .collect();
        let sessions: Vec<_> = self
            .sessions
            .iter()
            .filter(|entry| entry.global_seq() > after)
            .cloned()
            .collect();
        let scanned_to = executions
            .last()
            .map(JournalEntry::global_seq)
            .into_iter()
            .chain(intents.iter().map(|(global, _)| GlobalSeqNr::new(*global)))
            .chain(artifacts.iter().map(|entry| entry.global_seq()))
            .chain(sessions.iter().map(|entry| entry.global_seq()))
            .max();
        Ok(JournalBatch::new(
            executions,
            intents.into_iter().map(|(_, intent)| intent).collect(),
            Vec::new(),
            scanned_to,
        )
        .with_artifacts(artifacts)
        .with_sessions(sessions))
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

    async fn replace_plan_fingerprint(
        &mut self,
        row: &core_read_model_updater::read_tables::PlanFingerprintRow,
    ) -> Result<(), JournalReadError> {
        self.plan_fingerprints
            .insert(row.id().to_string(), row.clone());
        Ok(())
    }
    async fn replace_code_generation_approval(
        &mut self,
        row: &core_read_model_updater::read_tables::CodeGenerationApprovalRow,
    ) -> Result<(), JournalReadError> {
        self.code_generation_approvals
            .insert(row.id().to_string(), row.clone());
        Ok(())
    }
    async fn testing_source_digest(&self) -> Result<Option<String>, JournalReadError> {
        Ok(self
            .testing
            .as_ref()
            .map(|tables| tables.source_digest().to_string()))
    }
    async fn replace_testing(
        &mut self,
        tables: &core_read_model_updater::read_tables::TestingTables,
    ) -> Result<(), JournalReadError> {
        self.testing = Some(tables.clone());
        Ok(())
    }
    async fn steering_source_digest(&self) -> Result<Option<String>, JournalReadError> {
        Ok(self
            .steering
            .borrow()
            .as_ref()
            .map(|tables| tables.source_digest().to_string()))
    }

    async fn replace_steering(&mut self, tables: &SteeringTables) -> Result<(), JournalReadError> {
        *self.steering.borrow_mut() = Some(tables.clone());
        *self.steering_writes.borrow_mut() += 1;
        Ok(())
    }
}

fn projection() -> ProjectionName {
    ProjectionName::parse("state-file").expect("投影名は kebab")
}

/// 一時ディレクトリ上の書込先 2 面。
struct Fixture {
    publications: Rc<RefCell<BTreeMap<ProjectionName, (PublicationBatch, bool)>>>,
    _dir: TempDir,
    state_file: PathBuf,
    audit_shard: PathBuf,
    memory_dir: PathBuf,
    steering: Rc<RefCell<Option<SteeringTables>>>,
    steering_writes: Rc<RefCell<usize>>,
}

impl Fixture {
    fn new() -> Fixture {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let state_file = dir.path().join("aidlc-state.md");
        std::fs::write(&state_file, STATE).expect("出発点を置く");
        let audit_shard = dir.path().join("audit/host-abcd1234.md");
        let memory_dir = dir.path().join("memory");
        std::fs::create_dir_all(memory_dir.join("phases")).expect("memory 層を作る");
        std::fs::write(
            memory_dir.join("org.md"),
            "# Org\n\nALWAYS keep the audit record.\n",
        )
        .expect("規則を置く");
        Fixture {
            publications: Rc::new(RefCell::new(BTreeMap::new())),
            _dir: dir,
            state_file,
            audit_shard,
            memory_dir,
            steering: Rc::new(RefCell::new(None)),
            steering_writes: Rc::new(RefCell::new(0)),
        }
    }

    /// memory 層のファイルを 1 本置き換える (参照入力の編集)。
    fn write_rule(&self, relative: &str, text: &str) {
        std::fs::write(self.memory_dir.join(relative), text).expect("規則を書く");
    }

    /// memory 層のファイルを 1 本読む。
    fn rule(&self, relative: &str) -> String {
        std::fs::read_to_string(self.memory_dir.join(relative)).expect("規則は読める")
    }

    /// memory 層のファイルを 1 本消す。
    fn remove_rule(&self, relative: &str) {
        std::fs::remove_file(self.memory_dir.join(relative)).expect("規則を消す");
    }

    fn steering_source(&self) -> SteeringSource {
        SteeringSource::new(self.memory_dir.clone())
    }

    /// steering 面を差し替えた回数。
    fn steering_writes(&self) -> usize {
        *self.steering_writes.borrow()
    }

    /// 保存済みの steering 面。
    fn steering(&self) -> Option<SteeringTables> {
        self.steering.borrow().clone()
    }

    fn targets(&self) -> ProjectionTargets {
        ProjectionTargets::new(
            self.state_file.clone(),
            self.audit_shard.clone(),
            self.memory_dir.clone(),
        )
    }

    fn updater(
        &self,
        journal: Vec<JournalEntry>,
        intents: Vec<(u64, Intent)>,
    ) -> OrchestrationReadModelUpdater<FakeReader> {
        self.spied_updater(journal, intents).0
    }

    /// 読み手が受け取った構造化リードモデルを覗ける形で組む。
    fn spied_updater(
        &self,
        journal: Vec<JournalEntry>,
        intents: Vec<(u64, Intent)>,
    ) -> (
        OrchestrationReadModelUpdater<FakeReader>,
        Rc<RefCell<Option<ReadTables>>>,
    ) {
        // genesis を投影せずに済むよう、チェックポイントはその直後から始める。
        let mut checkpoints = BTreeMap::new();
        if !journal.is_empty() {
            checkpoints.insert(projection(), GlobalSeqNr::new(2));
        }
        let spy = Rc::new(RefCell::new(None));
        let updater = OrchestrationReadModelUpdater::new(
            FakeReader {
                plan_fingerprints: BTreeMap::new(),
                code_generation_approvals: BTreeMap::new(),
                testing: None,
                journal,
                intents,
                checkpoints,
                tables: Rc::clone(&spy),
                late_row: Rc::new(RefCell::new(None)),
                reads: Rc::new(RefCell::new(0)),
                lose_history_after_probe: false,
                steering: Rc::clone(&self.steering),
                steering_writes: Rc::clone(&self.steering_writes),
                publications: Rc::clone(&self.publications),
                artifacts: Vec::new(),
                sessions: Vec::new(),
            },
            projection(),
            self.targets(),
            self.steering_source(),
        );
        (updater, spy)
    }

    /// 参照入力 (steering) を**別のディレクトリ**に向けて組む。
    ///
    /// 取得ループは投影面 (`ProjectionTargets`) と参照入力 (`SteeringSource`) を別の引数で
    /// 受け取る。両者が同じ memory ディレクトリを指すのは合成ルートの配線であって、ループの
    /// 契約ではない — 投影面の読取失敗だけを孤立させて観測するにはここを分ける。
    fn updater_with_isolated_steering(
        &self,
        journal: Vec<JournalEntry>,
        intents: Vec<(u64, Intent)>,
        steering_dir: PathBuf,
    ) -> OrchestrationReadModelUpdater<FakeReader> {
        let mut checkpoints = BTreeMap::new();
        if !journal.is_empty() {
            checkpoints.insert(projection(), GlobalSeqNr::new(2));
        }
        OrchestrationReadModelUpdater::new(
            FakeReader {
                plan_fingerprints: BTreeMap::new(),
                code_generation_approvals: BTreeMap::new(),
                testing: None,
                journal,
                intents,
                checkpoints,
                tables: Rc::new(RefCell::new(None)),
                late_row: Rc::new(RefCell::new(None)),
                reads: Rc::new(RefCell::new(0)),
                lose_history_after_probe: false,
                steering: Rc::clone(&self.steering),
                steering_writes: Rc::clone(&self.steering_writes),
                publications: Rc::clone(&self.publications),
                artifacts: Vec::new(),
                sessions: Vec::new(),
            },
            projection(),
            self.targets(),
            SteeringSource::new(steering_dir),
        )
    }

    /// 1 回目の読取の後に 1 行だけ届く読み手で組む (書込との競合の再現)。
    fn racing_updater(
        &self,
        journal: Vec<JournalEntry>,
        intents: Vec<(u64, Intent)>,
        late_row: JournalEntry,
    ) -> (
        OrchestrationReadModelUpdater<FakeReader>,
        Rc<RefCell<Option<ReadTables>>>,
    ) {
        let mut checkpoints = BTreeMap::new();
        if !journal.is_empty() {
            checkpoints.insert(projection(), GlobalSeqNr::new(2));
        }
        let spy = Rc::new(RefCell::new(None));
        let updater = OrchestrationReadModelUpdater::new(
            FakeReader {
                plan_fingerprints: BTreeMap::new(),
                code_generation_approvals: BTreeMap::new(),
                testing: None,
                journal,
                intents,
                checkpoints,
                tables: Rc::clone(&spy),
                late_row: Rc::new(RefCell::new(Some(late_row))),
                reads: Rc::new(RefCell::new(0)),
                lose_history_after_probe: false,
                steering: Rc::clone(&self.steering),
                steering_writes: Rc::clone(&self.steering_writes),
                publications: Rc::clone(&self.publications),
                artifacts: Vec::new(),
                sessions: Vec::new(),
            },
            projection(),
            self.targets(),
            self.steering_source(),
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

    updater.update_read_models().await.expect("更新");

    let reached = updater
        .checkpoint()
        .await
        .expect("チェックポイントを読める");
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
            ArtifactPaths::empty(),
        )),
    );
    let (mut updater, spy) = fixture.racing_updater(journal(), intents(), late);

    updater.update_read_models().await.expect("更新");

    let reached = updater
        .checkpoint()
        .await
        .expect("チェックポイントを読める");
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
async fn a_second_update_has_nothing_to_do_and_touches_nothing() {
    let fixture = Fixture::new();
    let mut updater = fixture.updater(journal(), intents());

    updater.update_read_models().await.expect("1 回目");

    let first = updater
        .checkpoint()
        .await
        .expect("チェックポイントを読める");
    let state_after_first = fixture.state();
    let shard_after_first = fixture.shard();

    updater.update_read_models().await.expect("2 回目");

    let second = updater
        .checkpoint()
        .await
        .expect("チェックポイントを読める");
    assert_eq!(second, first, "チェックポイントは動かない");
    assert_eq!(fixture.state(), state_after_first, "状態面は同一バイト");
    assert_eq!(fixture.shard(), shard_after_first, "監査面も同一バイト");
}

#[tokio::test]
async fn an_empty_journal_writes_nothing_at_all() {
    let fixture = Fixture::new();
    let mut updater = fixture.updater(Vec::new(), Vec::new());

    updater.update_read_models().await.expect("更新");

    let reached = updater
        .checkpoint()
        .await
        .expect("チェックポイントを読める");
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
        updater.update_read_models().await.expect("更新");
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

    let error = updater
        .update_read_models()
        .await
        .expect_err("投影が失敗する");
    assert!(
        error.to_string().starts_with("projection: "),
        "実際: {error}"
    );
    assert!(!fixture.audit_shard.exists(), "監査面へ何も書かない");

    // 次の試行が同じ差分をもう一度読めることが「進んでいない」ことの観測である。
    let retried = updater
        .update_read_models()
        .await
        .expect_err("同じ失敗を繰り返す");
    assert_eq!(retried.to_string(), error.to_string());
}

#[tokio::test]
async fn a_missing_state_file_is_refused_with_the_verbatim_wording() {
    let fixture = Fixture::new();
    std::fs::remove_file(&fixture.state_file).expect("消す");
    let mut updater = fixture.updater(journal(), intents());

    let error = updater.update_read_models().await.expect_err("読めない");
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
    assert_eq!(
        updater.targets().team_md(),
        fixture.memory_dir.join("team.md")
    );
    assert_eq!(
        updater.targets().project_md(),
        fixture.memory_dir.join("project.md")
    );
}

// ---- b49: メモリ層の投影面 ----

/// 昇格の事実（節 1 つ + 規則 1 本ずつ）。
fn practices_affirmed() -> IntentExecutionEvent {
    IntentExecutionEvent::PracticesAffirmed(PracticesAffirmed::new(
        event_id(),
        execution_id(),
        slug("practices-discovery"),
        "owner",
        PromotedSections::new(vec![PromotedSection::new(
            "Way of Working",
            "trunk-based.\n",
        )])
        .expect("見出しは 1 つ"),
        RuleLines::new(vec!["ALWAYS review. (affirmed 2026-09-05)".to_string()]),
        RuleLines::empty(),
    ))
}

/// メモリ層 2 本を書いたフィクスチャで昇格を 1 件投影する。
#[tokio::test]
async fn a_promotion_rewrites_the_memory_layer_and_the_other_faces() {
    let fixture = Fixture::new();
    fixture.write_rule("team.md", TEAM_MD);
    fixture.write_rule("project.md", PROJECT_MD);
    let mut updater = fixture.updater(
        vec![entry(2, 1, genesis()), entry(3, 2, practices_affirmed())],
        intents(),
    );

    updater.update_read_models().await.expect("更新");

    assert_eq!(
        fixture.rule("team.md"),
        "# Team\n\n## Way of Working\ntrunk-based.\n"
    );
    assert_eq!(
        fixture.rule("project.md"),
        "# Project\n\n## Mandated\n\nALWAYS review. (affirmed 2026-09-05)\n## Forbidden\n"
    );
    assert!(
        fixture
            .state()
            .contains("- **Practices Affirmed Timestamp**: 2026-08-21T09:14:07Z\n"),
        "{}",
        fixture.state()
    );
    assert!(
        std::fs::read_to_string(&fixture.audit_shard)
            .expect("シャードは在る")
            .contains("**Event**: PRACTICES_AFFIRMED\n")
    );
}

/// メモリ層 2 本が揃っていなければ載せない — 昇格を描けと言われたら fail-closed で止まる。
#[tokio::test]
async fn a_promotion_without_both_memory_files_is_refused_and_nothing_advances() {
    let fixture = Fixture::new();
    // team.md だけ在る（片方だけは載せない）。
    fixture.write_rule("team.md", TEAM_MD);
    let mut updater = fixture.updater(
        vec![entry(2, 1, genesis()), entry(3, 2, practices_affirmed())],
        intents(),
    );

    let error = updater
        .update_read_models()
        .await
        .expect_err("面が無ければ描けない");
    assert_eq!(error.to_string(), "projection: memory files missing");
    assert_eq!(fixture.state(), STATE, "状態ファイルに触らない");
    assert_eq!(fixture.rule("team.md"), TEAM_MD, "正本に触らない");
}

/// メモリ層を触らない更新は 2 本の mtime を動かさない（dirty のときだけ書く）。
#[tokio::test]
async fn a_update_that_touches_no_memory_face_leaves_both_files_untouched() {
    let fixture = Fixture::new();
    fixture.write_rule("team.md", TEAM_MD);
    fixture.write_rule("project.md", PROJECT_MD);
    let before = (
        modified(&fixture.memory_dir.join("team.md")),
        modified(&fixture.memory_dir.join("project.md")),
    );
    let mut updater = fixture.updater(journal(), intents());

    updater.update_read_models().await.expect("更新");

    assert_eq!(fixture.rule("team.md"), TEAM_MD);
    assert_eq!(fixture.rule("project.md"), PROJECT_MD);
    assert_eq!(
        (
            modified(&fixture.memory_dir.join("team.md")),
            modified(&fixture.memory_dir.join("project.md")),
        ),
        before,
        "書き替えていない面は 1 バイトも書かない"
    );
}

/// 在るのに読めないメモリ層は blocking である（不在と混ぜない）。
///
/// 参照入力 (steering) は別のディレクトリへ向けてある — 同じ memory ディレクトリを指すと
/// `update_steering` が先に同じファイルで倒れ、投影面の読取失敗だけを観測できない。
#[tokio::test]
async fn a_memory_file_that_exists_but_cannot_be_read_stops_the_update() {
    let fixture = Fixture::new();
    // `team.md` の位置にディレクトリを置く — `exists()` は真だが `read_to_string` は失敗する。
    std::fs::create_dir(fixture.memory_dir.join("team.md")).expect("ディレクトリを置く");
    fixture.write_rule("project.md", PROJECT_MD);
    let isolated = fixture.memory_dir.join("isolated");
    std::fs::create_dir_all(&isolated).expect("参照入力の置き場");
    let mut updater = fixture.updater_with_isolated_steering(
        vec![entry(2, 1, genesis()), entry(3, 2, practices_affirmed())],
        intents(),
        isolated,
    );

    let error = updater
        .update_read_models()
        .await
        .expect_err("在るのに読めないので止まる");
    assert!(
        error.to_string().starts_with("memory file read: "),
        "{error}"
    );
    assert_eq!(fixture.state(), STATE, "状態ファイルに触らない");
}

/// 書けないメモリ層も blocking である（read-only バリア）。
#[cfg(unix)]
#[tokio::test]
async fn a_read_only_memory_file_stops_the_update_with_its_material() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new();
    fixture.write_rule("team.md", TEAM_MD);
    fixture.write_rule("project.md", PROJECT_MD);
    let project = fixture.memory_dir.join("project.md");
    let mut permissions = std::fs::metadata(&project).unwrap().permissions();
    permissions.set_mode(0o444);
    std::fs::set_permissions(&project, permissions).unwrap();

    let mut updater = fixture.updater(
        vec![entry(2, 1, genesis()), entry(3, 2, practices_affirmed())],
        intents(),
    );
    let error = updater
        .update_read_models()
        .await
        .expect_err("書けないので止まる");
    assert!(
        error
            .to_string()
            .starts_with("memory file write: read-only target at "),
        "{error}"
    );

    // 後片付け（tempdir の削除が permission に引っかからないように戻す）。
    let mut permissions = std::fs::metadata(&project).unwrap().permissions();
    permissions.set_mode(0o644);
    std::fs::set_permissions(&project, permissions).unwrap();
    // project.md が先なので team.md は無傷のままである。
    assert_eq!(fixture.rule("team.md"), TEAM_MD);
}

/// 置き場そのものが書けないときは OS の I/O 文言を材料に運ぶ。
#[cfg(unix)]
#[tokio::test]
async fn a_memory_directory_that_cannot_be_written_stops_the_update_with_the_io_material() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new();
    fixture.write_rule("team.md", TEAM_MD);
    fixture.write_rule("project.md", PROJECT_MD);
    // 置き場を読取専用にする — ファイル自体は書けるが、原子的書込の一時ファイルが作れない。
    let mut permissions = std::fs::metadata(&fixture.memory_dir)
        .unwrap()
        .permissions();
    permissions.set_mode(0o555);
    std::fs::set_permissions(&fixture.memory_dir, permissions).unwrap();

    let mut updater = fixture.updater(
        vec![entry(2, 1, genesis()), entry(3, 2, practices_affirmed())],
        intents(),
    );
    let error = updater
        .update_read_models()
        .await
        .expect_err("置き場が書けないので止まる");

    // 後片付け（tempdir の削除が permission に引っかからないように戻す）。
    let mut permissions = std::fs::metadata(&fixture.memory_dir)
        .unwrap()
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fixture.memory_dir, permissions).unwrap();

    let rendered = error.to_string();
    assert!(rendered.starts_with("memory file write: "), "{rendered}");
    assert!(
        !rendered.contains("read-only target"),
        "read-only バリアではなく I/O の材料が上がる: {rendered}"
    );
}

/// ファイルの最終更新時刻（`dirty` でない面を書いていないことの観測点）。
fn modified(path: &std::path::Path) -> std::time::SystemTime {
    std::fs::metadata(path)
        .expect("メタデータは読める")
        .modified()
        .expect("mtime は読める")
}

#[tokio::test]
async fn an_intent_only_batch_advances_the_checkpoint_without_writing() {
    // intent の行しか無いバッチは書くものが無い — それでもチェックポイントは走査済み位置
    // まで進む（intent 行を毎回再走査しない。issue #56 申し送りの解消）。
    let fixture = Fixture::new();
    let mut updater = fixture.updater(Vec::new(), intents());

    updater.update_read_models().await.expect("更新");

    let reached = updater
        .checkpoint()
        .await
        .expect("チェックポイントを読める");
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
            ArtifactPaths::empty(),
        )),
    )];
    let mut updater = fixture.updater(journal, intents());

    let error = updater
        .update_read_models()
        .await
        .expect_err("計画の材料が無い");
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
                genesis_intent().stages().clone(),
            )),
        ),
        entry(
            4,
            2,
            IntentExecutionEvent::GateOpened(GateOpened::new(
                event_id(),
                execution_id(),
                slug("practices-discovery"),
                ArtifactPaths::empty(),
            )),
        ),
    ];
    let mut updater = fixture.updater(journal, intents());

    let error = updater.update_read_models().await.expect_err("混在は拒否");
    assert_eq!(error.to_string(), "mixed intents");
    assert_eq!(fixture.state(), STATE, "状態ファイルに触らない");
}

#[tokio::test]
async fn a_started_without_its_created_is_plan_unavailable() {
    // `Started` は intent の識別子しか運ばない — 指された誕生記録がジャーナルに無ければ
    // 計画は組めない（ジャーナルが途中から切り落とされた兆候）。
    let fixture = Fixture::new();
    let mut updater = fixture.updater(journal(), Vec::new());

    let error = updater
        .update_read_models()
        .await
        .expect_err("計画の材料が無い");
    assert_eq!(error.to_string(), "plan unavailable");
}

// ---------------------------------------------------------------------------
// 構造化面 (系統 (2)) — 行は前進と同じ呼出で読み手へ渡る (b39 / 裁定 §3)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_structured_rows_reach_the_reader_with_the_advance() {
    let fixture = Fixture::new();
    let (mut updater, spy) = fixture.spied_updater(journal(), intents());

    updater.update_read_models().await.expect("更新");

    let reached = updater
        .checkpoint()
        .await
        .expect("チェックポイントを読める");

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

    updater.update_read_models().await.expect("更新");

    assert!(spy.borrow().is_none(), "前進が起きないので行も渡らない");
}

#[tokio::test]
async fn a_second_update_leaves_the_rows_as_the_first_one_left_them() {
    let fixture = Fixture::new();
    let (mut updater, spy) = fixture.spied_updater(journal(), intents());

    updater.update_read_models().await.expect("1 回目");
    let after_first = spy.borrow().clone().expect("1 回目で届く");

    updater.update_read_models().await.expect("2 回目");
    assert_eq!(
        spy.borrow().as_ref(),
        Some(&after_first),
        "差分が無ければ行も書き直さない"
    );
}

// ---- 参照入力 (steering) ----

#[tokio::test]
async fn the_first_update_projects_the_memory_layer_it_finds() {
    let fixture = Fixture::new();
    fixture.write_rule(
        "phases/inception.md",
        "# Inception\n\nALWAYS confirm the scope.\n",
    );
    let mut updater = fixture.updater(journal(), intents());
    updater.update_read_models().await.expect("更新");

    assert_eq!(fixture.steering_writes(), 1);
    let steering = fixture.steering().expect("steering 面が書かれている");
    assert_eq!(steering.plans().len(), 5, "束は phase の関数 (5 フェーズ)");
    let inception = steering
        .plans()
        .iter()
        .find(|row| row.phase() == "inception")
        .expect("inception の行");
    assert_eq!(inception.part_count(), 1);
    assert!(
        inception.delivered_paths().contains("org.md")
            && inception.delivered_paths().contains("phases/inception.md"),
        "実際: {}",
        inception.delivered_paths()
    );
}

#[tokio::test]
async fn an_unchanged_memory_layer_is_not_reprojected() {
    // 参照入力を読み直すのは毎回だが、**書き替えるのはダイジェストが動いたときだけ**である。
    // 毎回書き替えると、規則を 1 文字も触っていないのに束のバイトが動きうる。
    let fixture = Fixture::new();
    let mut updater = fixture.updater(journal(), intents());
    updater.update_read_models().await.expect("1 回目");
    assert_eq!(fixture.steering_writes(), 1);
    updater.update_read_models().await.expect("2 回目");
    assert_eq!(fixture.steering_writes(), 1, "同じ参照入力では書き替えない");
}

#[tokio::test]
async fn an_edited_rule_file_is_reprojected_even_when_the_journal_has_not_moved() {
    // ジャーナル差分が空でも参照入力は見る — 規則は人が編集するので、イベントを伴わない。
    let fixture = Fixture::new();
    let mut updater = fixture.updater(journal(), intents());
    updater.update_read_models().await.expect("1 回目");
    let before = fixture.steering().expect("1 回目の面");

    fixture.write_rule("org.md", "# Org\n\n変更した規則\n");
    updater
        .update_read_models()
        .await
        .expect("2 回目 — ジャーナル差分は空");

    assert_eq!(fixture.steering_writes(), 2);
    let after = fixture.steering().expect("2 回目の面");
    assert_ne!(before.source_digest(), after.source_digest());
    assert_ne!(
        before
            .plans()
            .first()
            .map(|row| row.bundle_digest().to_string()),
        after
            .plans()
            .first()
            .map(|row| row.bundle_digest().to_string()),
        "束のダイジェストも動く"
    );
}

#[tokio::test]
async fn a_rule_file_that_disappears_is_normal_and_shrinks_the_bundle() {
    let fixture = Fixture::new();
    let mut updater = fixture.updater(journal(), intents());
    updater.update_read_models().await.expect("1 回目");
    fixture.remove_rule("org.md");
    updater.update_read_models().await.expect("欠損は正常");

    let steering = fixture.steering().expect("steering 面");
    assert_eq!(fixture.steering_writes(), 2);
    for row in steering.plans() {
        assert_eq!(row.part_count(), 0, "配る規則が 1 本も無い");
        assert_eq!(row.delivered_paths(), "[]");
    }
}

#[tokio::test]
async fn a_missing_memory_directory_is_normal_too() {
    // 規則未整備のワークスペース — ディレクトリごと無くても止まらない (bare run-stage)。
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let source = SteeringSource::new(dir.path().join("absent"));
    let rules = source.read().expect("欠損は正常");
    assert_eq!(rules, Default::default());
}

#[tokio::test]
async fn a_rule_file_that_exists_but_cannot_be_read_stops_the_update() {
    // 「在るのに読めない」は blocking である — 規則を落として進むと、届く steering が
    // 静かに痩せる。
    let fixture = Fixture::new();
    fixture.write_rule("team.md", "# Team\n");
    let path = fixture.memory_dir.join("team.md");
    std::fs::write(&path, [0x80_u8, 0x81]).expect("UTF-8 として不正なバイトを置く");

    let mut updater = fixture.updater(journal(), intents());
    let error = updater
        .update_read_models()
        .await
        .expect_err("読めない規則は止める");
    match error {
        ReadModelUpdateError::SteeringRead { path: named, kind } => {
            assert!(named.ends_with("team.md"), "実際: {named}");
            assert_eq!(kind, std::io::ErrorKind::InvalidData);
        }
        other => panic!("読取の失敗として上がる (実際: {other:?})"),
    }
    assert_eq!(fixture.steering_writes(), 0, "1 行も書かない");
}

#[tokio::test]
async fn the_steering_failure_renders_its_material() {
    let error = ReadModelUpdateError::SteeringRead {
        path: "memory/team.md".to_string(),
        kind: std::io::ErrorKind::PermissionDenied,
    };
    assert_eq!(
        error.to_string(),
        "steering read: PermissionDenied at memory/team.md"
    );
    assert!(std::error::Error::source(&error).is_none());
}

#[tokio::test]
async fn the_memory_layer_is_read_before_the_journal_difference_is_probed() {
    // 空のジャーナルでも steering は投影される — 参照入力の比較は早期 return の**前**に
    // 行われる (ジャーナルが動くまで規則が届かない、という穴を塞ぐ)。
    let fixture = Fixture::new();
    let mut updater = fixture.updater(Vec::new(), Vec::new());
    updater.update_read_models().await.expect("空のジャーナル");
    assert_eq!(fixture.steering_writes(), 1);
    assert_eq!(fixture.steering().expect("steering 面").plans().len(), 5);
}

/// 計画の材料を安全に読めなければ、公開要求すら作らず全ファイルを保持する。
#[tokio::test]
async fn an_invalid_audit_target_prevents_any_file_or_plan_publication() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(&fixture.audit_shard).unwrap();
    let state = fixture.state();
    let (mut updater, published_tables) = fixture.spied_updater(journal(), intents());
    assert_eq!(
        updater.update_read_models().await,
        Err(ReadModelUpdateError::PublicationConflict {
            path: fixture.audit_shard.clone()
        })
    );
    assert_eq!(fixture.state(), state);
    assert!(fixture.audit_shard.is_dir());
    assert!(fixture.publications.borrow().is_empty());
    assert!(published_tables.borrow().is_none());
    std::fs::remove_dir(&fixture.audit_shard).unwrap();
    updater.update_read_models().await.unwrap();
    let audit = fixture.shard();
    updater.update_read_models().await.unwrap();
    assert_eq!(fixture.shard(), audit);
}

/// 差分を観測した直後に履歴が消えた場合、古い読取位置で成功したことにしない。
#[tokio::test]
async fn a_disappeared_history_is_not_a_successful_update() {
    for structured in [false, true] {
        let fixture = Fixture::new();
        let state = fixture.state();
        let tables = Rc::new(RefCell::new(None));
        let mut reader = FakeReader {
            journal: journal(),
            intents: intents(),
            checkpoints: BTreeMap::from([(projection(), GlobalSeqNr::new(2))]),
            lose_history_after_probe: true,
            publications: Rc::clone(&fixture.publications),
            tables: Rc::clone(&tables),
            ..FakeReader::default()
        };
        let result = if structured {
            let result = StructuredReadModelUpdater::new(&mut reader, &projection())
                .update_read_models()
                .await;
            assert_eq!(
                reader.checkpoint(&projection()).await.unwrap(),
                GlobalSeqNr::new(2)
            );
            result
        } else {
            let mut updater = OrchestrationReadModelUpdater::new(
                reader,
                projection(),
                fixture.targets(),
                fixture.steering_source(),
            );
            updater.update_read_models().await
        };
        assert_eq!(
            result,
            Err(ReadModelUpdateError::HistoryDisappeared),
            "structured={structured}"
        );
        assert!(fixture.publications.borrow().is_empty());
        assert!(tables.borrow().is_none());
        assert_eq!(fixture.state(), state);
        assert!(!fixture.audit_shard.exists());
    }
}

/// 3 行目に任意のイベントを載せた履歴（genesis → GateOpened → 指定イベント）。
fn journal_with(third: IntentExecutionEvent) -> Vec<JournalEntry> {
    let mut rows = journal();
    rows.pop();
    rows.push(entry(4, 3, third));
    rows
}

fn prompt_observed() -> IntentExecutionEvent {
    IntentExecutionEvent::PromptObserved(core_command_domain::orchestration::PromptObserved::new(
        event_id(),
        execution_id(),
        "session-1",
        "A",
        false,
    ))
}

fn directive_issued() -> IntentExecutionEvent {
    IntentExecutionEvent::DirectiveIssued(core_command_domain::orchestration::DirectiveIssued::new(
        event_id(),
        execution_id(),
        core_command_domain::orchestration::ActiveDirective::new(
            1,
            genesis_intent().id().clone(),
            core_command_domain::orchestration::DirectivePublication::new(
                "1".repeat(64),
                "2".repeat(64),
                core_command_domain::orchestration::PublishedDirective::RunStage {
                    stage: slug("practices-discovery"),
                    unit: None,
                },
            ),
            "2".repeat(64),
            "sessionless:1111111111111111".to_string(),
            0,
            0,
            1,
        ),
    ))
}

fn publication_io(error: ReadModelUpdateError) -> (PathBuf, std::io::ErrorKind) {
    match error {
        ReadModelUpdateError::PublicationIo { path, kind } => (path, kind),
        other => panic!("PublicationIo を期待した: {other:?}"),
    }
}

#[tokio::test]
async fn a_human_turn_file_that_cannot_be_read_stops_the_update_with_its_path() {
    let fixture = Fixture::new();
    let human_turn = fixture.targets().human_turn_file().to_path_buf();
    std::fs::create_dir_all(&human_turn).expect("読めない形で置く");
    let mut updater = fixture.updater(journal_with(prompt_observed()), intents());
    let (path, kind) = publication_io(
        updater
            .update_read_models()
            .await
            .expect_err("読めないので止まる"),
    );
    assert_eq!(path, human_turn);
    assert_ne!(kind, std::io::ErrorKind::NotFound);
    assert!(!fixture.audit_shard.exists(), "監査面へ何も書かない");
}

#[tokio::test]
async fn a_human_turn_is_published_as_the_prompt_time() {
    let fixture = Fixture::new();
    let human_turn = fixture.targets().human_turn_file().to_path_buf();
    std::fs::write(&human_turn, "2020-01-01T00:00:00Z\n").expect("前回の値");
    let mut updater = fixture.updater(journal_with(prompt_observed()), intents());
    updater.update_read_models().await.expect("更新");
    let batch = fixture
        .publications
        .borrow()
        .get(&projection())
        .map(|(batch, _)| batch.clone())
        .expect("公開要求");
    let file = batch
        .files()
        .iter()
        .find(|file| file.path() == human_turn)
        .expect("人間応答の時刻ファイルが公開対象に入る");
    assert_eq!(
        *file,
        core_read_model_updater::orchestration::PublicationFile::replacement(
            &human_turn,
            "2020-01-01T00:00:00Z\n",
            "2026-08-21T09:14:07Z\n"
        ),
        "前回の値からの置換として公開される"
    );
}

#[tokio::test]
async fn an_active_directive_file_that_cannot_be_read_stops_the_update_with_its_path() {
    let fixture = Fixture::new();
    let directive = fixture.targets().active_directive_file().to_path_buf();
    std::fs::create_dir_all(&directive).expect("読めない形で置く");
    let mut updater = fixture.updater(journal_with(directive_issued()), intents());
    let (path, kind) = publication_io(
        updater
            .update_read_models()
            .await
            .expect_err("読めないので止まる"),
    );
    assert_eq!(path, directive);
    assert_ne!(kind, std::io::ErrorKind::NotFound);
}

#[tokio::test]
async fn a_record_directory_that_is_a_file_is_refused_as_a_state_file_read() {
    let fixture = Fixture::new();
    let blocker = fixture._dir.path().join("record");
    std::fs::write(&blocker, "not a directory").expect("ファイルで塞ぐ");
    let mut checkpoints = BTreeMap::new();
    checkpoints.insert(projection(), GlobalSeqNr::new(2));
    let mut updater = OrchestrationReadModelUpdater::new(
        FakeReader {
            plan_fingerprints: BTreeMap::new(),
            code_generation_approvals: BTreeMap::new(),
            testing: None,
            journal: journal(),
            intents: intents(),
            checkpoints,
            tables: Rc::new(RefCell::new(None)),
            late_row: Rc::new(RefCell::new(None)),
            reads: Rc::new(RefCell::new(0)),
            lose_history_after_probe: false,
            steering: Rc::clone(&fixture.steering),
            steering_writes: Rc::clone(&fixture.steering_writes),
            publications: Rc::clone(&fixture.publications),
            artifacts: Vec::new(),
            sessions: Vec::new(),
        },
        projection(),
        ProjectionTargets::new(
            blocker.join("aidlc-state.md"),
            blocker.join("audit/host.md"),
            fixture.memory_dir.clone(),
        ),
        fixture.steering_source(),
    );
    let error = updater
        .update_read_models()
        .await
        .expect_err("存在を確かめられない");
    assert!(
        matches!(error, ReadModelUpdateError::StateFileRead(_)),
        "実際: {error:?}"
    );
}

#[tokio::test]
async fn a_reader_without_a_pipeline_face_reports_unsupported_instead_of_pretending() {
    let mut reader = FakeReader::default();
    let tables = core_read_model_updater::read_tables::PipelineTables::project(
        &JournalBatch::empty(),
        &IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
        None,
    )
    .expect("空の履歴からも表は組める");
    let error = reader
        .replace_pipeline(&tables)
        .await
        .expect_err("pipeline 面を持たない読み手");
    assert!(
        matches!(
            error,
            JournalReadError::Io {
                kind: std::io::ErrorKind::Unsupported,
                path: None
            }
        ),
        "実際: {error:?}"
    );
}

/// 公開名（記録ディレクトリ名と slug）を持つ intent の誕生。
fn named_intent() -> Intent {
    let base = genesis_intent();
    Intent::from((
        Created::new(
            intent_event_id(),
            base.id().clone(),
            WorkflowDefinitionId::parse("claude").expect("定義 id"),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision"),
            StartRequest::new("classic", "build it").with_record_name(
                core_command_domain::orchestration::IntentRecordName::new(
                    core_command_domain::workspace::IntentDirName::parse("260907-selfhost-stage1")
                        .expect("記録名"),
                    "selfhost-stage1",
                ),
            ),
            base.stages().clone(),
            base.scan().clone(),
        ),
        at(),
    ))
}

/// 記録ディレクトリを 1 段掘り、`intents.json` の置き場（記録の親）を一時ディレクトリ直下に置く。
struct RegistryFixture {
    fixture: Fixture,
    record: PathBuf,
}

impl RegistryFixture {
    fn new() -> Self {
        let fixture = Fixture::new();
        let record = fixture._dir.path().join("record");
        std::fs::create_dir_all(&record).expect("記録ディレクトリ");
        std::fs::write(record.join("aidlc-state.md"), STATE).expect("出発点を置く");
        Self { fixture, record }
    }
    fn registry(&self) -> PathBuf {
        self.fixture._dir.path().join("intents.json")
    }
    fn updater(&self) -> OrchestrationReadModelUpdater<FakeReader> {
        let mut checkpoints = BTreeMap::new();
        checkpoints.insert(projection(), GlobalSeqNr::new(2));
        OrchestrationReadModelUpdater::new(
            FakeReader {
                plan_fingerprints: BTreeMap::new(),
                code_generation_approvals: BTreeMap::new(),
                testing: None,
                journal: journal(),
                intents: vec![(1, named_intent())],
                checkpoints,
                tables: Rc::new(RefCell::new(None)),
                late_row: Rc::new(RefCell::new(None)),
                reads: Rc::new(RefCell::new(0)),
                lose_history_after_probe: false,
                steering: Rc::clone(&self.fixture.steering),
                steering_writes: Rc::clone(&self.fixture.steering_writes),
                publications: Rc::clone(&self.fixture.publications),
                artifacts: Vec::new(),
                sessions: Vec::new(),
            },
            projection(),
            ProjectionTargets::new(
                self.record.join("aidlc-state.md"),
                self.record.join("audit/host-abcd1234.md"),
                self.fixture.memory_dir.clone(),
            ),
            self.fixture.steering_source(),
        )
    }
}

#[tokio::test]
async fn a_named_intent_is_registered_and_a_foreign_row_is_kept() {
    let registry = RegistryFixture::new();
    std::fs::write(
        registry.registry(),
        "[{\"uuid\":\"11111111-1111-7111-8111-111111111111\",\"slug\":\"other\",\"dirName\":\"260901-other\",\"scope\":\"express\",\"status\":\"complete\"}]\n",
    )
    .expect("既存の登録");
    registry.updater().update_read_models().await.expect("更新");
    let rows: Vec<serde_json::Value> =
        serde_json::from_str(&std::fs::read_to_string(registry.registry()).expect("登録"))
            .expect("JSON");
    assert_eq!(rows.len(), 2, "イベント外の既存行は保持する");
    let registered = rows.last().expect("追加された行");
    assert_eq!(
        registered.get("uuid"),
        Some(&serde_json::Value::from(INTENT))
    );
    assert_eq!(
        registered.get("dirName"),
        Some(&serde_json::Value::from("260907-selfhost-stage1"))
    );
    assert_eq!(
        registered.get("slug"),
        Some(&serde_json::Value::from("selfhost-stage1"))
    );
    assert_eq!(
        registered.get("status"),
        Some(&serde_json::Value::from("in-flight"))
    );
}

#[tokio::test]
async fn a_registry_row_whose_directory_disagrees_with_the_birth_is_a_conflict() {
    let registry = RegistryFixture::new();
    std::fs::write(
        registry.registry(),
        format!("[{{\"uuid\":\"{INTENT}\",\"slug\":\"selfhost-stage1\",\"dirName\":\"260907-elsewhere\",\"scope\":\"classic\",\"status\":\"in-flight\"}}]\n"),
    )
    .expect("食い違う登録");
    let error = registry
        .updater()
        .update_read_models()
        .await
        .expect_err("読み替えない");
    assert!(
        matches!(&error, ReadModelUpdateError::PublicationConflict { path } if *path == registry.registry()),
        "実際: {error:?}"
    );
}

#[tokio::test]
async fn a_registry_row_that_already_owns_the_directory_under_another_intent_is_a_conflict() {
    let registry = RegistryFixture::new();
    std::fs::write(
        registry.registry(),
        "[{\"uuid\":\"11111111-1111-7111-8111-111111111111\",\"slug\":\"other\",\"dirName\":\"260907-selfhost-stage1\",\"scope\":\"express\",\"status\":\"complete\"}]\n",
    )
    .expect("同じディレクトリを持つ別 intent");
    let error = registry
        .updater()
        .update_read_models()
        .await
        .expect_err("読み替えない");
    assert!(
        matches!(&error, ReadModelUpdateError::PublicationConflict { path } if *path == registry.registry()),
        "実際: {error:?}"
    );
}

#[tokio::test]
async fn a_registry_that_cannot_be_read_stops_the_update_with_its_path() {
    let registry = RegistryFixture::new();
    std::fs::create_dir_all(registry.registry()).expect("読めない形で置く");
    let (path, kind) = publication_io(
        registry
            .updater()
            .update_read_models()
            .await
            .expect_err("読めない"),
    );
    assert_eq!(path, registry.registry());
    assert_ne!(kind, std::io::ErrorKind::NotFound);
    let broken = RegistryFixture::new();
    std::fs::write(broken.registry(), "{not json").expect("壊れた登録");
    let (path, kind) = publication_io(
        broken
            .updater()
            .update_read_models()
            .await
            .expect_err("読めない"),
    );
    assert_eq!(path, broken.registry());
    assert_eq!(kind, std::io::ErrorKind::InvalidData);
}

/// 取得ループの失敗は、材料（投影名・位置・内包した失敗）を診断文言に載せる。
#[test]
fn the_update_failures_render_their_material() {
    use core_read_model_updater::read_tables::ReadTablesError;
    use core_read_model_updater::workspace::StateFileWriteError;
    let cases: Vec<(ReadModelUpdateError, &str)> = vec![
        (
            ReadModelUpdateError::LegacyProjection {
                projection: "state-file".to_string(),
            },
            "legacy shared projection requires migration: state-file",
        ),
        (
            ReadModelUpdateError::StateFileWrite(StateFileWriteError::Io {
                message: "disk full".to_string(),
            }),
            "state file write: Io { message: \"disk full\" }",
        ),
        (
            ReadModelUpdateError::HistoryDisappeared,
            "history disappeared between reads",
        ),
        (
            ReadModelUpdateError::ReadTables(ReadTablesError::MissingGenesis {
                aggregate_id: "abc".to_string(),
            }),
            "read tables: missing genesis for abc",
        ),
        (ReadModelUpdateError::PlanUnavailable, "plan unavailable"),
        (ReadModelUpdateError::MixedIntents, "mixed intents"),
    ];
    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected);
    }
}

// ---------------------------------------------------------------------------
// 誕生 (genesis) からの初回更新 — 状態ファイルの骨格・依頼原文・ソース基準
// ---------------------------------------------------------------------------

/// 記録名とソース基準を持つ intent の誕生記録（初回投影が公開する付随ファイルの材料）。
///
/// 初期化ステージ (`state-init`) を先頭に持つ — 誕生の投影はそれを完了済みにし、最初の
/// ゲート付きステージへ着地する。`gated` を偽にすると着地先が無い計画になる。
fn genesis_intent_with_baseline(gated: bool) -> Intent {
    use core_command_domain::orchestration::{IntentRecordName, SourceBaseline};
    use core_command_domain::workspace::IntentDirName;
    let listing = format!("-\tsrc/lib.rs\t100644\t{}\n", "0".repeat(64));
    let init = StageEntry::new(
        slug("state-init"),
        PhaseId::Initialization,
        PlanAction::Execute,
        false,
        StageDisplay::new(
            StageNumber::parse("0.1").expect("番号"),
            "State Init",
            "orchestrator",
        )
        .expect("単一行"),
    );
    let stage = StageEntry::new(
        slug("practices-discovery"),
        PhaseId::Inception,
        PlanAction::Execute,
        false,
        StageDisplay::new(
            StageNumber::parse("2.2").expect("番号"),
            "Practices Discovery",
            "aidlc-pipeline-deploy-agent",
        )
        .expect("単一行"),
    );
    let stages = if gated { vec![init, stage] } else { vec![init] };
    Intent::from((
        Created::new(
            intent_event_id(),
            IntentId::parse(INTENT).expect("UUIDv7"),
            WorkflowDefinitionId::parse("claude").expect("定義 id"),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision"),
            StartRequest::new("classic", "build \"it\"")
                .with_record_name(IntentRecordName::new(
                    IntentDirName::parse("260907-selfhost-stage1").expect("記録名"),
                    "selfhost-stage1",
                ))
                .with_source_baseline(SourceBaseline::new(Some(listing)).expect("一覧")),
            StageEntries::new(stages).expect("計画"),
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

/// チェックポイント 0 から始める読み手（誕生の行から描く）。
fn updater_from_zero(
    fixture: &Fixture,
    journal: Vec<JournalEntry>,
    intents: Vec<(u64, Intent)>,
) -> OrchestrationReadModelUpdater<FakeReader> {
    OrchestrationReadModelUpdater::new(
        FakeReader {
            journal,
            intents,
            publications: Rc::clone(&fixture.publications),
            steering: Rc::clone(&fixture.steering),
            steering_writes: Rc::clone(&fixture.steering_writes),
            ..FakeReader::default()
        },
        projection(),
        fixture.targets(),
        fixture.steering_source(),
    )
}

#[tokio::test]
async fn the_first_update_from_genesis_composes_the_state_and_publishes_the_birth_files() {
    let fixture = Fixture::new();
    std::fs::remove_file(&fixture.state_file).expect("骨格は投影が組む");
    let intent = genesis_intent_with_baseline(true);
    let started = IntentExecutionEvent::Started(Started::new(
        event_id(),
        execution_id(),
        intent.id().clone(),
        intent.stages().clone(),
    ));
    let journal = vec![
        entry(2, 1, started),
        entry(
            3,
            2,
            IntentExecutionEvent::GateOpened(GateOpened::new(
                event_id(),
                execution_id(),
                slug("practices-discovery"),
                ArtifactPaths::empty(),
            )),
        ),
    ];
    let mut updater = updater_from_zero(&fixture, journal, vec![(1, intent.clone())]);
    updater.update_read_models().await.expect("初回更新");
    let reached = updater
        .checkpoint()
        .await
        .expect("チェックポイントを読める");
    assert_eq!(reached, GlobalSeqNr::new(3));
    let state = fixture.state();
    assert!(
        state.contains("## Stage Progress"),
        "骨格が組まれる: {state}"
    );
    assert!(
        state.contains("- **Current Stage**: practices-discovery"),
        "{state}"
    );
    let targets = fixture.targets();
    assert_eq!(
        std::fs::read_to_string(targets.description_file()).expect("依頼原文"),
        "\"build \\\"it\\\"\"\n",
        "依頼原文は契約 JSON の文字列 1 つ"
    );
    let baseline = intent
        .source_baseline()
        .and_then(|baseline| baseline.snapshot_name())
        .expect("基準名");
    assert!(
        fixture
            .state_file
            .with_file_name(".aidlc-source-review")
            .join("code-generation")
            .join(&baseline)
            .is_file(),
        "ソース基準の一覧が公開される"
    );
    assert!(
        fixture.shard().contains("WORKFLOW_STARTED"),
        "{}",
        fixture.shard()
    );

    // 依頼原文が既に在れば書き直さない（人が触る可能性のある正本）。ゲート付きステージの
    // 無い計画 (初期化だけ) でも誕生は描ける — 着地先が無いだけである。
    let again = Fixture::new();
    std::fs::remove_file(&again.state_file).unwrap();
    std::fs::write(again.targets().description_file(), "\"kept\"\n").unwrap();
    let init_only = genesis_intent_with_baseline(false);
    let mut updater = updater_from_zero(
        &again,
        vec![entry(
            2,
            1,
            IntentExecutionEvent::Started(Started::new(
                event_id(),
                execution_id(),
                init_only.id().clone(),
                init_only.stages().clone(),
            )),
        )],
        vec![(1, init_only.clone())],
    );
    updater.update_read_models().await.expect("初回更新");
    assert_eq!(
        std::fs::read_to_string(again.targets().description_file()).unwrap(),
        "\"kept\"\n"
    );
    assert!(
        again.state().contains("- [x] state-init — EXECUTE"),
        "{}",
        again.state()
    );
}

#[tokio::test]
async fn a_state_or_description_path_that_cannot_be_probed_stops_the_update() {
    // 状態ファイルの親の位置にファイルがある → 在否を確かめられない (ENOTDIR)。
    let fixture = Fixture::new();
    let blocker = fixture._dir.path().join("blocker");
    std::fs::write(&blocker, "file").unwrap();
    let targets = ProjectionTargets::new(
        blocker.join("aidlc-state.md"),
        fixture.audit_shard.clone(),
        fixture.memory_dir.clone(),
    );
    let mut updater = OrchestrationReadModelUpdater::new(
        FakeReader {
            journal: journal(),
            intents: intents(),
            publications: Rc::clone(&fixture.publications),
            steering: Rc::clone(&fixture.steering),
            steering_writes: Rc::clone(&fixture.steering_writes),
            ..FakeReader::default()
        },
        projection(),
        targets,
        fixture.steering_source(),
    );
    let error = updater
        .update_read_models()
        .await
        .expect_err("在否を確かめられない");
    assert!(
        matches!(error, ReadModelUpdateError::StateFileRead(_)),
        "実際: {error:?}"
    );
    assert!(fixture.publications.borrow().is_empty());

    // 状態ファイルは在るが、依頼原文の位置を確かめられない (project-description.json の
    // 位置にディレクトリではなくファイルの下の経路を要求される形)。
    let fixture = Fixture::new();
    std::fs::remove_file(&fixture.state_file).unwrap();
    let description = fixture.targets().description_file().to_path_buf();
    std::fs::write(&description, "x").unwrap();
    let nested = ProjectionTargets::new(
        description.join("aidlc-state.md"),
        fixture.audit_shard.clone(),
        fixture.memory_dir.clone(),
    );
    let mut updater = OrchestrationReadModelUpdater::new(
        FakeReader {
            journal: journal(),
            intents: intents(),
            publications: Rc::clone(&fixture.publications),
            steering: Rc::clone(&fixture.steering),
            steering_writes: Rc::clone(&fixture.steering_writes),
            ..FakeReader::default()
        },
        projection(),
        nested,
        fixture.steering_source(),
    );
    let error = updater
        .update_read_models()
        .await
        .expect_err("在否を確かめられない");
    assert!(
        matches!(error, ReadModelUpdateError::StateFileRead(_)),
        "実際: {error:?}"
    );
}

// ---------------------------------------------------------------------------
// 保存済みの公開要求 (未確定) は、新しいイベントを描く前に片付ける
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_pending_publication_is_finished_before_new_events_are_drawn() {
    let fixture = Fixture::new();
    let pending = PublicationBatch::new(
        GlobalSeqNr::new(2),
        GlobalSeqNr::new(3),
        vec![
            core_read_model_updater::orchestration::PublicationFile::audit(
                &fixture.audit_shard,
                "\n## Saved\n**Event**: SAVED_CUT\n\n---\n",
            )
            .unwrap(),
        ],
    )
    .for_targets(&fixture.targets())
    .unwrap();
    fixture
        .publications
        .borrow_mut()
        .insert(projection(), (pending, false));
    let mut updater = fixture.updater(journal(), intents());
    updater
        .update_read_models()
        .await
        .expect("保存済みの断面を先に確定する");
    let reached = updater
        .checkpoint()
        .await
        .expect("チェックポイントを読める");
    assert_eq!(reached, GlobalSeqNr::new(4));
    let shard = fixture.shard();
    let saved = shard
        .find("SAVED_CUT")
        .expect("保存済みの断面が先に書かれる");
    // 保存済みの断面は位置 3 までを確定したので、新しく描かれるのは位置 4 の改訂だけ。
    let fresh = shard
        .find("Re-entering gate after revision")
        .unwrap_or_else(|| panic!("新しいイベントは後に描かれる: {shard}"));
    assert!(saved < fresh, "{shard}");
    assert_eq!(
        shard.matches("STAGE_AWAITING_APPROVAL").count(),
        1,
        "確定済みの位置 (ゲート開始) は描き直さない: {shard}"
    );
    assert!(
        fixture
            .publications
            .borrow()
            .get(&projection())
            .is_some_and(|(_, committed)| *committed)
    );
}

#[tokio::test]
async fn a_pending_publication_for_other_targets_or_beyond_the_history_is_refused() {
    // 別の出力先へ束ねられた要求は、この出力先では確定しない。
    let fixture = Fixture::new();
    let elsewhere = tempfile::tempdir().unwrap();
    let foreign = ProjectionTargets::new(
        elsewhere.path().join("aidlc-state.md"),
        elsewhere.path().join("audit/host.md"),
        elsewhere.path().join("memory"),
    );
    let pending = PublicationBatch::new(GlobalSeqNr::new(2), GlobalSeqNr::new(3), Vec::new())
        .for_targets(&foreign)
        .unwrap();
    fixture
        .publications
        .borrow_mut()
        .insert(projection(), (pending, false));
    let mut updater = fixture.updater(journal(), intents());
    assert_eq!(
        updater.update_read_models().await,
        Err(ReadModelUpdateError::PublicationConflict {
            path: fixture.state_file.clone()
        })
    );
    assert_eq!(fixture.state(), STATE, "状態ファイルに触らない");

    // 履歴が要求の位置まで無ければ、材料が無いので確定できない。
    let fixture = Fixture::new();
    let pending = PublicationBatch::new(GlobalSeqNr::new(2), GlobalSeqNr::new(9), Vec::new())
        .for_targets(&fixture.targets())
        .unwrap();
    fixture
        .publications
        .borrow_mut()
        .insert(projection(), (pending, false));
    let mut updater = fixture.updater(journal(), intents());
    assert_eq!(
        updater.update_read_models().await,
        Err(ReadModelUpdateError::PlanUnavailable)
    );
    assert!(!fixture.audit_shard.exists());
}

// ---------------------------------------------------------------------------
// 成果物・セッション監査は、記録の監査シャードへ同じ公開要求で積む
// ---------------------------------------------------------------------------

/// 記録配置 (`aidlc/spaces/<space>/intents/<record>/`) の出力先。
struct RecordFixture {
    _dir: TempDir,
    record: PathBuf,
    publications: Rc<RefCell<BTreeMap<ProjectionName, (PublicationBatch, bool)>>>,
}

impl RecordFixture {
    fn new() -> RecordFixture {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let record = dir.path().join("aidlc/spaces/default/intents/260907-x");
        std::fs::create_dir_all(record.join("memory")).unwrap();
        std::fs::write(record.join("aidlc-state.md"), STATE).unwrap();
        RecordFixture {
            _dir: dir,
            record,
            publications: Rc::new(RefCell::new(BTreeMap::new())),
        }
    }
    fn targets(&self) -> ProjectionTargets {
        ProjectionTargets::new(
            self.record.join("aidlc-state.md"),
            self.record.join("audit/host-abcd1234.md"),
            self.record.join("memory"),
        )
    }
    fn shard(&self) -> String {
        std::fs::read_to_string(self.record.join("audit/host-abcd1234.md")).unwrap_or_default()
    }
    fn updater(
        &self,
        checkpoint: u64,
        journal: Vec<JournalEntry>,
        intents: Vec<(u64, Intent)>,
        artifacts: Vec<core_read_model_updater::orchestration::ArtifactJournalEntry>,
        sessions: Vec<core_read_model_updater::orchestration::SessionJournalEntry>,
    ) -> OrchestrationReadModelUpdater<FakeReader> {
        let mut checkpoints = BTreeMap::new();
        if checkpoint > 0 {
            checkpoints.insert(projection(), GlobalSeqNr::new(checkpoint));
        }
        OrchestrationReadModelUpdater::new(
            FakeReader {
                journal,
                intents,
                checkpoints,
                artifacts,
                sessions,
                publications: Rc::clone(&self.publications),
                ..FakeReader::default()
            },
            projection(),
            self.targets(),
            SteeringSource::new(self.record.join("memory")),
        )
    }
}

fn hook_target(record: &str) -> core_command_domain::workspace::HookHealthTarget {
    use core_command_domain::workspace::{HookHealthTarget, IntentDirName, SpaceName};
    HookHealthTarget::new(
        SpaceName::default(),
        Some(IntentDirName::parse(record).expect("記録名")),
    )
}

fn artifact_entry(
    global: u64,
    seq_nr: usize,
    record: &str,
    file: &str,
    created: bool,
) -> core_read_model_updater::orchestration::ArtifactJournalEntry {
    use core_command_domain::workspace::{
        ArtifactAuditEvent, ArtifactAuditEventId, ArtifactAuditId, ArtifactSaved,
        ArtifactWriteObservation,
    };
    let target = hook_target(record);
    let event = ArtifactAuditEvent::Saved(ArtifactSaved::new(
        ArtifactAuditEventId::generate(),
        ArtifactAuditId::for_target(&target),
        ArtifactWriteObservation::new(
            target,
            "Write".into(),
            file.into(),
            "intent-capture".into(),
            created,
        ),
    ));
    core_read_model_updater::orchestration::ArtifactJournalEntry::new(
        GlobalSeqNr::new(global),
        seq_nr,
        at(),
        event,
    )
}

fn session_entry(
    global: u64,
    seq_nr: usize,
    record: &str,
) -> core_read_model_updater::orchestration::SessionJournalEntry {
    use core_command_domain::workspace::{
        AuditFieldKey, AuditFields, EventType, SessionAuditEvent, SessionAuditEventId,
        SessionAuditId, SessionAuditObservationId, SessionAuditRecord,
    };
    let target = hook_target(record);
    let event = SessionAuditEvent::new(
        SessionAuditEventId::generate(),
        SessionAuditObservationId::generate(),
        SessionAuditId::for_target(&target),
        target,
        SessionAuditRecord::new(
            EventType::SessionEnded,
            AuditFields::new().with(AuditFieldKey::parse("Reason").unwrap(), "clear"),
        )
        .unwrap(),
    )
    .unwrap();
    core_read_model_updater::orchestration::SessionJournalEntry::new(
        GlobalSeqNr::new(global),
        seq_nr,
        at(),
        event,
    )
}

/// 成果物監査・セッション監査の行は、実行の行が確定済みの窓でも、記録のシャードへ積まれる。
/// 別の記録の観測は積まない。
///
/// 成果物とセッションを**別の窓**で流すのは、同じシャードへの追記計画が 1 バッチに 2 つ
/// 入ると 2 つ目の適用が競合として拒否されるため（`PublicationFile::audit` は計画時の
/// 内容を前提に固定する）。混在窓の扱いは人間の裁定事項として報告書に記す。
#[tokio::test]
async fn artifact_and_session_rows_of_the_record_are_appended_to_its_audit_shard() {
    let fixture = RecordFixture::new();
    let mut updater = fixture.updater(
        4,
        journal(),
        intents(),
        vec![
            artifact_entry(5, 1, "260907-x", "ideation/intent.md", true),
            // 別の記録の観測はこのシャードへ積まない。
            artifact_entry(6, 1, "260907-other", "elsewhere.md", false),
        ],
        Vec::new(),
    );
    updater.update_read_models().await.expect("成果物の窓");
    let reached = updater
        .checkpoint()
        .await
        .expect("チェックポイントを読める");
    assert_eq!(reached, GlobalSeqNr::new(6));
    let shard = fixture.shard();
    assert!(shard.contains("**Event**: ARTIFACT_CREATED"), "{shard}");
    assert!(shard.contains("**File**: ideation/intent.md"), "{shard}");
    assert!(shard.contains("**Context**: intent-capture"), "{shard}");
    assert!(!shard.contains("elsewhere.md"), "{shard}");
    assert!(
        !shard.contains("GATE_OPENED"),
        "確定済みの実行の行は描き直さない: {shard}"
    );

    let mut updater = fixture.updater(
        6,
        journal(),
        intents(),
        Vec::new(),
        vec![
            session_entry(7, 1, "260907-x"),
            session_entry(8, 1, "260907-other"),
        ],
    );
    updater.update_read_models().await.expect("セッションの窓");
    let reached = updater
        .checkpoint()
        .await
        .expect("チェックポイントを読める");
    assert_eq!(reached, GlobalSeqNr::new(8));
    let shard = fixture.shard();
    assert_eq!(
        shard.matches("**Event**: SESSION_ENDED").count(),
        1,
        "{shard}"
    );
    assert!(shard.contains("**Reason**: clear"), "{shard}");
}

/// 監査の行しか無いバッチは計画なしで投影され、チェックポイントは走査済み位置まで進む。
#[tokio::test]
async fn an_audit_only_batch_is_projected_without_a_plan_and_advances_the_checkpoint() {
    for (artifacts, sessions, event) in [
        (
            vec![artifact_entry(
                1,
                1,
                "260907-x",
                "ideation/intent.md",
                false,
            )],
            Vec::new(),
            "ARTIFACT_UPDATED",
        ),
        (
            Vec::new(),
            vec![session_entry(1, 1, "260907-x")],
            "SESSION_ENDED",
        ),
    ] {
        let fixture = RecordFixture::new();
        let mut updater = fixture.updater(0, Vec::new(), Vec::new(), artifacts, sessions);
        updater.update_read_models().await.expect("監査だけの更新");
        let reached = updater
            .checkpoint()
            .await
            .expect("チェックポイントを読める");
        assert_eq!(reached, GlobalSeqNr::new(1));
        let shard = fixture.shard();
        assert!(shard.contains(&format!("**Event**: {event}")), "{shard}");
        assert_eq!(
            std::fs::read_to_string(fixture.record.join("aidlc-state.md")).unwrap(),
            STATE,
            "状態ファイルに触らない"
        );
    }
}

#[tokio::test]
async fn an_audit_shard_that_cannot_be_read_stops_the_artifact_and_session_publication() {
    for (artifacts, sessions) in [
        (
            vec![artifact_entry(5, 1, "260907-x", "ideation/intent.md", true)],
            Vec::new(),
        ),
        (Vec::new(), vec![session_entry(5, 1, "260907-x")]),
    ] {
        let fixture = RecordFixture::new();
        std::fs::create_dir_all(fixture.record.join("audit/host-abcd1234.md")).unwrap();
        let mut updater = fixture.updater(0, Vec::new(), Vec::new(), artifacts, sessions);
        let error = updater
            .update_read_models()
            .await
            .expect_err("シャードが読めない");
        assert!(
            matches!(error, ReadModelUpdateError::PublicationConflict { .. }),
            "実際: {error:?}"
        );
        assert!(fixture.publications.borrow().is_empty());
    }
}

#[tokio::test]
async fn a_first_human_turn_is_published_as_a_creation() {
    let fixture = Fixture::new();
    let human_turn = fixture.targets().human_turn_file().to_path_buf();
    let mut updater = fixture.updater(journal_with(prompt_observed()), intents());
    updater.update_read_models().await.expect("更新");
    assert_eq!(
        std::fs::read_to_string(&human_turn).expect("作られる"),
        "2026-08-21T09:14:07Z\n"
    );
}

#[tokio::test]
async fn an_active_directive_is_created_and_then_replaced_in_place() {
    let fixture = Fixture::new();
    let directive = fixture.targets().active_directive_file().to_path_buf();
    let mut updater = fixture.updater(journal_with(directive_issued()), intents());
    updater.update_read_models().await.expect("初回は作成");
    let created = std::fs::read_to_string(&directive).expect("指示ファイルが作られる");
    assert!(
        created.contains("practices-discovery"),
        "指示の位置が載る: {created}"
    );
    // 同じ指示をもう一度 (別の窓で) 描くと、既存の内容からの置換として公開される。
    let again = Fixture::new();
    let directive = again.targets().active_directive_file().to_path_buf();
    std::fs::write(&directive, "stale\n").unwrap();
    let mut updater = again.updater(journal_with(directive_issued()), intents());
    updater.update_read_models().await.expect("置換");
    let replaced = std::fs::read_to_string(&directive).unwrap();
    assert_ne!(replaced, "stale\n");
    assert_eq!(replaced, created, "同じ指示は同じ内容へ落ち着く");
}

/// 1 ステージ計画 (`genesis_intent`) の投影に足りる骨格。
const FULL_STATE: &str = "\
## Project Information
- **Active Agent**: orchestrator

## Scope Configuration
- **Stages to Execute**: 2.2
- **Stages to Skip**: None

## Execution Plan Summary
- **Total Stages**: 1
- **Completed**: 0
- **In Progress**: practices-discovery

## Runtime State
- **Revision Count**: 0
- **Construction Autonomy Mode**: gated

## Stage Progress
- [-] practices-discovery — EXECUTE

## Phase Progress
- **Initialization**: Verified
- **Ideation**: Pending
- **Inception**: Active
- **Construction**: Pending
- **Operation**: Pending

## Current Status
- **Lifecycle Phase**: INCEPTION
- **Current Stage**: practices-discovery
- **Next Stage**: -
- **Status**: Running
- **Last Updated**: 2026-08-20T00:00:00Z

## Session Resume Point
- **Last Completed Stage**: 
- **Next Action**: Execute Stage
";

#[tokio::test]
async fn a_report_carrying_a_source_baseline_publishes_its_listing() {
    use core_command_domain::orchestration::{
        ReportId, ReportResult, ReportTransition, Reported, SourceBaseline, TransitionStep,
        TransitionSteps,
    };
    let listing = format!("-\tsrc/lib.rs\t100644\t{}\n", "1".repeat(64));
    let baseline = SourceBaseline::new(Some(listing.clone())).unwrap();
    let name = baseline.snapshot_name().expect("一覧があれば名前がある");
    let reported = IntentExecutionEvent::Reported(
        Reported::new(
            event_id(),
            execution_id(),
            ReportId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0555").unwrap(),
            ReportResult::Committed {
                stage: slug("practices-discovery"),
                scope: "classic".to_string(),
                steps: TransitionSteps::new(vec![TransitionStep::Skip]).unwrap(),
                transition: ReportTransition::StageSkipped {
                    reason: "out of scope".to_string(),
                },
            },
            None,
            Some(baseline),
        )
        .unwrap(),
    );
    let fixture = Fixture::new();
    std::fs::write(&fixture.state_file, FULL_STATE).unwrap();
    let mut updater = fixture.updater(journal_with(reported), intents());
    updater.update_read_models().await.expect("更新");
    let published = fixture
        .state_file
        .with_file_name(".aidlc-source-review")
        .join("code-generation")
        .join(&name);
    assert_eq!(
        std::fs::read_to_string(&published).expect("一覧が公開される"),
        listing
    );
    assert!(
        fixture.shard().contains("**Event**: STAGE_SKIPPED"),
        "{}",
        fixture.shard()
    );
}
