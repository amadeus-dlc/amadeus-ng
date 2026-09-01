//! RMU の試験装置 — 読む対象のジャーナル行を**本家のイベントストアに直接書かせる**。
//!
//! # なぜコマンド側の Repository を使わないのか
//!
//! 禁止されているからではない。RMU は中間なのでコマンド側クレートを dev-dependency に書いても
//! 規則違反ではない (`coding-rules/cqrs-boundaries.md` — 2026-08-29 是正)。**そのほうが試験
//! 対象に忠実だから**である。`JournalReaderImpl` が結合しているのは本家の `journal` 表であって、
//! 我々の Repository ではない。行を本家に書かせれば、読む側のテストは「本家が書いた行を我々が
//! 読めるか」だけを見ることになり、コマンド側の実装が変わっても揺れない。
//!
//! 集約とドメインイベントは `core-command-domain` の型である。ドメインはコマンド側の持ち物だが、
//! 中間である RMU はそれに依存してよい。
//!
//! # 行のバイトはこの側の DTO で組む (改訂 9)
//!
//! ドメインは永続化知識から中立になったので、封筒に載せる payload とストア鍵は
//! **RMU 自身の** `orchestration::dto` が持つ型である
//! (`coding-rules/domain-persistence-neutrality.md` / `cqrs-boundaries.md`)。書く側の DTO を
//! 借りていないので、このテストは「読む側の綴りで書いた行を読む」ことしか示さない —
//! 書く側との一致は横断適合テスト (`journal_protocol_conformance`) が固定する。

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    AutonomyMode, CommandError, Created, Intent, IntentExecution, IntentExecutionEvent,
    IntentExecutionId, IntentId, StageDisplay, StageEntry, StartRequest, WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
    WorkflowDefinitionId,
};
use core_command_domain::workspace::StorePath;
use core_read_model_updater::orchestration::{IntentEventDto, IntentExecutionEventDto};
use event_store_adapter_rs::EventStoreForSqlite;
use event_store_adapter_rs::event_envelope::EventEnvelope;
use event_store_adapter_rs::types::{AggregateId, EventStore};

/// ジャーナル行 `manifest` 列に書く型判別子 (読む側の定数と同じ綴り)。
pub(crate) const MANIFEST: &str = "intent-execution-event/1";

/// 本家 `AggregateId` を満たすストア鍵 (テストが行を書くためだけに要る)。
///
/// RMU の本番経路は `rusqlite` で `journal` 表を直接読むので、この鍵は使わない。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct StoreKey(String);

impl StoreKey {
    pub(crate) fn of(id: &IntentExecutionId) -> StoreKey {
        StoreKey(id.as_str().to_string())
    }
}

impl std::fmt::Display for StoreKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AggregateId for StoreKey {
    fn type_name(&self) -> String {
        "IntentExecution".to_string()
    }

    fn value(&self) -> String {
        self.0.clone()
    }
}

/// 本家の SQLite イベントストア (ジャーナル行の書き手)。
///
/// 集約 payload は `serde_json::Value` である — RMU はスナップショット行を読まないので、
/// この側にスナップショットの DTO は無い (`orchestration::dto` の doc を参照)。
pub(crate) type UpstreamStore =
    EventStoreForSqlite<StoreKey, serde_json::Value, IntentExecutionEventDto>;

/// イベントの `occurred_at` の逐語形 (集約は値を素通しするので固定値でよい)。
pub(crate) const AT_TEXT: &str = "2026-08-23T00:00:00Z";

/// テストの intent 識別子 (UUIDv7)。
pub(crate) const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

/// テストの実行識別子 (UUIDv7)。
///
/// intent と**別の値**であることは前提である — 本家の journal は `(aid, seq_nr)` に UNIQUE
/// 索引を type_name 抜きの生値で張るため、同じストアに同居する 2 ストリームの識別子の値は
/// 一意でなければならない (issue #50)。
pub(crate) const EXECUTION: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";

/// 2 集約を並べるときの相手側 (UUIDv7)。
pub(crate) const OTHER_INTENT: &str = "018f3b2c-4d5e-7f60-8abc-def012345678";

/// イベントの `occurred_at`。
#[must_use]
pub(crate) fn at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(AT_TEXT)
        .expect("固定の ISO 8601 UTC")
        .with_timezone(&Utc)
}

/// テストの intent 識別子。
#[must_use]
pub(crate) fn intent_id() -> IntentId {
    IntentId::parse(INTENT).expect("テストの IntentId は UUIDv7")
}

/// テストの実行識別子 (ジャーナル行の集約キー)。
#[must_use]
pub(crate) fn execution_id() -> IntentExecutionId {
    IntentExecutionId::parse(EXECUTION).expect("テストの IntentExecutionId は UUIDv7")
}

