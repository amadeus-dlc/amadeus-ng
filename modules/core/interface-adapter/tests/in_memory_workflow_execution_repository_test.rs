//! `InMemoryWorkflowExecutionRepository` の実装固有の契約 (BR1.2 / BR2.7)。
//!
//! ポートの面から見える約束は `workflow_execution_repository_contract.rs` が SQLite 実装と
//! 共有して検査する。本ファイルが持つのは**スナップショットとジャーナルがずれた状態**の
//! 振る舞い — `store` 経由では作れないので、内包するストアへ `persist_event`
//! (ジャーナルだけの追記) で直接足して作る。SQLite 側の同名テストと 1:1 で対応させ、
//! 「片方だけ通る経路」を残さない (BR2.7)。

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use core_domain::orchestration::{
    StageCompleted, WorkflowExecution, WorkflowExecutionEvent, WorkflowExecutionEventPayload,
};
use core_domain::workflow_definition::StageSlug;
use core_interface_adapter::orchestration::{
    InMemoryEventStore, InMemoryWorkflowExecutionRepository,
};
use event_store_adapter_rs::types::Aggregate;

use core_use_case::orchestration::{
    CorruptCause, EventStore, GlobalSeqNr, JournalReader, RepositoryError,
    WorkflowExecutionRepository,
};

use support::{absent_intent_id, advanced, at, genesis, intent_id};

/// genesis を書いたストアと、書込後に版を載せ替えた集約を返す。
async fn seeded() -> (InMemoryEventStore, WorkflowExecution) {
    let mut store = InMemoryEventStore::new();
    let (aggregate, event) = genesis();
    store
        .persist_event_and_snapshot(&event, &aggregate)
        .await
        .expect("genesis の書込");
    (store, advanced(aggregate, &event))
}

/// スナップショットを genesis に残したまま、ジャーナルにだけ 2 件足す。
///
/// 返すのは「その 2 件を適用し終えた集約」で、`find_by_id` の期待値になる。
async fn journal_ahead_of_the_snapshot(store: &mut InMemoryEventStore) -> WorkflowExecution {
    let (mut aggregate, _) = genesis();
    let second = aggregate.complete_stage(at()).expect("索引 0 は非ゲート");
    store
        .persist_event(&second, 1)
        .await
        .expect("ジャーナルだけに追記");
    let third = aggregate
        .open_gate(vec!["intent.md".to_string()], at())
        .expect("索引 1 はゲート付き");
    store
        .persist_event(&third, 1)
        .await
        .expect("スナップショットは動かないので版は 1 のまま");
    advanced(aggregate, &third)
}

// ---------------------------------------------------------------------------
// 再構成 (BR1.2)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_store_without_any_row_reports_not_found() {
    let repository = InMemoryWorkflowExecutionRepository::default();
    let err = repository
        .find_by_id(&absent_intent_id())
        .await
        .expect_err("書いていない集約");
    assert_eq!(
        err,
        RepositoryError::NotFound {
            intent_id: absent_intent_id()
        }
    );
}

#[tokio::test]
async fn a_journal_without_a_snapshot_is_corrupt_not_missing() {
    // `persist_event` は写しを作らないので、ジャーナル行だけが残る。「まだ無い」と
    // 「壊れている」を分けるのはこの 1 点だけである (BR1.2)。
    let mut store = InMemoryEventStore::new();
    let (_, event) = genesis();
    store.persist_event(&event, 0).await.expect("追記");
    let repository = InMemoryWorkflowExecutionRepository::new(store);

    let err = repository
        .find_by_id(&intent_id())
        .await
        .expect_err("ジャーナルはあるのに写しが無い");
    assert_eq!(
        err,
        RepositoryError::Corrupt {
            aggregate_id: intent_id(),
            seq_nr: None,
            cause: CorruptCause::MissingSnapshot,
        }
    );
}

#[tokio::test]
async fn the_version_after_a_replay_is_the_sequence_of_the_last_applied_event() {
    let (mut store, _) = seeded().await;
    let expected = journal_ahead_of_the_snapshot(&mut store).await;
    let repository = InMemoryWorkflowExecutionRepository::new(store);

    let found = repository.find_by_id(&intent_id()).await.expect("読める");
    assert_eq!(found.version(), 3, "replay 後に最後の seq_nr を載せる");
    assert_eq!(found.seq_nr(), 3);
    assert_eq!(found.state(), expected.state(), "17 属性が一致する");
}

#[tokio::test]
async fn a_gap_in_the_replayed_journal_is_corrupt() {
    let (mut store, _) = seeded().await;
    let (mut aggregate, _) = genesis();
    let _skipped = aggregate.complete_stage(at()).expect("索引 0 は非ゲート");
    let third = aggregate
        .open_gate(vec!["intent.md".to_string()], at())
        .expect("索引 1 はゲート付き");
    // seq_nr 2 を飛ばして 3 だけを足す。
    store.persist_event(&third, 1).await.expect("追記");
    let repository = InMemoryWorkflowExecutionRepository::new(store);

    let err = repository
        .find_by_id(&intent_id())
        .await
        .expect_err("seq_nr が飛ぶ");
    assert_eq!(
        err,
        RepositoryError::Corrupt {
            aggregate_id: intent_id(),
            seq_nr: Some(3),
            cause: CorruptCause::SequenceGap,
        }
    );
}

#[tokio::test]
async fn a_replayed_event_naming_a_stage_outside_the_plan_is_corrupt() {
    // 復号はできるが、解決済み計画に無いステージを名指すイベント。`apply_event` の
    // `UnknownStage` は `Corrupt(InvariantViolation)` へ写す (SequenceGap とは別の原因)。
    let (mut store, _) = seeded().await;
    let bogus = WorkflowExecutionEvent::new(
        intent_id(),
        2,
        at(),
        WorkflowExecutionEventPayload::StageCompleted(StageCompleted::new(
            StageSlug::parse("no-such-stage").unwrap(),
            None,
        )),
    );
    store.persist_event(&bogus, 1).await.expect("追記");
    let repository = InMemoryWorkflowExecutionRepository::new(store);

    let err = repository
        .find_by_id(&intent_id())
        .await
        .expect_err("計画に無いステージ");
    assert_eq!(
        err,
        RepositoryError::Corrupt {
            aggregate_id: intent_id(),
            seq_nr: Some(2),
            cause: CorruptCause::InvariantViolation,
        }
    );
}

// ---------------------------------------------------------------------------
// 内包するストアへのハンドル (BR1.4)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_repository_hands_out_a_reader_over_the_same_store() {
    let (store, _) = seeded().await;
    let mut repository = InMemoryWorkflowExecutionRepository::new(store);

    let rows = repository
        .event_store()
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("読める");
    assert_eq!(
        rows.iter().map(|(g, _)| g.to_u64()).collect::<Vec<_>>(),
        [1],
        "genesis の 1 件が見える"
    );

    // Repository 越しに書いたものが同じストアから見えること — 内包しているストアは
    // 1 つだけで、読取の口 (`event_store`) はその参照を返す (別ハンドルは配らない)。
    let mut aggregate = repository.find_by_id(&intent_id()).await.expect("読める");
    let next = aggregate.complete_stage(at()).expect("索引 0 は非ゲート");
    repository.store(&next, &aggregate).await.expect("2 件目");
    let rows = repository
        .event_store()
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("読める");
    assert_eq!(rows.len(), 2);
}
