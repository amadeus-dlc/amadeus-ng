//! `WorkflowExecutionRepository` / `JournalReader` の契約 (BR1.2 / BR1.3 / BR1.4)。
//!
//! ここに書いた関数群が「実装が満たすべき約束」の**唯一の記述**である。in-memory と
//! SQLite は同じ関数を通す (BR2.7 — 片方だけ通るテストを残さない)。
//!
//! 破損 (`MissingSnapshot` / `UndecodablePayload` / `SchemaVersion`) は、行を直接壊す
//! 手段が実装ごとに違い、ポートの面からは作れない。契約テストからは外し、実装固有の
//! テスト (in-memory は `in_memory_event_store.rs` のインラインテスト、SQLite は
//! 直接 SQL) に置く。

use core_domain::orchestration::{
    AutonomyMode, WorkflowExecution, WorkflowExecutionEvent, WorkflowExecutionEventPayload,
};
use core_use_case::orchestration::{
    CorruptCause, EventStoreError, GlobalSeqNr, JournalReader, ProjectionName, RepositoryError,
    WorkflowExecutionRepository,
};

use super::{AT, StoreFixture, absent_intent_id, advanced, genesis, intent_id};

/// 契約テストが使う投影名。
fn projection() -> ProjectionName {
    ProjectionName::parse("state-file").expect("契約テストの投影名は kebab")
}

/// genesis から 5 イベントぶん書き進め、最後の集約 (版を載せ替え済み) を返す。
///
/// 内訳: `Started` → `StageCompleted` → `GateOpened` → `GateApproved` → `AutonomyModeSet`。
async fn seed<R: WorkflowExecutionRepository>(repository: &R) -> WorkflowExecution {
    let (mut aggregate, event) = genesis();
    repository
        .store(&event, &aggregate)
        .await
        .expect("genesis の store は通る");
    aggregate = advanced(aggregate, &event);

    let event = aggregate.complete_stage(AT).expect("索引 0 は非ゲート");
    repository.store(&event, &aggregate).await.expect("store");
    aggregate = advanced(aggregate, &event);

    let event = aggregate
        .open_gate(vec!["intent.md".to_string()], AT)
        .expect("索引 1 はゲート付き");
    repository.store(&event, &aggregate).await.expect("store");
    aggregate = advanced(aggregate, &event);

    let event = aggregate
        .approve_gate(Some("ok".to_string()), None, AT)
        .expect("承認");
    repository.store(&event, &aggregate).await.expect("store");
    aggregate = advanced(aggregate, &event);

    let event = aggregate
        .set_autonomy(AutonomyMode::Autonomous, AT)
        .expect("自律モードの設定");
    repository.store(&event, &aggregate).await.expect("store");
    advanced(aggregate, &event)
}

/// 書いた集約は、同じストアを開き直した別インスタンスから同じ状態で読み直せる (BR1.2)。
pub(crate) async fn round_trip<F: StoreFixture>(fixture: &F) {
    let repository = fixture.open();
    let expected = seed(&repository).await;

    let reopened = fixture.open();
    let found = reopened
        .find_by_id(&intent_id())
        .await
        .expect("書いた集約は読み直せる");

    assert_eq!(found.state(), expected.state(), "16 属性が一致する");
    assert_eq!(found.version(), 5, "版は永続化済みの最後の seq_nr");
    assert_eq!(found.seq_nr(), 5, "順序番号は適用済みイベント数");
}

