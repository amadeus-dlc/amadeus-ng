//! 本家 event-store-adapter-rs v3.0.0 への適合テスト (ADR-010 Conformist / B7)。
//!
//! 型が揃うことは適合の証明にならない。**本家の memory バックエンドにこの層の永続化 DTO を
//! 実際に永続化し、スナップショット + リプレイでドメインへ復元できる**ところまでを固定する。
//!
//! **B12 改訂 9 で検証対象が adapter の面へ移った**ため、このテストも domain からこの層へ移設した。ドメインは永続化知識から中立になり
//! (`coding-rules/domain-persistence-neutrality.md`)、本家ストアに載るのは DTO だからである。
//! したがって検証の主語も「ドメイン型が本家契約を満たす」から「**この層の DTO が本家契約を
//! 満たし、往復でドメインへ戻る**」へ変わった — 往復の両端でドメイン値が一致することを
//! 見るので、確認している性質そのものは落ちていない。
//!
//! v3 で `Event` / `Aggregate` trait は消え、ストアに載るのはライブラリ trait を一切実装しない
//! 素の serde 型になった。改訂 9 以降その型は**この層の DTO** であり、ドメイン型ではない。
//! 境界を越えるメタデータは `EventEnvelope` / `SnapshotEnvelope` が運ぶ。
//! ここで固定する本家の意味論は次の 4 つである:
//!
//! - `persist_event`（イベントのみ）は `seq_nr == 1` を**受理しない** — 新規作成は
//!   `persist_event_and_snapshot` である (`ContractViolation`)
//! - `persist_event_and_snapshot` は `seq_nr == 1` ⇔ `expected_version == 0` の対応を要求し、
//!   崩れた呼出しを `ContractViolation` で拒否する
//! - `get_events_by_id_since_seq_nr` の `seq_nr` は**その番号を含む**
//! - 楽観 version はストアが採番し、`SnapshotEnvelope::version()` が正本である
//!   (集約 payload には載らない)

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{DateTime, TimeDelta, Utc};
use core_command_domain::orchestration::{
    Intent, IntentExecution, IntentExecutionEvent, IntentExecutionId, IntentId, StageDisplay,
    StageEntry, StartRequest, WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
    WorkflowDefinitionId,
};
use core_command_domain::workspace::CheckboxState;
use core_command_interface_adapter::orchestration::{
    AggregateKey, IntentExecutionMemoryStore, WireEvent, WireSnapshot,
};
use event_store_adapter_rs::event_envelope::EventEnvelope;
use event_store_adapter_rs::types::{EventStore, EventStoreWriteError};

/// 本家 memory バックエンドを我々の 3 つの型で具体化したストア。
///
/// 型引数は v3 の並びで `AID` (集約 ID) / `A` (集約 payload) / `P` (イベント payload)。
type Store = IntentExecutionMemoryStore;

/// 我々が封筒に載せる型判別子 (Repository が書く値と同じ綴り)。
const MANIFEST: &str = "intent-execution-event/1";

/// 新規作成の `expected_version` (本家 v3 の規約 — BR2.6)。
const CREATE_EXPECTED_VERSION: usize = 0;

const RAW_ID: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

fn intent_id() -> IntentId {
    IntentId::parse(RAW_ID).unwrap()
}

fn at(offset_secs: i64) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-08-23T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
        + TimeDelta::seconds(offset_secs)
}

fn slug(s: &str) -> StageSlug {
    StageSlug::parse(s).unwrap()
}

/// 合成計画の表示属性。
fn display(number: &str, name: &str) -> StageDisplay {
    StageDisplay::new(StageNumber::parse(number).unwrap(), name, "orchestrator").unwrap()
}

/// 合成計画の走査結果。
fn scan() -> WorkspaceScan {
    WorkspaceScan::new(
        BrownfieldGreenfield::Greenfield,
        "Unknown",
        "Unknown",
        "Unknown",
    )
    .unwrap()
}

/// initialization 1 ステージ + ゲート付き 2 ステージの合成計画 (BR2.5 と同じ流儀)。
fn stages() -> Vec<StageEntry> {
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
            slug("requirements-analysis"),
            PhaseId::Inception,
            PlanAction::Execute,
            false,
            display("2.1", "Requirements Analysis"),
        ),
    ]
}

fn intent() -> Intent {
    Intent::from_material(
        intent_id(),
        WorkflowDefinitionId::parse("claude").unwrap(),
        DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
        StartRequest::new("mvp", "本家ストアへの適合を確かめる"),
        stages(),
        scan(),
    )
    .unwrap()
}

fn execution_id() -> IntentExecutionId {
    IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap()
}

