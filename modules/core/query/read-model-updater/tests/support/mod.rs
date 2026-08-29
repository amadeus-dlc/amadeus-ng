//! クエリ側の試験装置 — 読む対象のジャーナル行を**本家のイベントストアに直接書かせる**。
//!
//! # なぜコマンド側の Repository を使わないのか
//!
//! 2026-08-29 の側分割で、クエリ側クレートの `Cargo.toml` にコマンド側クレートを書くことは
//! 禁止された (`coding-rules/cqrs-boundaries.md`)。dev-dependency も `Cargo.toml` に現れる
//! 以上、機械判定では違反である。
//!
//! 都合の話ではなく、**そのほうが試験対象に忠実**でもある。`JournalReaderImpl` が結合して
//! いるのは本家の `journal` 表であって、我々の Repository ではない。行を本家に書かせれば、
//! 読む側のテストは「本家が書いた行を我々が読めるか」だけを見ることになり、コマンド側の
//! 実装が変わっても揺れない。
//!
//! 集約とドメインイベントは共有層 (`core-domain`) の型であり、両側が使ってよい。

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use core_domain::orchestration::{
    AutonomyMode, CommandError, EVENT_MANIFEST, IntentId, StageDisplay, StageEntry, StartRequest,
    WorkflowExecution, WorkflowExecutionEvent, WorkspaceScan,
};
use core_domain::workflow_definition::{
    BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
    WorkflowDefinitionId,
};
use core_domain::workspace::StorePath;
use event_store_adapter_rs::EventStoreForSqlite;
use event_store_adapter_rs::event_envelope::EventEnvelope;
use event_store_adapter_rs::types::EventStore;

/// 本家の SQLite イベントストア (ジャーナル行の書き手)。
pub(crate) type UpstreamStore =
    EventStoreForSqlite<IntentId, WorkflowExecution, WorkflowExecutionEvent>;

/// イベントの `occurred_at` の逐語形 (集約は値を素通しするので固定値でよい)。
pub(crate) const AT_TEXT: &str = "2026-08-23T00:00:00Z";

/// テストの集約識別子 (UUIDv7)。
pub(crate) const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

/// 2 集約を並べるときの相手側 (UUIDv7)。
pub(crate) const OTHER_INTENT: &str = "018f3b2c-4d5e-7f60-8abc-def012345678";

/// イベントの `occurred_at`。
#[must_use]
pub(crate) fn at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(AT_TEXT)
        .expect("固定の ISO 8601 UTC")
        .with_timezone(&Utc)
}

/// テストの集約識別子。
#[must_use]
pub(crate) fn intent_id() -> IntentId {
    IntentId::parse(INTENT).expect("テストの IntentId は UUIDv7")
}

/// 相手側の集約識別子。
#[must_use]
pub(crate) fn other_intent_id() -> IntentId {
    IntentId::parse(OTHER_INTENT).expect("テストの IntentId は UUIDv7")
}

fn slug(value: &str) -> StageSlug {
    StageSlug::parse(value).expect("テストの slug は文法内")
}

/// 合成計画の表示属性 (投影の検収は専用テストが持つので、ここは固定値でよい)。
fn display(number: &str, name: &str) -> StageDisplay {
    StageDisplay::new(
        StageNumber::parse(number).expect("テストのステージ番号は文法内"),
        name,
        "orchestrator",
    )
    .expect("単一行")
}

/// 合成計画の走査結果。
#[must_use]
pub(crate) fn scan() -> WorkspaceScan {
    WorkspaceScan::new(
        BrownfieldGreenfield::Greenfield,
        "Unknown",
        "Unknown",
        "Unknown",
    )
    .expect("単一行")
}

