//! 本家 event-store-adapter-rs v2.0.0 への適合テスト (ADR-010 Conformist)。
//!
//! trait 実装がコンパイルを通ることは適合の証明にならない。**本家の memory バックエンドに
//! 我々の集約を実際に永続化し、スナップショット + リプレイで復元できる**ところまでを固定する。
//!
//! 本家の意味論のうち、ここで固定するもの:
//!
//! - `Event::is_created` が真のイベント (genesis) は `persist_event` では受理されない
//!   (本家 `GenericEventStore::persist_event` が明示的に拒む)
//! - `persist_event_and_snapshot` は genesis を create 経路、それ以外を update 経路へ送る
//! - `get_events_by_id_since_seq_nr` の `seq_nr` は**その番号を含む** (我々のローカル
//!   `EventStore` ポートの「より後」とは境界が 1 ずれる — 委任 2 の申し送り事項)
//! - 楽観 version はストアが採番する。呼出側は永続化のたびに読み直す

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{DateTime, TimeDelta, Utc};
use core_domain::orchestration::{
    IntentId, StageEntry, StartRequest, WorkflowExecution, WorkflowExecutionEvent,
    WorkflowExecutionEventId,
};
use core_domain::workflow_definition::{
    DefinitionRevision, PhaseId, PlanAction, StageSlug, WorkflowDefinitionId,
};
use core_domain::workspace::CheckboxState;
use event_store_adapter_rs::EventStoreForMemory;
use event_store_adapter_rs::types::{Aggregate, Event, EventStore};
use std::num::NonZeroUsize;

/// 本家 memory バックエンドを我々の 3 つのドメイン型で具体化したストア。
type Store = EventStoreForMemory<IntentId, WorkflowExecution, WorkflowExecutionEvent>;

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

/// initialization 1 ステージ + ゲート付き 2 ステージの合成計画 (BR2.5 と同じ流儀)。
fn stages() -> Vec<StageEntry> {
    vec![
        StageEntry::new(
            slug("state-init"),
            PhaseId::Initialization,
            PlanAction::Execute,
            false,
        ),
        StageEntry::new(
            slug("intent-capture"),
            PhaseId::Ideation,
            PlanAction::Execute,
            false,
        ),
        StageEntry::new(
            slug("requirements-analysis"),
            PhaseId::Inception,
            PlanAction::Execute,
            false,
        ),
    ]
}

fn genesis() -> (WorkflowExecution, WorkflowExecutionEvent) {
    WorkflowExecution::start_from_plan_unchecked(
        intent_id(),
        WorkflowDefinitionId::parse("claude").unwrap(),
        DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
        &StartRequest::new("mvp", "本家ストアへの適合を確かめる"),
        stages(),
        at(0),
    )
    .unwrap()
}

/// 本家の推奨手順そのままの再構成 — 最新スナップショット + その `seq_nr` 以降のイベント。
async fn find_by_id(store: &Store, id: &IntentId) -> Option<WorkflowExecution> {
    let snapshot = store.get_latest_snapshot_by_id(id).await.unwrap()?;
    let events = store
        .get_events_by_id_since_seq_nr(id, snapshot.seq_nr() + 1)
        .await
        .unwrap();
    let mut replayed = snapshot;
    for event in &events {
        replayed.apply_event(event).unwrap();
    }
    Some(replayed)
}

#[tokio::test]
async fn an_unknown_aggregate_has_no_snapshot() {
    let store = Store::new();
    assert!(find_by_id(&store, &intent_id()).await.is_none());
}

#[tokio::test]
async fn the_genesis_event_is_refused_by_the_append_only_write() {
    // `Event::is_created` の配線を実挙動で固定する — 本家は creation を `persist_event`
    // では受け付けない。
    let mut store = Store::new();
    let (_, started) = genesis();
    assert!(started.is_created());
    assert!(store.persist_event(&started, 0).await.is_err());
}