fn genesis() -> (IntentExecution, IntentExecutionEvent) {
    IntentExecution::start(execution_id(), intent(), at(0))
}

/// commit 済みの集約とイベントから封筒を組む (Repository と同じ手順 — B7 裁定 3)。
///
/// 通番も発生時刻も**適用後の集約**が持っている。ドメインは封筒を作らない。
fn envelope(
    aggregate: &IntentExecution,
    event: &IntentExecutionEvent,
) -> EventEnvelope<AggregateKey, WireEvent> {
    EventEnvelope::new(
        AggregateKey::of(aggregate.id()),
        aggregate.seq_nr(),
        *aggregate.last_updated_at(),
        WireEvent::of(event),
    )
    .with_manifest(MANIFEST)
}

/// 集約をストアへ載せる形 (スナップショット行の payload)。
fn snapshot_of(aggregate: &IntentExecution) -> WireSnapshot {
    WireSnapshot::of(aggregate)
}

/// 再水和の結果 (本家 v3 の移行ガイド §3 と同じ形 — 集約 + ストアが載せた版)。
struct Replayed {
    aggregate: IntentExecution,
    version: usize,
}

/// 本家の推奨手順そのままの再構成 — 最新スナップショット封筒 + その先のイベント封筒。
async fn find_by_id(store: &Store, id: &IntentExecutionId) -> Option<Replayed> {
    let key = AggregateKey::of(id);
    let snapshot = store.get_latest_snapshot_by_id(&key).await.unwrap()?;
    let version = snapshot.version();
    // 往復の折り返し点 — DTO で受けて、検査点を通してドメインへ戻す。
    let mut aggregate = snapshot.into_aggregate().to_domain().unwrap();
    let envelopes = store
        .get_events_by_id_since_seq_nr(&key, aggregate.seq_nr() + 1)
        .await
        .unwrap();
    for envelope in &envelopes {
        assert_eq!(envelope.manifest(), MANIFEST, "manifest は往復する");
        let event = envelope.payload().to_domain().unwrap();
        aggregate
            .apply_event(
                &intent(),
                envelope.seq_nr(),
                *envelope.occurred_at(),
                &event,
            )
            .unwrap();
    }
    Some(Replayed { aggregate, version })
}

#[tokio::test]
async fn an_unknown_aggregate_has_no_snapshot() {
    let store = Store::new();
    assert!(find_by_id(&store, &execution_id()).await.is_none());
}

#[tokio::test]
async fn the_creation_event_is_refused_by_the_event_only_write() {
    // v3 は `is_created()` を持たず、create / update の分岐を `seq_nr == 1` から導出する。
    // イベントのみの書込 API は新規作成を受け付けない (BR2.2)。
    let mut store = Store::new();
    let (aggregate, started) = genesis();
    let error = store
        .persist_event(envelope(&aggregate, &started), CREATE_EXPECTED_VERSION)
        .await
        .expect_err("seq_nr == 1 は persist_event では書けない");
    assert!(
        matches!(error, EventStoreWriteError::ContractViolation(_)),
        "実際: {error:?}"
    );
}

#[tokio::test]
async fn a_creation_with_a_non_zero_expected_version_violates_the_contract() {
    // BR2.6: seq_nr == 1 ⇔ expected_version == 0。対応が崩れる呼出しは契約違反である。
    let mut store = Store::new();
    let (aggregate, started) = genesis();
    let error = store
        .persist_event_and_snapshot(envelope(&aggregate, &started), snapshot_of(&aggregate), 1)
        .await
        .expect_err("seq_nr == 1 に版 1 は対応しない");
    assert!(
        matches!(error, EventStoreWriteError::ContractViolation(_)),
        "実際: {error:?}"
    );
}

