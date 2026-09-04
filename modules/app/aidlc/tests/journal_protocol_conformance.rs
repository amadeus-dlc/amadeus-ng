//! ITF 準拠テスト (ADR 0003 決定 5 / FD BR3.5) — `formal/orchestration/journal_protocol.qnt` の
//! トレースを `IntentExecutionRepositoryImpl` + `JournalReaderImpl` + 実 RMU 投影に再生し、
//! 全ステップでモデルの状態射影を突き合わせる。
//!
//! # なぜ合成ルート (`modules/app/aidlc`) に置くのか
//!
//! このテストは**コマンド側（Repository）と RMU（JournalReader / 投影）の両方**を駆動する。
//! コマンド側のクレートは `Cargo.toml` に RMU を書けない（違反）ので、置けるのは RMU 自身か
//! 合成ルートに限られる (`coding-rules/cqrs-boundaries.md`)。合成ルートを選ぶのは、両者が
//! **実際に結線される場所**で駆動するほうが ITF 準拠の観測として忠実だからである。
//!
//! フィクスチャは `tests/conformance/fixtures/journal_protocol/` にコミット済み
//! (`#meta` 正規化済み)。各遷移は `lastAction` × `lastActor` で駆動する (lastAction 規約)。
//!
//! 集約の永続化そのものは本家 event-store-adapter-rs が担い、横断読取とチェックポイントは
//! 我々の `JournalReaderImpl` が持つ (ADR-010)。**モデルは 1 文字も変えていない** — 本家に
//! 載せ替えても同じトレースがそのまま再生できることが、この乗り換えの意味論的な検収である。
//!
//! モデルの抽象は「集約 1・writer 2・投影 1」である。writer 2 つは同じ `IntentExecutionId` を別々に
//! 再水和した 2 本の「ロード済み集約」で表し、衝突は楽観 version の不一致だけで起きる
//! (ロックは ADR-007 で退役した — BR3.2)。投影は**実 RMU** (`ReadModelUpdater::catch_up`) で
//! ある — フェイクではない (固定裁定 7)。モデルが持つ `readModelSeq` は、実 RMU が
//! 「リードモデルを書き終えてから進めた」チェックポイントへ射影される。
//!
//! 射影規則 (モデル変数 → 実装の観測):
//!   journalLen    = `JournalReader::events_after(ZERO)` の行数 (本家 `journal` の rowid 順)
//!   snapVersion   = 本家 `get_latest_snapshot_by_id` が返す封筒の `version()` (行が無ければ 0)
//!   snapSeq       = 同じ封筒の `seq_nr()` (行が無ければ 0)
//!   checkpoint    = `JournalReader::checkpoint(ProjectionName)` の値 (我々の表)
//!   readModelSeq  = 実 RMU の `catch_up` が描き終えて返した最後の global 通番
//!   loadedVersion = 各 writer が握っている再水和結果の `version()` (未永続の genesis は 0)
//!
//! v3 で楽観 version は集約から外れ、`SnapshotEnvelope` (列) が正本になった (ADR-010 / B7)。
//! モデルの `loadedVersion` はそのまま**再水和結果が握る版**へ射影される — 読んだ版で書くこと
//! そのものが `store_ok` / `store_conflict` の分岐条件だからである。

// テストコードでは unwrap / expect / panic を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
// indexing_slicing も同じ理由 (固定長フィクスチャの添字参照) で file 単位の allow が要る。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    AutonomyMode, AutonomyModeSet, Created, GateApproved, GateOpened, GateRejected, Intent,
    IntentEvent, IntentEventId, IntentExecution, IntentExecutionEvent, IntentExecutionEventId,
    IntentExecutionId, IntentId, Jumped, Parked, Recomposed, SingleStageRunCommitted,
    SkeletonStance, SkeletonStanceRecorded, StageDisplay, StageEntry, StageRevised, StageSkipped,
    StartRequest, Unparked, WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, CompiledDefinition, CompiledDefinitionId, DefinitionRevision,
    ExecutionKind, PhaseId, PlanAction, ScopeGrid, StageGraph, StageMode, StageNodeBuilder,
    StageNumber, StageSlug, WorkflowDefinition, WorkflowDefinitionEvent, WorkflowDefinitionId,
};
use core_command_domain::workspace::{CheckboxState, SpaceName, StorePath};
use core_command_interface_adapter::orchestration::{
    IntentExecutionAggregateKeyDto, IntentExecutionRepositoryImpl, IntentExecutionSqliteStore,
    IntentRepositoryImpl, SnapshotStrategy, WorkflowDefinitionRepositoryImpl,
};
// 両側の永続化 DTO は**同名の別の型**である (側ごと専用化 —
// `coding-rules/cqrs-boundaries.md`)。同じファイルで名指すために別名で引く。
use core_command_interface_adapter::orchestration::IntentExecutionEventDto as CommandExecutionEventDto;
use core_command_interface_adapter::orchestration::WorkflowDefinitionEventDto as CommandDefinitionEventDto;
use core_command_use_case::orchestration::{
    IntentExecutionRepository, IntentRepository, RepositoryError, WorkflowDefinitionRepository,
};
use core_read_model_updater::orchestration::IntentExecutionEventDto as ProjectionExecutionEventDto;
use core_read_model_updater::orchestration::WorkflowDefinitionEventDto as ProjectionDefinitionEventDto;
use core_read_model_updater::orchestration::{
    GlobalSeqNr, JournalReader, JournalReaderImpl, ProjectionName, ProjectionTargets,
    ReadModelUpdater, SteeringSource,
};
use event_store_adapter_rs::types::EventStore;
use serde_json::Value;
use tempfile::TempDir;

