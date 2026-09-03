//! DAO 契約テストのフィクスチャ — **行の書き手は RMU 本体**である。
//!
//! 期待値をテストに書き下すと「DAO がリードモデルを引けている」ことを確かめられなくなる
//! ので、行は投影 (`ReadTables::project` / `SteeringTables::pack`) が実際に書いたものを
//! 使う。DAO が読むのはその結果だけである。
//!
//! # ジャーナル表は最小の殻だけ用意する
//!
//! `JournalReaderImpl::open` は本家の `journal` 表の**存在**を検査する (存在しないパスに
//! 空 DB を作らないため)。一方 `advance_checkpoint` は前進先が `GlobalSeqNr::ZERO` のとき
//! アンカー行を引かないので、行の差し替えだけを行うのに本家ストアは要らない。したがって
//! ここでは殻だけを作り、本家のイベントストアを dev-dependency に引かない — クエリ側の
//! テストが書込側の DDL を知る必要は無い。

// テストコードでは unwrap / expect / 添字を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

pub(crate) mod doubles;

use std::collections::BTreeMap;
use std::path::Path;

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    Created, Intent, IntentEventId, IntentExecution, IntentExecutionEvent, IntentExecutionId,
    IntentId, StageDisplay, StageEntry, StartRequest, WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, Defined, DefinitionRevision, ExecutionKind, PhaseId, PlanAction,
    ReviewCapValue, ReviewClass, RuleInContext, RuleScope, ScopeGrid, ScopeMetadata,
    SkeletonDefault, StageGraph, StageMode, StageNode, StageNodeBuilder, StageNumber, StageSlug,
    WorkflowDefinitionEvent, WorkflowDefinitionEventId, WorkflowDefinitionId,
};
use core_command_domain::workspace::{SpaceName, StorePath};
use core_read_model_updater::orchestration::{
    DefinitionEntry, GlobalSeqNr, JournalBatch, JournalEntry, JournalReader, JournalReaderImpl,
    ProjectionName,
};
use core_read_model_updater::read_tables::{MemoryRules, ReadTables, RuleContent, SteeringTables};
use tempfile::TempDir;

/// 定義の識別子 (`harness_name()` の固定値と同じ綴り)。
pub(crate) const DEFINITION: &str = "claude";
/// 実行の識別子。
pub(crate) const EXECUTION: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";
/// intent の識別子。
pub(crate) const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

/// 本家 `journal` 表の最小の殻 (`open` の存在検査を満たすためだけ)。
const JOURNAL_SHELL: &str = "CREATE TABLE IF NOT EXISTS journal (
  aid         TEXT    NOT NULL,
  seq_nr      INTEGER NOT NULL,
  payload     BLOB    NOT NULL,
  occurred_at INTEGER NOT NULL,
  manifest    TEXT    NOT NULL
)";

fn at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-09-03T00:00:00Z")
        .expect("固定の ISO 8601 UTC")
        .with_timezone(&Utc)
}

fn slug(value: &str) -> StageSlug {
    StageSlug::parse(value).expect("テストの slug は文法内")
}

fn definition_id() -> WorkflowDefinitionId {
    WorkflowDefinitionId::parse(DEFINITION).expect("テストの定義 id")
}

fn intent_id() -> IntentId {
    IntentId::parse(INTENT).expect("テストの IntentId は UUIDv7")
}

fn execution_id() -> IntentExecutionId {
    IntentExecutionId::parse(EXECUTION).expect("テストの IntentExecutionId は UUIDv7")
}

fn definition_event_id() -> WorkflowDefinitionEventId {
    WorkflowDefinitionEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0003").expect("UUIDv7")
}

fn intent_event_id() -> IntentEventId {
    IntentEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0001").expect("UUIDv7")
}

fn revision() -> DefinitionRevision {
    DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("内容版")
}

/// 任意フィールドまで埋めたノード (run-stage の全列を行に出すため)。
fn saturated_node() -> StageNode {
    StageNodeBuilder::new(
        slug("intent-capture"),
        StageNumber::parse("1.1").expect("番号"),
        "Intent Capture".to_string(),
        PhaseId::Ideation,
        ExecutionKind::Always,
        StageMode::Mob,
    )
    .lead_agent("aidlc-product-agent".to_string())
    .support_agents(vec!["aidlc-design-agent".to_string()])
    .produces(vec!["intent.md".to_string()])
    .reviewer("aidlc-product-lead-agent".to_string())
    .reviewer_max_iterations(2)
    .review_class(ReviewClass::Adversarial)
    .rules_in_context(vec![RuleInContext::new("org.md", RuleScope::Org)])
    .build()
}