/// 書いていない集約は `NotFound` (部分データを返さない — C3 ①)。
pub(crate) async fn not_found<F: StoreFixture>(fixture: &F) {
    let repository = fixture.open();
    seed(&repository).await;

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

/// genesis の書込は版 0 を前提にする (スナップショット行の INSERT 経路 — BR2.3)。
pub(crate) async fn genesis_expects_version_zero<F: StoreFixture>(fixture: &F) {
    let repository = fixture.open();
    let (aggregate, event) = genesis();
    assert_eq!(aggregate.version(), 0);
    assert_eq!(event.seq_nr(), 1);
    repository.store(&event, &aggregate).await.expect("genesis");

    let found = repository
        .find_by_id(&intent_id())
        .await
        .expect("読み直せる");
    assert_eq!(found.version(), 1);
}

/// 同じ genesis を 2 度書くと衝突する (UNIQUE(aggregate_id, seq_nr) — BR1.3)。
pub(crate) async fn genesis_twice_conflicts<F: StoreFixture>(fixture: &F) {
    let repository = fixture.open();
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
    let repository = fixture.open();
    seed(&repository).await;

    let mut first = repository.find_by_id(&intent_id()).await.expect("再水和 1");
    let mut second = repository.find_by_id(&intent_id()).await.expect("再水和 2");
    assert_eq!(first.version(), second.version());

    let event = first
        .open_gate(vec!["scope.md".to_string()], AT)
        .expect("索引 2 はゲート付きで in-progress");
    repository
        .store(&event, &first)
        .await
        .expect("先に書いた方は通る");

    let event = second
        .open_gate(vec!["scope.md".to_string()], AT)
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

/// 呼出側の不整合 (`seq_nr` の飛び) は `Corrupt(SequenceGap)` で拒否する (BR1.3)。
pub(crate) async fn sequence_gap_is_refused<F: StoreFixture>(fixture: &F) {
    let repository = fixture.open();
    let (aggregate, event) = genesis();
    repository.store(&event, &aggregate).await.expect("genesis");

    // 版を載せ替えないまま次のイベントを書こうとする (呼出側のバグ)。
    let mut stale = aggregate;
    let next = stale.complete_stage(AT).expect("索引 0 は非ゲート");
    let err = repository
        .store(&next, &stale)
        .await
        .expect_err("版 0 のまま seq_nr 2 は書けない");
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
    let repository = fixture.open();
    let (aggregate, event) = genesis();
    let foreign = WorkflowExecutionEvent::new(
        absent_intent_id(),
        event.seq_nr(),
        AT,
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

/// `events_after(ZERO)` は全イベントを global 昇順で返す (BR1.4)。
pub(crate) async fn journal_reads_every_event_in_global_order<F: StoreFixture>(fixture: &F) {
    let repository = fixture.open();
    seed(&repository).await;

    let reader = fixture.reader();
    let rows = reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("差分読取");
    assert_eq!(rows.len(), 5);
    let globals: Vec<u64> = rows.iter().map(|(global, _)| global.value()).collect();
    let mut sorted = globals.clone();
    sorted.sort_unstable();
    assert_eq!(globals, sorted, "global 通番の昇順");
    let seqs: Vec<u64> = rows.iter().map(|(_, event)| event.seq_nr()).collect();
    assert_eq!(seqs, [1, 2, 3, 4, 5], "欠落なく順に読める");
}

/// `events_after(n)` は差分だけを返す (BR1.4)。
pub(crate) async fn journal_reads_only_the_difference<F: StoreFixture>(fixture: &F) {
    let repository = fixture.open();
    seed(&repository).await;

    let reader = fixture.reader();
    let all = reader.events_after(GlobalSeqNr::ZERO).await.expect("全件");
    let third = all.get(2).expect("3 件目").0;
    let rest = reader.events_after(third).await.expect("差分");
    assert_eq!(rest.len(), 2);
    assert!(rest.iter().all(|(global, _)| *global > third));
}

/// 未登録の投影のチェックポイントは `ZERO` (BR1.4)。
pub(crate) async fn unregistered_checkpoint_is_zero<F: StoreFixture>(fixture: &F) {
    let reader = fixture.reader();
    assert_eq!(
        reader.checkpoint(&projection()).await.expect("読取"),
        GlobalSeqNr::ZERO
    );
}

/// チェックポイントは進められ、同値の再適用は no-op (投影の冪等性の土台 — BR1.4)。
pub(crate) async fn checkpoint_advances_and_repeats_are_noops<F: StoreFixture>(fixture: &F) {
    let repository = fixture.open();
    seed(&repository).await;

    let mut reader = fixture.reader();
    let rows = reader.events_after(GlobalSeqNr::ZERO).await.expect("全件");
    let last = rows.last().expect("5 件ある").0;

    reader
        .advance_checkpoint(&projection(), last)
        .await
        .expect("前進");
    assert_eq!(reader.checkpoint(&projection()).await.expect("読取"), last);

    reader
        .advance_checkpoint(&projection(), last)
        .await
        .expect("同値は no-op");
    assert_eq!(reader.checkpoint(&projection()).await.expect("読取"), last);
    assert!(
        reader.events_after(last).await.expect("差分").is_empty(),
        "追いついた投影に差分は無い"
    );
}

/// チェックポイントの後退は拒否する (単調 — BR1.4)。
pub(crate) async fn checkpoint_regression_is_refused<F: StoreFixture>(fixture: &F) {
    let repository = fixture.open();
    seed(&repository).await;

    let mut reader = fixture.reader();
    reader
        .advance_checkpoint(&projection(), GlobalSeqNr::new(3))
        .await
        .expect("前進");
    let err = reader
        .advance_checkpoint(&projection(), GlobalSeqNr::new(2))
        .await
        .expect_err("後退は拒否");
    assert_eq!(
        err,
        EventStoreError::CheckpointRegression {
            projection: projection(),
            current: GlobalSeqNr::new(3),
            requested: GlobalSeqNr::new(2),
        }
    );
    assert_eq!(
        reader.checkpoint(&projection()).await.expect("読取"),
        GlobalSeqNr::new(3),
        "拒否しても現在値は動かない"
    );
}