/// 3 ステージの合成計画 (索引 0 = initialization、1〜2 = ideation)。
#[must_use]
pub(crate) fn stages() -> Vec<StageEntry> {
    vec![
        StageEntry::new(
            slug("state-init"),
            PhaseId::Initialization,
            PlanAction::Execute,
            false,
            display("0.1", "State Init"),
        ),
        StageEntry::new(
            slug("intent-capture"),
            PhaseId::Ideation,
            PlanAction::Execute,
            false,
            display("1.1", "Intent Capture"),
        ),
        StageEntry::new(
            slug("scope-definition"),
            PhaseId::Ideation,
            PlanAction::Execute,
            false,
            display("1.4", "Scope Definition"),
        ),
    ]
}

/// 指定した集約識別子の genesis (集約と `Started` イベント)。
#[must_use]
pub(crate) fn genesis_for(intent: IntentId) -> (WorkflowExecution, WorkflowExecutionEvent) {
    WorkflowExecution::start_from_plan_unchecked(
        intent,
        WorkflowDefinitionId::parse("claude").expect("テストの定義 id"),
        DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64)))
            .expect("テストの定義 revision"),
        &StartRequest::new("classic", "contract").with_depth("standard"),
        stages(),
        scan(),
        at(),
    )
    .expect("合成計画は start の前提を満たす")
}

/// 1 つの集約を本家のストアへ書き進める書き手。
///
/// 楽観 version は本家の規約どおり「新規作成は 0、以後は 1 件書くごとに 1 つ進む」で追う —
/// 読み直さずに追えるのは、この試験装置が唯一の書き手だからである。
pub(crate) struct JournalWriter {
    aggregate: WorkflowExecution,
    version: usize,
}

impl JournalWriter {
    /// genesis を書いて書き手を得る。
    pub(crate) async fn start(store: &mut UpstreamStore, intent: IntentId) -> JournalWriter {
        let (aggregate, event) = genesis_for(intent);
        let mut writer = JournalWriter {
            aggregate,
            version: 0,
        };
        writer.persist(store, &event).await;
        writer
    }

    /// コマンドを 1 つ打ち、生まれたイベントを書く。
    pub(crate) async fn advance<F>(&mut self, store: &mut UpstreamStore, command: F)
    where
        F: FnOnce(&mut WorkflowExecution) -> Result<WorkflowExecutionEvent, CommandError>,
    {
        let event = command(&mut self.aggregate).expect("コマンドは受理される");
        self.persist(store, &event).await;
    }

    /// 適用後の集約から本家の封筒を組んで書く (型判別子は共有語彙の `EVENT_MANIFEST`)。
    async fn persist(&mut self, store: &mut UpstreamStore, event: &WorkflowExecutionEvent) {
        let envelope = EventEnvelope::new(
            self.aggregate.intent_id().clone(),
            self.aggregate.seq_nr(),
            *self.aggregate.last_updated_at(),
            event.clone(),
        )
        .with_manifest(EVENT_MANIFEST);
        store
            .persist_event_and_snapshot(envelope, self.aggregate.clone(), self.version)
            .await
            .expect("本家ストアは書ける");
        self.version += 1;
    }
}

/// 5 件のジャーナル行を書く (`Started` / `StageCompleted` / `GateOpened` / `GateApproved` /
/// `AutonomyModeSet`)。読み方の約束を見るテストが共通で使う土台である。
pub(crate) async fn seed(store: &mut UpstreamStore) {
    let mut writer = JournalWriter::start(store, intent_id()).await;
    writer
        .advance(store, |aggregate| aggregate.complete_stage(at()))
        .await;
    writer
        .advance(store, |aggregate| {
            aggregate.open_gate(vec!["intent.md".to_string()], at())
        })
        .await;
    writer
        .advance(store, |aggregate| {
            aggregate.approve_gate(Some("ok".to_string()), None, at())
        })
        .await;
    writer
        .advance(store, |aggregate| {
            aggregate.switch_autonomy(AutonomyMode::Autonomous, at())
        })
        .await;
}

/// ストアファイルを開く (存在しなければ本家が表ごと作る)。
#[must_use]
pub(crate) fn open_store(path: &StorePath) -> UpstreamStore {
    UpstreamStore::new(path.as_path()).expect("本家ストアは開ける")
}
