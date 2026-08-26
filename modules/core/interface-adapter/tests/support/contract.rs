//! `WorkflowExecutionRepository` の契約 (BR1.2 / BR1.3)。
//!
//! ここに書いた関数群が「実装が満たすべき約束」の**唯一の記述**である。本家の memory
//! バックエンドと SQLite バックエンドは同じ関数を通す (BR2.7 — 片方だけ通るテストを残さない)。
//!
//! 破損 (`MissingSnapshot` / `UndecodablePayload`) は、行を直接壊す手段がバックエンドごとに
//! 違い、ポートの面からは作れない。契約テストからは外し、実装固有のテスト
//! (`workflow_execution_repository_impl_test.rs` の生 SQL) に置く。
//!
//! 全集約横断の読取とチェックポイント (`JournalReader`) は SQLite にしか無いので、
//! `journal_reader_impl_test.rs` が単独で持つ。

use core_domain::orchestration::{
    AutonomyMode, WorkflowExecution, WorkflowExecutionEvent, WorkflowExecutionEventPayload,
};
use event_store_adapter_rs::types::{Aggregate, Event};
use std::num::NonZeroUsize;

use core_use_case::orchestration::{CorruptCause, RepositoryError, WorkflowExecutionRepository};

use super::{
    StoreFixture, absent_intent_id, at, genesis, intent_id, store_and_reload, store_genesis,
    store_stage_completed,
};

/// genesis から 5 イベントぶん書き進め、最後の集約 (握り直し済み) を返す。
///
/// 内訳: `Started` → `StageCompleted` → `GateOpened` → `GateApproved` → `AutonomyModeSet`。
pub(crate) async fn seed<R: WorkflowExecutionRepository>(repository: &mut R) -> WorkflowExecution {
    let mut aggregate = store_genesis(repository).await;

    let event = aggregate.complete_stage(at()).expect("索引 0 は非ゲート");
    aggregate = store_and_reload(repository, &event, &aggregate).await;

    let event = aggregate
        .open_gate(vec!["intent.md".to_string()], at())
        .expect("索引 1 はゲート付き");
    aggregate = store_and_reload(repository, &event, &aggregate).await;

    let event = aggregate
        .approve_gate(Some("ok".to_string()), None, at())
        .expect("承認");
    aggregate = store_and_reload(repository, &event, &aggregate).await;

    let event = aggregate
        .switch_autonomy(AutonomyMode::Autonomous, at())
        .expect("自律モードの設定");
    store_and_reload(repository, &event, &aggregate).await
}

/// `open()` は毎回**空のストア**を指す新しい Repository を返す (BR2.7 — 実装によらない)。
///
/// 2 度目の `open()` が 1 度目の書込を見てしまうと、契約テストは「前のテストが書いた行」に
/// 依存しはじめる。空であることに加えて**独立して書ける**ことまで見るのは、単に空なだけの
/// 使い捨てインスタンスと区別するためである。
pub(crate) async fn open_twice_yields_independent_empty_stores<F: StoreFixture>(fixture: &F) {
    let mut first = fixture.open();
    store_genesis(&mut first).await;

    let mut second = fixture.open();
    let err = second
        .find_by_id(&intent_id())
        .await
        .expect_err("2 度目の open は空のストアを指す");
    assert_eq!(
        err,
        RepositoryError::NotFound {
            intent_id: intent_id()
        }
    );

    let found = store_genesis(&mut second).await;
    assert_eq!(found.version(), 1, "2 つ目のストアは独立して書ける");

    let found = first.find_by_id(&intent_id()).await.expect("読み直せる");
    assert_eq!(found.version(), 1, "1 つ目は 2 つ目の書込に影響されない");
}

/// `reopen()` は**開き直した時点までに書き終えた行**を見せる (BR2.7 の共通保証)。
pub(crate) async fn reopen_reflects_the_writes_completed_before_it_was_reopened<F: StoreFixture>(
    fixture: &F,
) {
    let mut repository = fixture.open();
    let aggregate = store_genesis(&mut repository).await;

    let early = fixture.reopen(&repository);
    let found = early.find_by_id(&intent_id()).await.expect("読み直せる");
    assert_eq!(found.version(), 1, "genesis まで書き終えた時点の開き直し");

    store_stage_completed(&mut repository, aggregate).await;

    let late = fixture.reopen(&repository);
    let found = late.find_by_id(&intent_id()).await.expect("読み直せる");
    assert_eq!(found.version(), 2, "2 件目を書き終えた後の開き直し");
}

/// 書いた集約は、同じストアを開き直した別インスタンスから同じ状態で読み直せる (BR1.2)。
pub(crate) async fn round_trip<F: StoreFixture>(fixture: &F) {
    let mut repository = fixture.open();
    let expected = seed(&mut repository).await;

    let reopened = fixture.reopen(&repository);
    let found = reopened
        .find_by_id(&intent_id())
        .await
        .expect("書いた集約は読み直せる");

    assert_eq!(found.state(), expected.state(), "17 属性が一致する");
    assert_eq!(found.version(), 5, "5 回の書込ぶんストアが採番した版");
    assert_eq!(found.seq_nr(), 5, "順序番号は適用済みイベント数");
}

/// 書いていない集約は `NotFound` (部分データを返さない — C3 ①)。
pub(crate) async fn not_found<F: StoreFixture>(fixture: &F) {
    let mut repository = fixture.open();
    seed(&mut repository).await;

    let err = repository
        .find_by_id(&absent_intent_id())
        .await
        .expect_err("未知の集約は NotFound");
    assert_eq!(
        err,
        RepositoryError::NotFound {
            intent_id: absent_intent_id()
        }
    );
}