/// b40 のテスト用固定イベント識別子 (同じ材料から組んだイベントを同値に保つため)。
#[must_use]
fn intent_event_id() -> IntentEventId {
    IntentEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0001").expect("UUIDv7")
}

/// ITF 再生は時計を持たない — `occurred_at` は固定値でよい (集約は値を素通しする)。
const AT_TEXT: &str = "2026-08-23T00:00:00Z";

/// 固定の発生時刻。
fn at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(AT_TEXT)
        .expect("固定の ISO 8601 UTC")
        .with_timezone(&Utc)
}

/// 再生に使う集約識別子 (UUIDv7)。
const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

/// 再生の実行識別子 — intent と**別の値**であることは前提である (本家 journal の
/// `(aid, seq_nr)` UNIQUE 索引は type_name 抜きの生値で張られる — issue #50)。
const EXECUTION: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";

/// モデルの `WRITERS`。
const WRITERS: usize = 2;

/// 合成計画のステージ数。
///
/// フィクスチャは `--max-steps 40` で採取しており、1 ステップが書けるイベントは高々 1 件
/// なのでジャーナルは 40 行を超えない。24 ステージの合成計画は genesis 1 + ゲート付き
/// 23 ステージ × 2 イベント = 47 件を受け付けるので、再生の途中で「もう打てるコマンドが
/// 無い (= ワークフロー完了)」状態には入らない。
///
/// 索引 0 (initialization) の完了 1 件は、誕生 = 初期化完了済み (issue #76) により
/// 誕生の一部になったので数から外れた — 余力は 48 から 47 へ減るが、40 は下回らない。
const STAGES: usize = 24;

/// 本家の SQLite イベントストア (射影の観測に使う読取ハンドル)。
type UpstreamStore = IntentExecutionSqliteStore;

/// Repository の具体型 (SQLite バックエンド)。
type Repository = IntentExecutionRepositoryImpl<UpstreamStore>;

// ---- ITF の読み取り ----

fn bigint(v: &Value) -> u64 {
    v["#bigint"]
        .as_str()
        .expect("ITF の整数は #bigint")
        .parse()
        .expect("非負の整数")
}

/// `int -> int` の #map を writer 順の Vec へ (値は集約の楽観 version = `usize`)。
fn map_to_vec(v: &Value) -> Vec<usize> {
    let pairs = v["#map"].as_array().expect("ITF の写像は #map");
    let mut out: Vec<Option<usize>> = (0..WRITERS).map(|_| None).collect();
    for pair in pairs {
        let key = usize::try_from(bigint(&pair[0])).expect("writer 添字");
        if let Some(slot) = out.get_mut(key) {
            *slot = Some(usize::try_from(bigint(&pair[1])).expect("楽観 version"));
        }
    }
    out.into_iter()
        .map(|slot| slot.expect("全 writer が写像に載っている"))
        .collect()
}

/// モデルの 1 状態 (準拠テストが使う変数だけを読む)。
struct ModelState {
    last_action: String,
    last_actor: usize,
    journal_len: u64,
    snap_version: usize,
    snap_seq: usize,
    checkpoint: u64,
    read_model_seq: u64,
    loaded_version: Vec<usize>,
}

impl ModelState {
    fn loaded_version_of(&self, writer: usize) -> usize {
        *self
            .loaded_version
            .get(writer)
            .expect("writer 添字はモデルの範囲内")
    }
}

fn parse_state(v: &Value) -> ModelState {
    ModelState {
        last_action: v["lastAction"]
            .as_str()
            .expect("lastAction は文字列")
            .to_string(),
        last_actor: usize::try_from(bigint(&v["lastActor"])).expect("writer 添字"),
        journal_len: bigint(&v["journalLen"]),
        snap_version: usize::try_from(bigint(&v["snapVersion"])).expect("snapVersion"),
        snap_seq: usize::try_from(bigint(&v["snapSeq"])).expect("snapSeq"),
        checkpoint: bigint(&v["checkpoint"]),
        read_model_seq: bigint(&v["readModelSeq"]),
        loaded_version: map_to_vec(&v["loadedVersion"]),
    }
}

// ---- 再生先の組み立て ----

fn execution_id() -> IntentExecutionId {
    IntentExecutionId::parse(EXECUTION).expect("再生の IntentExecutionId は UUIDv7")
}

fn projection_name() -> ProjectionName {
    ProjectionName::parse("state-file").expect("投影名は kebab")
}

/// 1 回の再生が使う 1 つのストアファイル。
struct Store {
    _dir: TempDir,
    path: StorePath,
}

