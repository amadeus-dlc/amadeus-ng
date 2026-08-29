//! ITF 準拠テスト (ADR 0003 決定 5 / FD BR3.5) — `formal/orchestration/journal_protocol.qnt` の
//! トレースを `WorkflowExecutionRepositoryImpl` + `JournalReaderImpl` + フェイク投影に再生し、
//! 全ステップでモデルの状態射影を突き合わせる。
//!
//! フィクスチャは `tests/conformance/fixtures/journal_protocol/` にコミット済み
//! (`#meta` 正規化済み)。各遷移は `lastAction` × `lastActor` で駆動する (lastAction 規約)。
//!
//! 集約の永続化そのものは本家 event-store-adapter-rs が担い、横断読取とチェックポイントは
//! 我々の `JournalReaderImpl` が持つ (ADR-010)。**モデルは 1 文字も変えていない** — 本家に
//! 載せ替えても同じトレースがそのまま再生できることが、この乗り換えの意味論的な検収である。
//!
//! モデルの抽象は「集約 1・writer 2・投影 1」である。writer 2 つは同じ `IntentId` を別々に
//! 再水和した 2 本の「ロード済み集約」で表し、衝突は楽観 version の不一致だけで起きる
//! (ロックは ADR-007 で退役した — BR3.2)。投影はモデルと同じく進捗 (`readModelSeq`) しか
//! 持たないフェイクで、真実源がジャーナルであること・キャッチアップが冪等であることだけを
//! 検査する。
//!
//! 射影規則 (モデル変数 → 実装の観測):
//!   journalLen    = `JournalReader::events_after(ZERO)` の行数 (本家 `journal` の rowid 順)
//!   snapVersion   = 本家 `get_latest_snapshot_by_id` が返す封筒の `version()` (行が無ければ 0)
//!   snapSeq       = 同じ封筒の `seq_nr()` (行が無ければ 0)
//!   checkpoint    = `JournalReader::checkpoint(ProjectionName)` の値 (我々の表)
//!   readModelSeq  = フェイク投影が描き終えた最後の global 通番
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

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use core_domain::orchestration::{
    IntentId, StageEntry, StartRequest, WorkflowExecution, WorkflowExecutionEvent,
};
use core_domain::workflow_definition::{
    DefinitionRevision, PhaseId, PlanAction, StageSlug, WorkflowDefinitionId,
};
use core_domain::workspace::{CheckboxState, SpaceName, StorePath};
use core_interface_adapter::orchestration::{JournalReaderImpl, WorkflowExecutionRepositoryImpl};
use core_use_case::orchestration::{
    GlobalSeqNr, JournalReader, ProjectionName, RehydratedWorkflowExecution, RepositoryError,
    WorkflowExecutionRepository,
};
use event_store_adapter_rs::EventStoreForSqlite;
use event_store_adapter_rs::types::EventStore;
use serde_json::Value;
use tempfile::TempDir;

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

/// モデルの `WRITERS`。
const WRITERS: usize = 2;

/// 合成計画のステージ数。
///
/// フィクスチャは `--max-steps 40` で採取しており、1 ステップが書けるイベントは高々 1 件
/// なのでジャーナルは 40 行を超えない。24 ステージの合成計画は genesis 1 + 索引 0 の完了 1 +
/// ゲート付き 23 ステージ × 2 イベント = 48 件を受け付けるので、再生の途中で「もう打てる
/// コマンドが無い (= ワークフロー完了)」状態には入らない。
const STAGES: usize = 24;

/// 本家の SQLite イベントストア (射影の観測に使う読取ハンドル)。
type UpstreamStore = EventStoreForSqlite<IntentId, WorkflowExecution, WorkflowExecutionEvent>;

/// Repository の具体型 (SQLite バックエンド)。
type Repository = WorkflowExecutionRepositoryImpl<UpstreamStore>;

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

fn intent_id() -> IntentId {
    IntentId::parse(INTENT).expect("再生の IntentId は UUIDv7")
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
        WorkflowExecutionRepositoryImpl::open(&self.path).expect("ストアは開ける")
    }

    /// 投影が使う横断読取 (同じファイルへの別接続)。
    fn reader(&self) -> JournalReaderImpl {
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
            )
        })
        .collect()
}