#[tokio::test]
async fn the_aggregate_survives_a_snapshot_and_replay_round_trip_through_the_upstream_store() {
    let mut store = Store::new();
    let id = intent_id();

    // 1. genesis — `is_created` が真なので create 経路へ入る。
    let (aggregate, started) = genesis();
    store
        .persist_event_and_snapshot(&started, &aggregate)
        .await
        .unwrap();

    let restored = find_by_id(&store, &id).await.unwrap();
    assert_eq!(restored, aggregate, "genesis がそのまま戻る");
    assert_eq!(restored.seq_nr(), 1);
    assert_eq!(restored.last_updated_at(), &at(0));

    // 2. スナップショット同時更新 — 本家が version を 1 つ進める。
    let mut aggregate = restored;
    let completed = aggregate.complete_stage(at(1)).unwrap();
    store
        .persist_event_and_snapshot(&completed, &aggregate)
        .await
        .unwrap();

    let restored = find_by_id(&store, &id).await.unwrap();
    assert_eq!(restored.seq_nr(), 2);
    assert_eq!(restored.version(), 1, "楽観 version はストアが採番する");
    assert_eq!(restored.cursor(), restored.stage_index(1).unwrap());
    assert_eq!(restored.last_updated_at(), &at(1));

    // 3. ジャーナルだけへの追記 — スナップショットは進まないので、復元はリプレイを通る。
    let mut aggregate = restored;
    let opened = aggregate
        .open_gate(vec!["docs/x.md".to_string()], at(2))
        .unwrap();
    store
        .persist_event(&opened, aggregate.version())
        .await
        .unwrap();

    let restored = find_by_id(&store, &id).await.unwrap();
    assert_eq!(
        restored.seq_nr(),
        3,
        "スナップショットの先をリプレイで追いつく"
    );
    assert_eq!(
        restored.checkbox(restored.stage_index(1).unwrap()),
        Some(CheckboxState::AwaitingApproval)
    );
    assert_eq!(restored.last_updated_at(), &at(2));

    // 4. もう 1 度スナップショット同時更新 — 直前のリプレイ結果から続けられる。
    let mut aggregate = restored;
    let approved = aggregate.approve_gate(None, None, at(3)).unwrap();
    store
        .persist_event_and_snapshot(&approved, &aggregate)
        .await
        .unwrap();

    let restored = find_by_id(&store, &id).await.unwrap();
    assert_eq!(restored.seq_nr(), 4);
    assert_eq!(restored.cursor(), restored.stage_index(2).unwrap());
    assert_eq!(
        restored.checkbox(restored.stage_index(1).unwrap()),
        Some(CheckboxState::Completed)
    );
    assert_eq!(
        restored.approved(restored.stage_index(1).unwrap()),
        Some(true)
    );
    assert_eq!(restored.last_updated_at(), &at(3));
}

#[tokio::test]
async fn the_journal_keeps_every_event_in_order_with_a_stable_identifier() {
    let mut store = Store::new();
    let id = intent_id();

    let (mut aggregate, started) = genesis();
    store
        .persist_event_and_snapshot(&started, &aggregate)
        .await
        .unwrap();
    let completed = aggregate.complete_stage(at(1)).unwrap();
    store
        .persist_event_and_snapshot(&completed, &aggregate)
        .await
        .unwrap();

    // `seq_nr` 引数はその番号を含む (本家 `fetch_events_since` は `>=`)。
    let events = store.get_events_by_id_since_seq_nr(&id, 1).await.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events.iter().map(Event::seq_nr).collect::<Vec<_>>(), [1, 2]);
    assert_eq!(
        events.iter().map(Event::id).collect::<Vec<_>>(),
        [
            &WorkflowExecutionEventId::new(id.clone(), NonZeroUsize::new(1).unwrap()),
            &WorkflowExecutionEventId::new(id.clone(), NonZeroUsize::new(2).unwrap()),
        ]
    );
    assert!(events.iter().all(|event| event.aggregate_id() == &id));

    let tail = store.get_events_by_id_since_seq_nr(&id, 2).await.unwrap();
    assert_eq!(tail.iter().map(Event::seq_nr).collect::<Vec<_>>(), [2]);
}