impl Store {
    fn new() -> Store {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let path = StorePath::for_space(&dir.path().join("aidlc"), &SpaceName::default());
        std::fs::create_dir_all(path.as_path().parent().expect("親 dir を持つ"))
            .expect("intents/ を先に作る");
        Store { _dir: dir, path }
    }

    /// 「プロセスを起動する」— 同じファイルへ新しい接続を開く。
    fn repository(&self) -> Repository {
        // Quint モデル (journal_protocol.qnt) は「毎書込でスナップショット更新」を前提に
        // snapSeq を追う。ITF 再生はモデルと同じ構成 — 毎イベントでスナップショットを書く
        // ストラテジ — で流す (間引き構成の挙動は intent_execution_repository_impl_test 側の
        // 結合テストが固定する — issue #44)。
        IntentExecutionRepositoryImpl::open(&self.path)
            .expect("ストアは開ける")
            .with_snapshot_strategy(SnapshotStrategy::every(
                std::num::NonZeroUsize::new(1).expect("1 は非零"),
            ))
    }

    /// 投影が使う横断読取 (同じファイルへの別接続)。
    fn journal_reader(&self) -> JournalReaderImpl {
        JournalReaderImpl::open(&self.path).expect("Reader は開ける")
    }

    /// スナップショット列を直接観測するための読取ハンドル (射影の突合せ用)。
    fn snapshot_view(&self) -> UpstreamStore {
        UpstreamStore::new(self.path.as_path()).expect("本家ストアは開ける")
    }
}

/// 索引 0 = initialization (非ゲート)、以降 = inception (ゲート付き) の合成計画。
fn stages() -> Vec<StageEntry> {
    (0..STAGES)
        .map(|index| {
            let phase = if index == 0 {
                PhaseId::Initialization
            } else {
                PhaseId::Inception
            };
            StageEntry::new(
                StageSlug::parse(&format!("stage-{index}")).expect("合成 slug は文法内"),
                phase,
                PlanAction::Execute,
                false,
                StageDisplay::new(
                    StageNumber::parse(&format!("{}.{}", phase.index(), index + 1))
                        .expect("合成のステージ番号は文法内"),
                    "Stage",
                    "orchestrator",
                )
                .expect("単一行"),
            )
        })
        .collect()
}