/// genesis の集約と `Started` イベント (`seq_nr` = 1。版はまだストアに無い)。
fn genesis() -> (WorkflowExecution, WorkflowExecutionEvent) {
    WorkflowExecution::start_from_plan_unchecked(
        intent_id(),
        WorkflowDefinitionId::parse("claude").expect("定義 id"),
        DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("定義 revision"),
        &StartRequest::new("classic", "conformance"),
        stages(),
        at(),
    )
    .expect("合成計画は start の前提を満たす")
}

/// カーソル位置から打てる唯一のコマンドを打ち、1 イベントを得る (1 ステップ 1 イベント)。
///
/// 非ゲートは `complete_stage`、ゲート付きは `open_gate` → `approve_gate` の順。どのコマンドを
/// 打つかは集約の状態だけで決まるので、モデル側に「どのコマンドか」の情報は要らない
/// (モデルは書込の成否だけを持つ抽象である)。
fn next_command(aggregate: &mut WorkflowExecution) -> WorkflowExecutionEvent {
    let cursor = aggregate.cursor();
    let gated = aggregate.gated(cursor).expect("カーソルは範囲内");
    let checkbox = aggregate.checkbox(cursor).expect("カーソルは範囲内");
    let result = if gated {
        if checkbox == CheckboxState::InProgress {
            aggregate.open_gate(vec!["artifact.md".to_string()], at())
        } else {
            aggregate.approve_gate(None, None, at())
        }
    } else {
        aggregate.complete_stage(at())
    };
    result.expect("合成計画はフィクスチャ長ぶんのコマンドを受け付ける (STAGES の見積り)")
}

/// 1 人の writer が握っている「ロード済み集約 + 版」。
struct Writer {
    aggregate: WorkflowExecution,
    /// モデルの `loadedVersion` — この writer が書込に提示する版。
    version: usize,
    /// まだ書けていない genesis の `Started` (書込済みなら `None`)。
    pending: Option<WorkflowExecutionEvent>,
}

impl Writer {
    /// まだ何も書かれていないストアを読んだ writer (genesis を握る — 版は未永続の 0)。
    fn genesis() -> Writer {
        let (aggregate, event) = genesis();
        Writer {
            aggregate,
            version: <Repository as WorkflowExecutionRepository>::UNPERSISTED_VERSION,
            pending: Some(event),
        }
    }

    fn loaded(rehydrated: RehydratedWorkflowExecution) -> Writer {
        Writer {
            version: rehydrated.version(),
            aggregate: rehydrated.into_aggregate(),
            pending: None,
        }
    }

    /// モデルの `loadedVersion` — この writer が書込に提示する版。
    ///
    /// 未永続の genesis を握っている writer は 0 である (ストアには行がまだ無く、新規作成の
    /// 規約が `expected_version == 0` だから)。モデルの初期値と同じ意味になる。
    const fn loaded_version(&self) -> usize {
        self.version
    }

    /// 次の書込に使う (イベント, 集約) の下書き。
    ///
    /// コマンドは**複製**に対して打つ。書込が `Err` になっても writer が握っている集約は
    /// 1 ビットも動かない — モデルの `store_conflict` が `loadedVersion` を変えないことと
    /// 同じ意味論であり、衝突のたびに再水和し直さずに次の試行ができる。
    fn draft(&self) -> (WorkflowExecutionEvent, WorkflowExecution) {
        if let Some(event) = self.pending.clone() {
            return (event, self.aggregate.clone());
        }
        let mut aggregate = self.aggregate.clone();
        let event = next_command(&mut aggregate);
        (event, aggregate)
    }

    /// 書込が通ったので、**ストアが採番した版**を握り直す (BR5.3 — 版を知るのはストアだけ)。
    fn commit(&mut self, stored: RehydratedWorkflowExecution) {
        self.version = stored.version();
        self.aggregate = stored.into_aggregate();
        self.pending = None;
    }
}

/// 投影 (U4) のフェイク — モデルと同じく進捗しか持たない。
#[derive(Debug, Default)]
struct FakeProjection {
    read_model_seq: u64,
}