/// genesis は未永続の集約 (`version` = 0) から書き、ストアが最初の版を採番する (BR5.3)。
pub(crate) async fn the_store_assigns_the_first_version_on_genesis<F: StoreFixture>(fixture: &F) {
    let mut repository = fixture.open();
    let (aggregate, event) = genesis();
    assert_eq!(aggregate.version(), 0, "未永続の集約は 0");
    assert_eq!(event.seq_nr(), 1);
    repository.store(&event, &aggregate).await.expect("genesis");
    assert_eq!(aggregate.version(), 0, "呼出側の集約は動かない (BR1.3)");

    let found = repository
        .find_by_id(&intent_id())
        .await
        .expect("読み直せる");
    assert_eq!(found.version(), 1, "採番したのはストア");
    assert_eq!(found.seq_nr(), 1, "seq_nr はドメインの通番");
}

/// 同じ genesis を 2 度書くと衝突する (スナップショット行の一意性 — BR1.3)。
pub(crate) async fn genesis_twice_conflicts<F: StoreFixture>(fixture: &F) {
    let mut repository = fixture.open();
    let (aggregate, event) = genesis();
    repository.store(&event, &aggregate).await.expect("1 度目");

    let err = repository
        .store(&event, &aggregate)
        .await
        .expect_err("2 度目は衝突");
    assert_eq!(
        err,
        RepositoryError::Conflict {
            expected: 0,
            actual: 1
        }
    );
}

/// 2 つの再水和が同じ版から書くと、後の 1 つが `Conflict` になる (楽観 version — BR1.3)。
pub(crate) async fn concurrent_rehydration_conflicts<F: StoreFixture>(fixture: &F) {
    let mut repository = fixture.open();
    seed(&mut repository).await;

    let mut first = repository.find_by_id(&intent_id()).await.expect("再水和 1");
    let mut second = repository.find_by_id(&intent_id()).await.expect("再水和 2");
    assert_eq!(first.version(), second.version());

    let event = first
        .open_gate(vec!["scope.md".to_string()], at())
        .expect("索引 2 はゲート付きで in-progress");
    repository
        .store(&event, &first)
        .await
        .expect("先に書いた方は通る");

    let event = second
        .open_gate(vec!["scope.md".to_string()], at())
        .expect("同じコマンド");
    let err = repository
        .store(&event, &second)
        .await
        .expect_err("後から書いた方は衝突");
    assert_eq!(
        err,
        RepositoryError::Conflict {
            expected: 5,
            actual: 6
        }
    );

    // 衝突しても状態は変わらない (rollback — NFR3.3)。
    let found = repository
        .find_by_id(&intent_id())
        .await
        .expect("読み直せる");
    assert_eq!(found.version(), 6);
}

/// 古い版のまま書こうとした続きは `Conflict` である (楽観 version はストアの関心 — BR5.3)。
///
/// かつては Repository が `version == seq_nr - 1` を前提検査していたので `Corrupt(SequenceGap)`
/// になっていた。version を `seq_nr` から導く検査は撤回した (オーナー裁定 2026-08-27 (B))
/// ため、この不整合を見つけるのはストアの CAS である。
pub(crate) async fn a_write_from_a_stale_version_conflicts<F: StoreFixture>(fixture: &F) {
    let mut repository = fixture.open();
    let (aggregate, event) = genesis();
    repository.store(&event, &aggregate).await.expect("genesis");

    // 握り直さないまま次のイベントを書こうとする (呼出側のバグ)。
    let mut stale = aggregate;
    let next = stale.complete_stage(at()).expect("索引 0 は非ゲート");
    let err = repository
        .store(&next, &stale)
        .await
        .expect_err("版 0 のままでは書けない");
    assert_eq!(
        err,
        RepositoryError::Conflict {
            expected: 0,
            actual: 1
        }
    );
}

/// イベントの `seq_nr` が適用後の集約と食い違う書込は `Corrupt(SequenceGap)` (BR1.3 の前提検査)。
pub(crate) async fn a_sequence_that_disagrees_with_the_aggregate_is_refused<F: StoreFixture>(
    fixture: &F,
) {
    let mut repository = fixture.open();
    let (aggregate, _) = genesis();
    let skewed = WorkflowExecutionEvent::new(
        intent_id(),
        NonZeroUsize::MIN.saturating_add(aggregate.seq_nr()),
        at(),
        WorkflowExecutionEventPayload::Unparked,
    );
    let err = repository
        .store(&skewed, &aggregate)
        .await
        .expect_err("1 コマンド 1 イベントの通番が合わない");
    assert_eq!(
        err,
        RepositoryError::Corrupt {
            aggregate_id: intent_id(),
            seq_nr: Some(2),
            cause: CorruptCause::SequenceGap,
        }
    );
}

/// イベントと集約の識別子が食い違う書込も `Corrupt(SequenceGap)` (BR1.3 の前提検査)。
pub(crate) async fn mismatched_identity_is_refused<F: StoreFixture>(fixture: &F) {
    let mut repository = fixture.open();
    let (aggregate, event) = genesis();
    let foreign = WorkflowExecutionEvent::new(
        absent_intent_id(),
        NonZeroUsize::new(event.seq_nr()).unwrap(),
        at(),
        WorkflowExecutionEventPayload::Unparked,
    );
    let err = repository
        .store(&foreign, &aggregate)
        .await
        .expect_err("別集約のイベントは書けない");
    assert!(matches!(
        err,
        RepositoryError::Corrupt {
            cause: CorruptCause::SequenceGap,
            ..
        }
    ));
}