/// genesis の集約と `Started` イベント (`seq_nr` = 1。版はまだストアに無い)。
fn intent() -> Intent {
    Intent::from((
        Created::new(
            intent_event_id(),
            IntentId::parse(INTENT).expect("再生の IntentId は UUIDv7"),
            WorkflowDefinitionId::parse("claude").expect("定義 id"),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64)))
                .expect("定義 revision"),
            StartRequest::new("classic", "conformance")
                .with_depth("standard")
                .with_test_strategy("standard")
                .with_review("adversarial"),
            stages(),
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

fn genesis() -> (IntentExecution, IntentExecutionEvent) {
    IntentExecution::start(execution_id(), &intent(), at())
}

/// カーソル位置から打てる唯一のコマンドを打ち、1 イベントを得る (1 ステップ 1 イベント)。
///
/// `open_gate` → `approve_gate` の順。どちらを打つかは集約の状態 (カーソルの checkbox) だけで
/// 決まるので、モデル側に「どのコマンドか」の情報は要らない (モデルは書込の成否だけを持つ
/// 抽象である)。誕生 = 初期化完了済み (issue #76) 以降カーソルは常にゲート付きに立つので、
/// 非ゲート完了のコマンドは b42 で撤去した (#85 = A)。
fn next_command(aggregate: &mut IntentExecution) -> IntentExecutionEvent {
    let cursor = aggregate.cursor();
    let checkbox = aggregate.checkbox(cursor).expect("カーソルは範囲内");
    let result = if checkbox == CheckboxState::InProgress {
        aggregate.open_gate(&intent(), vec!["artifact.md".to_string()], at())
    } else {
        aggregate.approve_gate(&intent(), None, at())
    };
    result.expect("合成計画はフィクスチャ長ぶんのコマンドを受け付ける (STAGES の見積り)")
}

/// 1 人の writer が握っている「ロード済み集約 + 版」。
struct Writer {
    /// 握っている集約。モデルの `loadedVersion` はこの集約が運んでいる版そのものである。
    aggregate: IntentExecution,
    /// まだ書けていない genesis の `Started` (書込済みなら `None`)。
    pending: Option<IntentExecutionEvent>,
}

impl Writer {
    /// まだ何も書かれていないストアを読んだ writer (genesis を握る — 版は未永続の 0)。
    fn genesis() -> Writer {
        let (aggregate, event) = genesis();
        Writer {
            aggregate,
            pending: Some(event),
        }
    }

    const fn loaded(rehydrated: IntentExecution) -> Writer {
        Writer {
            aggregate: rehydrated,
            pending: None,
        }
    }

    /// モデルの `loadedVersion` — この writer が書込に提示する版。
    ///
    /// 未永続の genesis を握っている writer は 0 である (ストアには行がまだ無く、新規作成の
    /// 規約が `expected_version == 0` だから)。モデルの初期値と同じ意味になる。
    const fn loaded_version(&self) -> usize {
        self.aggregate.version()
    }

    /// 次の書込に使う (イベント, 集約) の下書き。
    ///
    /// コマンドは**複製**に対して打つ。書込が `Err` になっても writer が握っている集約は
    /// 1 ビットも動かない — モデルの `store_conflict` が `loadedVersion` を変えないことと
    /// 同じ意味論であり、衝突のたびに再水和し直さずに次の試行ができる。
    fn draft(&self) -> (IntentExecutionEvent, IntentExecution) {
        if let Some(event) = self.pending.clone() {
            return (event, self.aggregate.clone());
        }
        let mut aggregate = self.aggregate.clone();
        let event = next_command(&mut aggregate);
        (event, aggregate)
    }

    /// 書込が通ったので、**ストアが採番した版**を握り直す (BR5.3 — 版を知るのはストアだけ)。
    fn commit(&mut self, stored: IntentExecution) {
        self.aggregate = stored;
        self.pending = None;
    }
}

/// 投影 (U4) — **実 RMU** を駆動する側。モデルの `readModelSeq` は `catch_up` の到達点。
///
/// リードモデルの書込先はこのテストが用意した一時ディレクトリで、状態ファイルには合成計画の
/// 24 ステージぶんのチェックボックス行と、投影が書き換えるフィールド行が入っている。
/// バイトの逐語性を見るのは `projection_golden_test.rs` の仕事であり、ここが見るのは
/// **ループの契約** (真実源がジャーナルであること・キャッチアップが冪等であること) である。
struct RealProjection {
    _dir: TempDir,
    targets: ProjectionTargets,
    read_model_seq: u64,
    /// キャッチアップが一度でも走ったか (モデルとの通番写像の分岐点 — 下の
    /// `INTENT_ROW_OFFSET` を参照)。
    caught_up: bool,
    /// 参照入力の読取先 (置かないので空計画になる)。
    memory_dir: std::path::PathBuf,
}

impl RealProjection {
    fn new() -> RealProjection {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let state_file = dir.path().join("aidlc-state.md");
        std::fs::write(&state_file, synthetic_state_file()).expect("状態ファイルを置く");
        let audit_shard = dir.path().join("audit/host-abcd1234.md");
        // memory 層は置かない — 規則未整備は正常であり、steering の面は空計画になる
        // (このモデルが抽象するのは実行のストリームだけである)。
        let memory_dir = dir.path().join("memory");
        RealProjection {
            targets: ProjectionTargets::new(state_file, audit_shard),
            memory_dir,
            _dir: dir,
            read_model_seq: 0,
            caught_up: false,
        }
    }

    /// チェックポイント以降を読んで描き、位置を進める (実 RMU の取得ループ)。
    async fn catch_up(&mut self, store: &Store) {
        let mut updater = ReadModelUpdater::new(
            store.journal_reader(),
            projection_name(),
            self.targets.clone(),
            SteeringSource::new(self.memory_dir.clone()),
        );
        let reached = updater.catch_up().await.expect("キャッチアップは通る");
        self.read_model_seq = reached.to_u64();
        self.caught_up = true;
    }
}

/// intent の誕生記録 1 行 (rowid 1) が実ジャーナルに先行するぶんの通番補正 (issue #56)。
///
/// モデル (`journal_protocol.qnt`) は**実行のストリームだけ**を抽象する。実ストアでは同じ
/// journal 表に intent の `Created` が rowid 1 で同居するため、実行の行の global 通番は
/// モデル値 + 1、チェックポイントと readModelSeq は**キャッチアップが一度でも走った後は**
/// モデル値 + 1 になる (走査は intent 行もまたいで前進する)。走る前は 0 のままで一致する。
const INTENT_ROW_OFFSET: u64 = 1;

/// チェックポイント / readModelSeq のモデル値を実ストアの通番へ写す。
const fn shifted(model_value: u64, caught_up: bool) -> u64 {
    if caught_up {
        model_value + INTENT_ROW_OFFSET
    } else {
        model_value
    }
}

/// 合成計画 24 ステージぶんの状態ファイル (投影が書き換える行だけを持つ最小の骨格)。
fn synthetic_state_file() -> String {
    let mut out = String::from(
        "## Project Information\n\
         - **Active Agent**: orchestrator\n\
         \n\
         ## Execution Plan Summary\n\
         - **Total Stages**: 24\n\
         - **Completed**: 0\n\
         - **In Progress**: stage-0\n\
         \n\
         ## Runtime State\n\
         - **Revision Count**: 0\n\
         \n\
         ## Stage Progress\n",
    );
    for index in 0..STAGES {
        let marker = if index == 0 { '-' } else { ' ' };
        out.push_str(&format!("- [{marker}] stage-{index} — EXECUTE\n"));
    }
    out.push_str(
        "\n\
         ## Current Status\n\
         - **Lifecycle Phase**: INITIALIZATION\n\
         - **Current Stage**: stage-0\n\
         - **Next Stage**: stage-1\n\
         \n\
         ## Session Resume Point\n\
         - **Last Completed Stage**: \n\
         - **Next Action**: Execute Stage\n",
    );
    out
}

// ---- 射影の突合 ----

async fn assert_projection(
    store: &Store,
    projection: &RealProjection,
    writers: &[Writer],
    m: &ModelState,
    label: &str,
) {
    let journal_reader = store.journal_reader();
    let batch = journal_reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("ジャーナルは読める");
    let rows = batch.executions();
    assert_eq!(
        u64::try_from(rows.len()).expect("行数"),
        m.journal_len,
        "{label}: journalLen"
    );
    // 単一集約なので seq_nr は 1 からの連番、global 通番は intent 行 1 本ぶんずれた連番に
    // なる (失敗した書込は採番しない)。
    for (offset, entry) in rows.iter().enumerate() {
        let expected = offset + 1;
        assert_eq!(
            entry.global_seq().to_u64(),
            u64::try_from(expected).expect("行番号") + INTENT_ROW_OFFSET,
            "{label}: global 通番"
        );
        assert_eq!(entry.seq_nr(), expected, "{label}: seq_nr");
    }

    let snapshot = store
        .snapshot_view()
        .get_latest_snapshot_by_id(&IntentExecutionAggregateKeyDto::of(&execution_id()))
        .await
        .expect("スナップショットは読める");
    let (version, seq_nr) =
        snapshot.map_or((0, 0), |envelope| (envelope.version(), envelope.seq_nr()));
    assert_eq!(version, m.snap_version, "{label}: snapVersion");
    assert_eq!(seq_nr, m.snap_seq, "{label}: snapSeq");

    let checkpoint = journal_reader
        .checkpoint(&projection_name())
        .await
        .expect("チェックポイントは読める");
    assert_eq!(
        checkpoint.to_u64(),
        shifted(m.checkpoint, projection.caught_up),
        "{label}: checkpoint"
    );
    assert_eq!(
        projection.read_model_seq,
        shifted(m.read_model_seq, projection.caught_up),
        "{label}: readModelSeq"
    );

    for (index, writer) in writers.iter().enumerate() {
        assert_eq!(
            writer.loaded_version(),
            m.loaded_version_of(index),
            "{label}: loadedVersion[{index}]"
        );
    }
}

// ---- 再生 ----

async fn replay(path: &Path, seen: &mut BTreeSet<String>) {
    let text = std::fs::read_to_string(path).expect("フィクスチャを読む");
    let trace: Value = serde_json::from_str(&text).expect("フィクスチャは JSON");
    let states: Vec<ModelState> = trace["states"]
        .as_array()
        .expect("ITF は states を持つ")
        .iter()
        .map(parse_state)
        .collect();
    let file = path.file_name().unwrap_or_default().to_string_lossy();

    let first = states.first().expect("トレースは 1 状態以上");
    assert_eq!(first.last_action, "init");

    // ストアは 1 つのファイル。Repository と Reader はそれぞれ自前の接続で開く
    // (本家は接続を露出しないので、横断読取は別接続で行う — ADR-010 決定 4)。
    // crash (プロセス再起動) のたびに開き直す。
    let store = Store::new();
    // intent の誕生記録を先に書く — 実運用の順序 (intent-create → 実行開始) と同じで、
    // 実 RMU の計画供給元は intent 自身のジャーナルである (issue #56)。
    {
        let mut intent_repository =
            IntentRepositoryImpl::open(&store.path).expect("intent ストアは開ける");
        let held = intent();
        let created = IntentEvent::Created(Created::new(
            intent_event_id(),
            held.id().clone(),
            held.definition_id().clone(),
            held.definition_revision().clone(),
            request_of(&held),
            held.stages().to_vec(),
            held.scan().clone(),
        ));
        intent_repository
            .store(&created, &held)
            .await
            .expect("intent の genesis は書ける");
    }
    let mut repository = store.repository();
    let mut projection = RealProjection::new();
    let mut writers: Vec<Writer> = (0..WRITERS).map(|_| Writer::genesis()).collect();

    assert_projection(
        &store,
        &projection,
        &writers,
        first,
        &format!("{file} step 0 (init)"),
    )
    .await;

    for (step, m) in states.iter().enumerate().skip(1) {
        seen.insert(m.last_action.clone());
        let prev = states.get(step - 1).expect("前状態はある");
        let label = format!("{file} step {step} ({})", m.last_action);
        let writer = m.last_actor;

        match m.last_action.as_str() {
            // 再水和 — writer は現在のスナップショット版を握り直す。
            "load" => match repository.find_by_id(&execution_id()).await {
                Ok(rehydrated) => {
                    assert_eq!(
                        rehydrated.version(),
                        prev.snap_version,
                        "{label}: 再水和の版"
                    );
                    *writers.get_mut(writer).expect("writer 添字") = Writer::loaded(rehydrated);
                }
                Err(RepositoryError::NotFound { .. }) => {
                    assert_eq!(
                        prev.journal_len, 0,
                        "{label}: NotFound はジャーナルが空のとき"
                    );
                    *writers.get_mut(writer).expect("writer 添字") = Writer::genesis();
                }
                Err(error) => panic!("{label}: 予期しない再水和エラー {error:?}"),
            },

            // 最新版を握っている writer の書込は通る (Tx 内で journal + snapshot)。
            "store_ok" => {
                let held = writers.get(writer).expect("writer 添字");
                let (event, aggregate) = held.draft();
                repository
                    .store(&event, &aggregate)
                    .await
                    .unwrap_or_else(|error| panic!("{label}: 書込は通るはず {error:?}"));
                assert_eq!(
                    u64::try_from(aggregate.seq_nr()).expect("seq_nr"),
                    m.journal_len,
                    "{label}: 追記された行の seq_nr"
                );
                // 新しい版を採番したのはストアなので、握り直して初めて分かる (BR5.3)。
                let stored = repository
                    .find_by_id(&execution_id())
                    .await
                    .unwrap_or_else(|error| panic!("{label}: 書いた集約は読み直せる {error:?}"));
                writers.get_mut(writer).expect("writer 添字").commit(stored);
            }

            // stale な writer の書込は拒否され、ストアの状態は 1 ビットも変わらない。
            "store_conflict" => {
                let held = writers.get(writer).expect("writer 添字");
                let (event, aggregate) = held.draft();
                let error = repository
                    .store(&event, &aggregate)
                    .await
                    .expect_err("stale な writer の書込は拒否される");
                assert!(
                    matches!(
                        error,
                        RepositoryError::Conflict { expected, actual }
                            if expected == prev.loaded_version_of(writer)
                                && actual == prev.snap_version
                    ),
                    "{label}: 衝突の材料"
                );
                // writer は触らない — 下書きは複製に対して打ったので握っている版は動かない。
            }

            // 投影のキャッチアップ — **実 RMU** がチェックポイント以降を読んで描き、
            // リードモデルを書いてから位置を進める (固定裁定 7)。読むものが無ければ
            // 何も書かず現在値を返す = 冪等 (projection_idempotent)。
            "catchup" => projection.catch_up(&store).await,

            // Tx 済み・投影未反映のままプロセスが落ちる。開き直しても永続状態は同じ。
            "crash" => {
                // 同じファイルへ新しい接続を開き直す (= 書き終えた行は落ちない)。
                repository = store.repository();
                if m.journal_len > 0 {
                    let rebuilt = repository
                        .find_by_id(&execution_id())
                        .await
                        .expect("落ちても書き終えた集約は読み直せる");
                    assert_eq!(rebuilt.version(), m.snap_version, "{label}: 再構成の版");
                }
            }

            "idle" => {}

            action => panic!("{label}: 未知のアクション {action}"),
        }

        assert_projection(&store, &projection, &writers, m, &label).await;
    }
}

#[tokio::test]
async fn the_store_conforms_to_every_committed_journal_protocol_trace() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/conformance/fixtures/journal_protocol");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("フィクスチャのディレクトリがある")
        .map(|entry| entry.expect("ディレクトリ項目").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();

    let mut seen = BTreeSet::new();
    for path in &paths {
        replay(path, &mut seen).await;
    }

    assert!(
        paths.len() >= 6,
        "コミット済みフィクスチャが足りない: {}",
        paths.len()
    );
    // アクション網羅: 全アクションが少なくとも 1 つのコミット済みトレースに現れること
    // (稀アクションを含むフィクスチャの消失退行をここで防ぐ — engine_loop 準拠テストと同型)。
    for action in [
        "load",
        "store_ok",
        "store_conflict",
        "catchup",
        "crash",
        "idle",
    ] {
        assert!(
            seen.contains(action),
            "action {action} を通るコミット済みトレースが無い"
        );
    }
}

// ---- 定義ストリームの横断適合 (b39) ----
//
// 実行・intent と同じストアファイルに同居する**第 3 のストリーム**である。書く側
// (`WorkflowDefinitionRepositoryImpl`) が本当に書いた行を、読む側 (`JournalReaderImpl`) が
// 本当に読めるかをここで固定する — 両側の DTO は同名の別の型であり
// (`coding-rules/cqrs-boundaries.md` の側ごと専用化)、型を共有して静的に揃えるのではなく
// 「書いた行が読める」ことで揃っていると示す。

/// 定義ストリームの試験に使う系譜 ID (ハーネス名 — UUID 空間と衝突しない綴り)。
const DEFINITION: &str = "claude";

/// `stage_count` 段の配布束 (系譜は [`DEFINITION`] と同じ name)。内容版は内容から導出される
/// ので、段数を変えれば別の内容版になる。
fn definition_bundle(stage_count: usize) -> CompiledDefinition {
    let nodes = (0..stage_count)
        .map(|index| {
            StageNodeBuilder::new(
                StageSlug::parse(&format!("stage-{index}")).expect("合成 slug は文法内"),
                StageNumber::parse(&format!("0.{}", index + 1)).expect("合成の番号は文法内"),
                "Stage".to_string(),
                PhaseId::Initialization,
                ExecutionKind::Always,
                StageMode::Inline,
            )
            .build()
        })
        .collect();
    let graph = StageGraph::new(nodes).expect("slug は重複しない");
    let grid = ScopeGrid::from_graph(&graph);
    CompiledDefinition::compile(
        CompiledDefinitionId::parse(DEFINITION).expect("配布束 id"),
        graph,
        grid,
        BTreeMap::new(),
    )
    .0
}

/// 誕生と改訂の対 (定義集約・誕生イベント・改訂イベント)。
fn definition_history() -> (
    WorkflowDefinition,
    WorkflowDefinitionEvent,
    WorkflowDefinitionEvent,
) {
    let (mut definition, defined) = WorkflowDefinition::define(
        WorkflowDefinitionId::parse(DEFINITION).expect("定義 id"),
        &definition_bundle(3),
        at(),
    )
    .expect("配布束は同じ系譜");
    let redefined = definition
        .redefine(&definition_bundle(5), at())
        .expect("内容版が違えば改訂できる");
    (definition, defined, redefined)
}

/// 集約が握っている要求を、省略可能な 3 つまで含めて組み直す。
///
/// `scope` と `request` だけを写すと、`depth` / `test_strategy` / `review` が黙って落ちる。
/// 落ちた値はワイヤにも現れないので、行の検査がその 3 つを素通りしてしまう。
fn request_of(intent: &Intent) -> StartRequest {
    let mut request = StartRequest::new(intent.scope(), intent.request());
    if let Some(depth) = intent.depth() {
        request = request.with_depth(depth);
    }
    if let Some(strategy) = intent.test_strategy() {
        request = request.with_test_strategy(strategy);
    }
    if let Some(review) = intent.review() {
        request = request.with_review(review);
    }
    request
}

#[tokio::test]
async fn the_intent_stream_written_by_the_command_side_keeps_every_optional_request_member() {
    // 書く側と読む側は別々の DTO を持つ (側ごと専用化)。片側にだけ列を足すとワイヤが
    // 食い違い、落ちた値は**読めるが空になる**という形で静かに消える。省略可能な 3 つを
    // 全部埋めた intent を実際に書いて読み戻し、1 つも落ちないことを見る。
    let store = Store::new();
    let held = intent();
    assert_eq!(
        held.review(),
        Some("adversarial"),
        "書く前の集約が握っている"
    );

    {
        let mut repository =
            IntentRepositoryImpl::open(&store.path).expect("intent ストアは開ける");
        let created = IntentEvent::Created(Created::new(
            intent_event_id(),
            held.id().clone(),
            held.definition_id().clone(),
            held.definition_revision().clone(),
            request_of(&held),
            held.stages().to_vec(),
            held.scan().clone(),
        ));
        repository
            .store(&created, &held)
            .await
            .expect("intent の genesis は書ける");
    }

    let reader = JournalReaderImpl::open(&store.path).expect("Reader は開ける");
    let batch = reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("intent の行は読める");

    let intents = batch.intents();
    assert_eq!(intents.len(), 1, "誕生記録 1 行 (実行・定義は無い)");
    let decoded = &intents[0];
    assert_eq!(decoded.review(), held.review(), "review が読む側まで届く");
    assert_eq!(decoded.depth(), held.depth());
    assert_eq!(decoded.test_strategy(), held.test_strategy());
    assert_eq!(decoded.scope(), held.scope());
    assert_eq!(decoded, &held, "誕生の材料は集約値へそのまま戻る");
}

#[tokio::test]
async fn the_definition_stream_written_by_the_command_side_is_read_back_by_the_projection() {
    let store = Store::new();
    let mut repository =
        WorkflowDefinitionRepositoryImpl::open(&store.path).expect("定義ストアは開ける");

    // 誕生 → 改訂。どちらも本番の書き手 (コマンド側の Repository) が本家ストアへ書く。
    let (genesis, defined) = WorkflowDefinition::define(
        WorkflowDefinitionId::parse(DEFINITION).expect("定義 id"),
        &definition_bundle(3),
        at(),
    )
    .expect("配布束は同じ系譜");
    repository
        .store(&defined, &genesis)
        .await
        .expect("誕生は書ける");
    // 楽観 version はストアが採番する — 握り直してから改訂を打つ (書込ユースケースと同型)。
    let mut definition = repository
        .find_by_id(genesis.id())
        .await
        .expect("書いた定義は握り直せる");
    let redefined = definition
        .redefine(&definition_bundle(5), at())
        .expect("内容版が違えば改訂できる");
    repository
        .store(&redefined, &definition)
        .await
        .expect("改訂は書ける");

    // 読む側は同じファイルを別接続で開き、定義の 2 行をドメインイベントへ戻す。
    let reader = JournalReaderImpl::open(&store.path).expect("Reader は開ける");
    let batch = reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("定義の行は読める");

    let definitions = batch.definitions();
    assert_eq!(
        definitions.len(),
        2,
        "誕生と改訂の 2 行 (実行・intent は無い)"
    );
    assert!(batch.executions().is_empty());
    assert!(batch.intents().is_empty());

    assert_eq!(definitions[0].event(), &defined, "誕生が逐語で戻る");
    assert_eq!(definitions[0].seq_nr(), 1);
    assert_eq!(definitions[1].event(), &redefined, "改訂が逐語で戻る");
    assert_eq!(definitions[1].seq_nr(), 2);
    assert!(
        definitions
            .iter()
            .all(|entry| entry.definition_id().as_str() == DEFINITION),
        "系譜 ID は行の `aid` 由来 (改訂は識別子を運ばない)"
    );
    assert!(
        definitions.iter().all(|entry| entry.occurred_at() == &at()),
        "発生時刻は封筒の列から戻る"
    );
}

#[expect(
    clippy::disallowed_methods,
    reason = "契約 JSON ではなくワイヤ形式そのものの一致検査 (BR1.7 の射程外)"
)]
#[test]
fn both_sides_write_the_definition_payload_with_the_same_bytes() {
    // 側ごと専用化した DTO の**ワイヤ形式の同一性**を直接固定する。上のテストが
    // 「読める」ことを示すのに対し、ここは「1 バイトも違わない」ことを示す — 片側にだけ
    // 欄が増える・並びが変わる、といった差分をゴールデン文字列を持たずに検出できる。
    let (_definition, defined, redefined) = definition_history();
    for event in [defined, redefined] {
        let written = serde_json::to_string(&CommandDefinitionEventDto::of(&event))
            .expect("書く側は直列化できる");
        let read = serde_json::to_string(&ProjectionDefinitionEventDto::of(&event))
            .expect("読む側は直列化できる");
        assert_eq!(written, read, "両側のワイヤ形式は同一である");
    }
}