// ---- 射影の突合 ----

async fn assert_projection(
    store: &Store,
    projection: &FakeProjection,
    writers: &[Writer],
    m: &ModelState,
    label: &str,
) {
    let reader = store.reader();
    let rows = reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("ジャーナルは読める");
    assert_eq!(
        u64::try_from(rows.len()).expect("行数"),
        m.journal_len,
        "{label}: journalLen"
    );
    // 単一集約なので global 通番と seq_nr は 1 から同じ連番になる (失敗した書込は採番しない)。
    for (offset, entry) in rows.iter().enumerate() {
        let expected = offset + 1;
        assert_eq!(
            entry.global_seq().to_u64(),
            u64::try_from(expected).expect("行番号"),
            "{label}: global 通番"
        );
        assert_eq!(entry.seq_nr(), expected, "{label}: seq_nr");
    }

    let snapshot = store
        .snapshot_view()
        .get_latest_snapshot_by_id(&intent_id())
        .await
        .expect("スナップショットは読める");
    let (version, seq_nr) =
        snapshot.map_or((0, 0), |envelope| (envelope.version(), envelope.seq_nr()));
    assert_eq!(version, m.snap_version, "{label}: snapVersion");
    assert_eq!(seq_nr, m.snap_seq, "{label}: snapSeq");

    let checkpoint = reader
        .checkpoint(&projection_name())
        .await
        .expect("チェックポイントは読める");
    assert_eq!(checkpoint.to_u64(), m.checkpoint, "{label}: checkpoint");
    assert_eq!(
        projection.read_model_seq, m.read_model_seq,
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
    let mut repository = store.repository();
    let mut projection = FakeProjection::default();
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
            "load" => match repository.find_by_id(&intent_id()).await {
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
                    .store(&event, &aggregate, held.loaded_version())
                    .await
                    .unwrap_or_else(|error| panic!("{label}: 書込は通るはず {error:?}"));
                assert_eq!(
                    u64::try_from(aggregate.seq_nr()).expect("seq_nr"),
                    m.journal_len,
                    "{label}: 追記された行の seq_nr"
                );
                // 新しい版を採番したのはストアなので、握り直して初めて分かる (BR5.3)。
                let stored = repository
                    .find_by_id(&intent_id())
                    .await
                    .unwrap_or_else(|error| panic!("{label}: 書いた集約は読み直せる {error:?}"));
                writers.get_mut(writer).expect("writer 添字").commit(stored);
            }

            // stale な writer の書込は拒否され、ストアの状態は 1 ビットも変わらない。
            "store_conflict" => {
                let held = writers.get(writer).expect("writer 添字");
                let (event, aggregate) = held.draft();
                let error = repository
                    .store(&event, &aggregate, held.loaded_version())
                    .await
                    .expect_err("stale な writer の書込は拒否される");
                assert_eq!(
                    error,
                    RepositoryError::Conflict {
                        expected: prev.loaded_version_of(writer),
                        actual: prev.snap_version,
                    },
                    "{label}: 衝突の材料"
                );
                // writer は触らない — 下書きは複製に対して打ったので握っている版は動かない。
            }

            // 投影のキャッチアップ — チェックポイント以降を読んで描き、位置を進める。
            "catchup" => {
                let mut reader = store.reader();
                let from = reader
                    .checkpoint(&projection_name())
                    .await
                    .expect("チェックポイントは読める");
                let rows = reader.events_after(from).await.expect("差分は読める");
                let last = rows.last().map(|entry| entry.global_seq());
                if let Some(global) = last {
                    projection.read_model_seq = global.to_u64();
                    reader
                        .advance_checkpoint(&projection_name(), global)
                        .await
                        .expect("前進は受理される");
                }
                // 読むものが無ければ何もしない = 冪等 (projection_idempotent)。
            }

            // Tx 済み・投影未反映のままプロセスが落ちる。開き直しても永続状態は同じ。
            "crash" => {
                // 同じファイルへ新しい接続を開き直す (= 書き終えた行は落ちない)。
                repository = store.repository();
                if m.journal_len > 0 {
                    let rebuilt = repository
                        .find_by_id(&intent_id())
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