#[tokio::test]
async fn the_aggregate_survives_a_snapshot_and_replay_round_trip_through_the_upstream_store() {
    let mut store = Store::new();

    // 1. genesis — `seq_nr == 1` なので create 経路へ入る (expected_version = 0)。
    let (aggregate, started) = genesis();
    store
        .persist_event_and_snapshot(
            envelope(&aggregate, &started),
            snapshot_of(&aggregate),
            CREATE_EXPECTED_VERSION,
        )
        .await
        .unwrap();

    let restored = find_by_id(&store, &execution_id()).await.unwrap();
    assert_eq!(restored.aggregate, aggregate, "genesis がそのまま戻る");
    assert_eq!(restored.aggregate.seq_nr(), 1);
    assert_eq!(restored.aggregate.last_updated_at(), &at(0));
    assert_eq!(restored.version, 1, "最初の版はストアが 1 で採番する");

    // 2. スナップショット同時更新 — 本家が version を 1 つ進める。
    let mut aggregate = restored.aggregate;
    let completed = aggregate.complete_stage(&intent(), at(1)).unwrap();
    store
        .persist_event_and_snapshot(
            envelope(&aggregate, &completed),
            snapshot_of(&aggregate),
            restored.version,
        )
        .await
        .unwrap();

    let restored = find_by_id(&store, &execution_id()).await.unwrap();
    assert_eq!(restored.aggregate.seq_nr(), 2);
    assert_eq!(restored.version, 2, "楽観 version はストアが採番する");
    assert_eq!(
        restored.aggregate.cursor(),
        restored.aggregate.stage_index(1).unwrap()
    );
    assert_eq!(restored.aggregate.last_updated_at(), &at(1));

    // 3. ジャーナルだけへの追記 — スナップショットは進まないので、復元はリプレイを通る。
    let mut aggregate = restored.aggregate;
    let opened = aggregate
        .open_gate(&intent(), vec!["docs/x.md".to_string()], at(2))
        .unwrap();
    store
        .persist_event(envelope(&aggregate, &opened), restored.version)
        .await
        .unwrap();

    let restored = find_by_id(&store, &execution_id()).await.unwrap();
    assert_eq!(
        restored.aggregate.seq_nr(),
        3,
        "スナップショットの先をリプレイで追いつく"
    );
    assert_eq!(
        restored
            .aggregate
            .checkbox(restored.aggregate.stage_index(1).unwrap()),
        Some(CheckboxState::AwaitingApproval)
    );
    assert_eq!(restored.aggregate.last_updated_at(), &at(2));
    assert_eq!(restored.version, 3, "イベントのみの書込でも版は進む");

    // 4. もう 1 度スナップショット同時更新 — 直前のリプレイ結果から続けられる。
    let mut aggregate = restored.aggregate;
    let approved = aggregate.approve_gate(&intent(), None, at(3)).unwrap();
    store
        .persist_event_and_snapshot(
            envelope(&aggregate, &approved),
            snapshot_of(&aggregate),
            restored.version,
        )
        .await
        .unwrap();

    let restored = find_by_id(&store, &execution_id()).await.unwrap();
    assert_eq!(restored.aggregate.seq_nr(), 4);
    assert_eq!(restored.version, 4, "版は書込のたびにストアが 1 つ進める");
    assert_eq!(
        restored.aggregate.cursor(),
        restored.aggregate.stage_index(2).unwrap()
    );
    assert_eq!(
        restored
            .aggregate
            .checkbox(restored.aggregate.stage_index(1).unwrap()),
        Some(CheckboxState::Completed)
    );
    assert_eq!(
        restored
            .aggregate
            .approved(restored.aggregate.stage_index(1).unwrap()),
        Some(true)
    );
    assert_eq!(restored.aggregate.last_updated_at(), &at(3));
}

#[tokio::test]
async fn the_journal_keeps_every_envelope_in_order_with_its_metadata() {
    let mut store = Store::new();
    let id = execution_id();

    let (mut aggregate, started) = genesis();
    store
        .persist_event_and_snapshot(
            envelope(&aggregate, &started),
            snapshot_of(&aggregate),
            CREATE_EXPECTED_VERSION,
        )
        .await
        .unwrap();
    let completed = aggregate.complete_stage(&intent(), at(1)).unwrap();
    store
        .persist_event_and_snapshot(envelope(&aggregate, &completed), snapshot_of(&aggregate), 1)
        .await
        .unwrap();

    // `seq_nr` 引数はその番号を含む (本家 `fetch_events_since` は `>=`)。
    let events = store
        .get_events_by_id_since_seq_nr(&AggregateKey::of(&id), 1)
        .await
        .unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(
        events.iter().map(EventEnvelope::seq_nr).collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(
        events
            .iter()
            .map(EventEnvelope::occurred_at)
            .collect::<Vec<_>>(),
        [&at(0), &at(1)],
        "発生時刻はドメイン供給値のまま往復する"
    );
    assert!(
        events
            .iter()
            .all(|envelope| envelope.manifest() == MANIFEST),
        "manifest は封筒がそのまま運ぶ"
    );
    assert!(
        events
            .iter()
            .all(|envelope| envelope.aggregate_id() == &AggregateKey::of(&id))
    );

    let tail = store
        .get_events_by_id_since_seq_nr(&AggregateKey::of(&id), 2)
        .await
        .unwrap();
    assert_eq!(
        tail.iter().map(EventEnvelope::seq_nr).collect::<Vec<_>>(),
        [2]
    );
}
