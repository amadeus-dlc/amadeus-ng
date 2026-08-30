//! `IntentRepository` の契約 (issue #50 — BR2.7 の形)。
//!
//! ここに書いた関数群が「実装が満たすべき約束」の**唯一の記述**である。本家の memory
//! バックエンドと SQLite バックエンドは同じ関数を通す (BR2.7 — 片方だけ通るテストを
//! 残さない)。
//!
//! 破損 (`MissingSnapshot` / 復号不能な行など) は、行を直接壊す手段がバックエンドごとに
//! 違い、ポートの面からは作れない。契約テストからは外し、実装固有のテスト
//! (`intent_repository_impl_test.rs` の生 SQL) に置く。

use core_command_use_case::orchestration::{IntentRepository, RepositoryError};

use super::{
    IntentStoreFixture, absent_intent_id, at, intent_genesis, intent_id, store_intent_genesis,
};

/// `open()` は毎回**空のストア**を指す新しい Repository を返す (BR2.7 — 実装によらない)。
pub(crate) async fn open_twice_yields_independent_empty_stores<F: IntentStoreFixture>(fixture: &F) {
    let mut first = fixture.open();
    store_intent_genesis(&mut first).await;

    let mut second = fixture.open();
    let err = second
        .find_by_id(&intent_id())
        .await
        .expect_err("2 度目の open は空のストアを指す");
    assert!(matches!(
        err,
        RepositoryError::NotFound { id } if id == intent_id()
    ));

    let found = store_intent_genesis(&mut second).await;
    assert_eq!(found, intent_genesis().0, "2 つ目のストアは独立して書ける");

    let found = first.find_by_id(&intent_id()).await.expect("読み直せる");
    assert_eq!(
        found,
        intent_genesis().0,
        "1 つ目は 2 つ目の書込に影響されない"
    );
}

/// 書いた intent は、同じストアを開き直した別インスタンスから同じ状態で読み直せる。
///
/// 再構成は必ずイベント由来である (オーナー裁定 2026-08-30 — コマンド側の読取規律):
/// スナップショット行 (ある時点の集約) を基底に、差分イベントを [`Intent::replay`] で
/// 畳み込んだ結果が返る。
///
/// [`Intent::replay`]: core_command_domain::orchestration::Intent::replay
pub(crate) async fn round_trip<F: IntentStoreFixture>(fixture: &F) {
    let mut repository = fixture.open();
    let expected = store_intent_genesis(&mut repository).await;

    let reopened = fixture.reopen(&repository);
    let found = reopened
        .find_by_id(&intent_id())
        .await
        .expect("書いた intent は読み直せる");

    assert_eq!(found, expected, "全状態が一致する");
    assert_eq!(
        found,
        intent_genesis().0,
        "誕生の材料そのものが再構成される"
    );
}

/// 書いていない intent は `NotFound` (部分データを返さない)。
pub(crate) async fn not_found<F: IntentStoreFixture>(fixture: &F) {
    let mut repository = fixture.open();
    store_intent_genesis(&mut repository).await;

    let err = repository
        .find_by_id(&absent_intent_id())
        .await
        .expect_err("未知の intent は NotFound");
    assert!(matches!(
        err,
        RepositoryError::NotFound { id } if id == absent_intent_id()
    ));
}

/// 同じ intent の genesis を 2 度書くことはできない (`Conflict`)。
///
/// 実装は現行スロット行の一意性 (本家の作成規約) が拒み、in-memory ダブルは保持写像の
/// キー重複が拒む — どちらもポート面では同じ `Conflict` である。
pub(crate) async fn a_duplicate_genesis_is_a_conflict<F: IntentStoreFixture>(fixture: &F) {
    let mut repository = fixture.open();
    store_intent_genesis(&mut repository).await;

    let (aggregate, event) = intent_genesis();
    let err = repository
        .store(&event, &aggregate, at())
        .await
        .expect_err("重複作成は拒否");
    assert!(matches!(err, RepositoryError::Conflict { expected: 0, .. }));

    let found = repository.find_by_id(&intent_id()).await.expect("読める");
    assert_eq!(found, aggregate, "拒否は保持中の intent を壊さない");
}