fn graph() -> StageGraph {
    StageGraph::new(vec![
        StageNodeBuilder::new(
            slug("state-init"),
            StageNumber::parse("0.1").expect("番号"),
            "State Init".to_string(),
            PhaseId::Initialization,
            ExecutionKind::Always,
            StageMode::Inline,
        )
        .build(),
        saturated_node(),
        StageNodeBuilder::new(
            slug("requirements-analysis"),
            StageNumber::parse("2.1").expect("番号"),
            "Requirements Analysis".to_string(),
            PhaseId::Inception,
            ExecutionKind::Always,
            StageMode::Subagent,
        )
        .build(),
    ])
    .expect("3 ノードのグラフ")
}

/// `classic` だけがグリッド列を持つ (`express` / `feature` は列なし = コスト NULL)。
fn grid() -> ScopeGrid {
    let column: BTreeMap<StageSlug, PlanAction> = [
        (slug("state-init"), PlanAction::Execute),
        (slug("intent-capture"), PlanAction::Execute),
        (slug("requirements-analysis"), PlanAction::Skip),
    ]
    .into_iter()
    .collect();
    ScopeGrid::new([("classic".to_string(), column)].into_iter().collect())
}

/// scope カタログ 3 件 — 既製 3 scope の並び順を見るために `feature` も置く。
fn scopes() -> BTreeMap<String, ScopeMetadata> {
    let classic = ScopeMetadata::new("classic")
        .expect("名前あり")
        .with_depth("standard".to_string())
        .with_keywords(vec!["api".to_string()])
        .with_skeleton(SkeletonDefault::Off)
        .with_review_cap(ReviewCapValue::Adversarial)
        .with_freeform_default(true);
    let express = ScopeMetadata::new("express")
        .expect("名前あり")
        .with_keywords(vec!["quick".to_string()]);
    let feature = ScopeMetadata::new("feature").expect("名前あり");
    [
        ("classic".to_string(), classic),
        ("express".to_string(), express),
        ("feature".to_string(), feature),
    ]
    .into_iter()
    .collect()
}

fn defined_event() -> WorkflowDefinitionEvent {
    WorkflowDefinitionEvent::Defined(Defined::new(
        definition_event_id(),
        definition_id(),
        revision(),
        graph(),
        grid(),
        scopes(),
    ))
}

fn stages() -> Vec<StageEntry> {
    let display = |number: &str, name: &str, agent: &str| {
        StageDisplay::new(StageNumber::parse(number).expect("番号"), name, agent).expect("単一行")
    };
    vec![
        StageEntry::new(
            slug("state-init"),
            PhaseId::Initialization,
            PlanAction::Execute,
            false,
            display("0.1", "State Init", "orchestrator"),
        ),
        StageEntry::new(
            slug("intent-capture"),
            PhaseId::Ideation,
            PlanAction::Execute,
            false,
            display("1.1", "Intent Capture", "aidlc-product-agent"),
        ),
        StageEntry::new(
            slug("requirements-analysis"),
            PhaseId::Inception,
            PlanAction::Skip,
            false,
            display("2.1", "Requirements Analysis", "aidlc-product-agent"),
        ),
    ]
}

fn intent() -> Intent {
    Intent::from((
        Created::new(
            intent_event_id(),
            intent_id(),
            definition_id(),
            revision(),
            StartRequest::new("classic", "build the thing"),
            stages(),
            WorkspaceScan::new(BrownfieldGreenfield::Brownfield, "Rust", "tokio", "cargo")
                .expect("単一行"),
        ),
        at(),
    ))
}