/// 実行イベント 11 変種を 1 つずつ (b40 — 全変種が `id` / `aggregate_id` を運ぶ)。
fn every_execution_variant() -> Vec<IntentExecutionEvent> {
    let ev = || IntentExecutionEventId::generate();
    let agg = execution_id;
    let slug = |s: &str| StageSlug::parse(s).expect("文法内の slug");
    let (_, started) = IntentExecution::start(execution_id(), &intent(), at());
    vec![
        started,
        IntentExecutionEvent::GateOpened(GateOpened::new(
            ev(),
            agg(),
            slug("stage-1"),
            vec!["artifact.md".to_string()],
        )),
        IntentExecutionEvent::GateApproved(GateApproved::new(
            ev(),
            agg(),
            slug("stage-1"),
            Some("ok".to_string()),
        )),
        IntentExecutionEvent::GateRejected(GateRejected::new(
            ev(),
            agg(),
            slug("stage-1"),
            Some("why".to_string()),
        )),
        IntentExecutionEvent::StageRevised(StageRevised::new(ev(), agg(), slug("stage-1"))),
        IntentExecutionEvent::StageSkipped(StageSkipped::new(
            ev(),
            agg(),
            slug("stage-1"),
            "out of scope".to_string(),
        )),
        IntentExecutionEvent::Jumped(Jumped::new(ev(), agg(), slug("stage-0"))),
        IntentExecutionEvent::Parked(Parked::new(ev(), agg(), slug("stage-1"))),
        IntentExecutionEvent::Unparked(Unparked::new(ev(), agg())),
        IntentExecutionEvent::Recomposed(Recomposed::new(
            ev(),
            agg(),
            vec![slug("stage-1")],
            Vec::new(),
        )),
        IntentExecutionEvent::AutonomyModeSet(AutonomyModeSet::new(
            ev(),
            agg(),
            AutonomyMode::Autonomous,
        )),
        IntentExecutionEvent::SingleStageRunCommitted(SingleStageRunCommitted::new(
            ev(),
            agg(),
            slug("contract-design"),
        )),
        IntentExecutionEvent::SkeletonStanceRecorded(SkeletonStanceRecorded::new(
            ev(),
            agg(),
            SkeletonStance::ScopeDependent,
        )),
    ]
}