/// 相手側の実行識別子。
#[must_use]
pub(crate) fn other_execution_id() -> IntentExecutionId {
    IntentExecutionId::parse(OTHER_INTENT).expect("テストの IntentExecutionId は UUIDv7")
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
pub(crate) fn intent() -> Intent {
    Intent::from((
        Created::new(
            intent_id(),
            WorkflowDefinitionId::parse("claude").expect("テストの定義 id"),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64)))
                .expect("テストの定義 revision"),
            StartRequest::new("classic", "contract").with_depth("standard"),
            stages(),
            scan(),
        ),
        at(),
    ))
}

/// 指定した実行識別子の genesis (横断読取のテストが 2 実行を並べるのに使う)。
#[must_use]
pub(crate) fn genesis_for(execution: IntentExecutionId) -> (IntentExecution, IntentExecutionEvent) {
    IntentExecution::start(execution, &intent(), at())
}

/// 1 つの集約を本家のストアへ書き進める書き手。
///
/// 楽観 version は本家の規約どおり「新規作成は 0、以後は 1 件書くごとに 1 つ進む」で追う —
/// 読み直さずに追えるのは、この試験装置が唯一の書き手だからである。
pub(crate) struct JournalWriter {
    aggregate: IntentExecution,
    version: usize,
}

impl JournalWriter {
    /// genesis を書いて書き手を得る。
    pub(crate) async fn start(
        store: &mut UpstreamStore,
        execution: IntentExecutionId,
    ) -> JournalWriter {
        let (aggregate, event) = genesis_for(execution);
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
        F: FnOnce(&mut IntentExecution) -> Result<IntentExecutionEvent, CommandError>,
    {
        let event = command(&mut self.aggregate).expect("コマンドは受理される");
        self.persist(store, &event).await;
    }

    /// 適用後の集約から本家の封筒を組んで書く (payload は読む側の DTO)。
    async fn persist(&mut self, store: &mut UpstreamStore, event: &IntentExecutionEvent) {
        let envelope = EventEnvelope::new(
            StoreKey::of(self.aggregate.id()),
            self.aggregate.seq_nr(),
            *self.aggregate.last_updated_at(),
            IntentExecutionEventDto::of(event),
        )
        .with_manifest(MANIFEST);
        // スナップショット行の中身は RMU の関心外である (読むのは journal 表だけ)。
        store
            .persist_event_and_snapshot(envelope, serde_json::Value::Null, self.version)
            .await
            .expect("本家ストアは書ける");
        self.version += 1;
    }
}

/// 4 件のジャーナル行を書く (`Started` / `GateOpened` / `GateApproved` /
/// `AutonomyModeSet`)。読み方の約束を見るテストが共通で使う土台である。
///
/// 誕生 = 初期化完了済み (issue #76) により、かつて先頭にあった `StageCompleted`
/// (索引 0 = 非ゲートの initialization を完了させる 1 件) は構成不能になった — 誕生の
/// 時点でその checkbox は completed で、カーソルは索引 1 のゲート付きステージに立って
/// いる。前置きが 1 件消えたぶん、以後の通番が 1 つずつ詰まる。
pub(crate) async fn seed(store: &mut UpstreamStore) {
    let mut writer = JournalWriter::start(store, execution_id()).await;
    writer
        .advance(store, |aggregate| {
            aggregate.open_gate(&intent(), vec!["intent.md".to_string()], at())
        })
        .await;
    writer
        .advance(store, |aggregate| {
            aggregate.approve_gate(&intent(), Some("ok".to_string()), at())
        })
        .await;
    writer
        .advance(store, |aggregate| {
            aggregate.switch_autonomy(&intent(), AutonomyMode::Autonomous, at())
        })
        .await;
}

/// ストアファイルを開く (存在しなければ本家が表ごと作る)。
#[must_use]
pub(crate) fn open_store(path: &StorePath) -> UpstreamStore {
    UpstreamStore::new(path.as_path()).expect("本家ストアは開ける")
}

/// intent ストリームのストア鍵 (`type_name = "Intent"` — 実行の行と同じファイルに同居する)。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct IntentStoreKey(String);

impl std::fmt::Display for IntentStoreKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AggregateId for IntentStoreKey {
    fn type_name(&self) -> String {
        "Intent".to_string()
    }

    fn value(&self) -> String {
        self.0.clone()
    }
}

/// intent の誕生記録 (`Created`) を 1 行書く (aid = intent 識別子、seq_nr = 1)。
///
/// payload は読む側の DTO ([`IntentEventDto`]) で組む — 書く側との一致は横断適合テスト
/// (`journal_protocol_conformance`) が固定する (実行の行と同じ理屈)。
pub(crate) async fn seed_intent(path: &StorePath) {
    let mut store: EventStoreForSqlite<IntentStoreKey, serde_json::Value, IntentEventDto> =
        EventStoreForSqlite::new(path.as_path()).expect("本家ストアは開ける");
    let held = intent();
    let envelope = EventEnvelope::new(
        IntentStoreKey(held.id().as_str().to_string()),
        1,
        at(),
        IntentEventDto::of(&held),
    )
    .with_manifest("intent-event/1");
    store
        .persist_event_and_snapshot(envelope, serde_json::Value::Null, 0)
        .await
        .expect("本家ストアは書ける");
}