/// 実行 1 本の歴史 (`state-init` のゲートを開けた直後)。
///
/// `park` が真なら続けて park マーカーを置く。park 中の実行は素の `next` に対して
/// 集約が `Parked` を返すので、`read_next_answer` の `bare` 行は **`stage_slug` を持つのに
/// `run_stage_id` は NULL** という形になる (`NextAnswerRow::of` — FK は決定が run-stage の
/// ときだけ書かれる)。同じ実行の `reentry` 行は park ガードを外して `RunStage` になるので
/// FK を持つ。この対が「FK をたどる」と「自然キーで結合し直す」の違いを見分ける。
fn execution_events(park: bool) -> Vec<(usize, IntentExecutionEvent)> {
    let intent = intent();
    let (mut aggregate, started) = IntentExecution::start(execution_id(), &intent, at());
    let mut events = vec![(aggregate.seq_nr(), started)];
    let opened = aggregate
        .open_gate(&intent, vec!["state.md".to_string()], at())
        .expect("ゲートは開く");
    events.push((aggregate.seq_nr(), opened));
    if park {
        let parked = aggregate.park(&intent, at()).expect("park は置ける");
        events.push((aggregate.seq_nr(), parked));
    }
    events
}

fn history(park: bool) -> JournalBatch {
    let mut executions = Vec::new();
    let mut global = 2_u64;
    for (seq_nr, event) in execution_events(park) {
        executions.push(JournalEntry::new(
            GlobalSeqNr::new(global),
            execution_id(),
            seq_nr,
            at(),
            event,
        ));
        global += 1;
    }
    let definitions = vec![DefinitionEntry::new(
        GlobalSeqNr::new(1),
        definition_id(),
        1,
        at(),
        defined_event(),
    )];
    JournalBatch::new(
        executions,
        vec![intent()],
        definitions,
        Some(GlobalSeqNr::new(global - 1)),
    )
}

/// steering の参照入力 — base 1 本 + ideation のフェーズ規則。
fn memory_rules() -> MemoryRules {
    let phases = [(
        PhaseId::Ideation,
        RuleContent::new("phases/ideation.md".to_string(), "# Ideation\n".to_string()),
    )]
    .into_iter()
    .collect();
    MemoryRules::new(
        vec![RuleContent::new(
            "org.md".to_string(),
            "# Org\n".to_string(),
        )],
        phases,
    )
}

/// 投影済みのストア 1 つ。
pub(crate) struct Fixture {
    _dir: TempDir,
    path: StorePath,
}

impl Fixture {
    /// 稼働中の実行 1 本を投影した一時ストア。
    pub(crate) fn projected() -> Fixture {
        Fixture::write(&history(false))
    }

    /// park 中の実行 1 本を投影した一時ストア。
    ///
    /// `bare` の答えが `parked` になり、その行は `stage_slug` を持ちながら `run_stage_id` を
    /// 持たない。「FK が NULL なら材料は無い」を実データで押さえるための断面である。
    pub(crate) fn parked() -> Fixture {
        Fixture::write(&history(true))
    }

    /// 一時ストアを作り、RMU に全 17 表を書かせる。
    fn write(batch: &JournalBatch) -> Fixture {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let path = StorePath::for_space(&dir.path().join("aidlc"), &SpaceName::default());
        std::fs::create_dir_all(path.as_path().parent().expect("親 dir を持つ"))
            .expect("intents/ を先に作る");
        rusqlite::Connection::open(path.as_path())
            .expect("殻を作る接続")
            .execute_batch(JOURNAL_SHELL)
            .expect("journal の殻");

        let tables = ReadTables::project(batch).expect("健全な履歴は投影できる");
        let steering = SteeringTables::pack(&memory_rules()).expect("規則束は分割できる");
        let projection = ProjectionName::parse("read-model").expect("投影名は kebab");
        let mut reader = JournalReaderImpl::open(&path).expect("Reader は開ける");
        // `GlobalSeqNr::ZERO` への前進はアンカーを引かない — 行の差し替えだけが起きる。
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("current_thread ランタイム")
            .block_on(async {
                reader
                    .advance_checkpoint(&projection, GlobalSeqNr::ZERO, &tables)
                    .await
                    .expect("ジャーナル由来 15 表の差し替え");
                reader
                    .replace_steering(&steering)
                    .await
                    .expect("参照入力由来 2 表の差し替え");
            });
        Fixture { _dir: dir, path }
    }

    /// DAO が開くストアファイル。
    pub(crate) fn store(&self) -> &Path {
        self.path.as_path()
    }
}