#[expect(
    clippy::disallowed_methods,
    reason = "契約 JSON ではなくワイヤ形式そのものの横断照合 (BR1.7 の射程外)"
)]
#[test]
fn every_execution_variant_written_by_the_command_side_is_read_back_by_the_projection() {
    // 両側の DTO は**同名の別の型**であり、一致は型ではなくこのテストが保証する
    // (`coding-rules/cqrs-boundaries.md` — 側ごと専用化)。b40 で全変種に `id` と
    // `aggregate_id` が加わり、`Unparked` は単位変種から構造体へ変わったので、変種ごとに
    // 書いて読み戻す照合をここに置く (ITF 駆動の経路は park / jump / recompose を通らない)。
    for event in every_execution_variant() {
        let bytes = serde_json::to_string(&CommandExecutionEventDto::of(&event))
            .expect("書く側の DTO は直列化できる");
        let read: ProjectionExecutionEventDto =
            serde_json::from_str(&bytes).expect("読む側の DTO が同じバイトを受ける");
        assert_eq!(
            read.to_domain().expect("ドメインへ戻せる"),
            event,
            "変種のワイヤ形式が両側で食い違う: {bytes}"
        );
        // 識別子 2 つが実際に行へ載っていること (載っていなければ照合の材料が無い)。
        assert!(
            bytes.contains(event.id().as_str()) && bytes.contains(event.aggregate_id().as_str()),
            "id / aggregate_id が行に載っていない: {bytes}"
        );
    }
}
