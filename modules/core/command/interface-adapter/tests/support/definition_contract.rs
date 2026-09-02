//! `WorkflowDefinitionRepository` の契約 (BR2.7 の形)。
//!
//! ここに書いた関数群が「実装が満たすべき約束」の**唯一の記述**である。本家の memory
//! バックエンドと SQLite バックエンドは同じ関数を通す (BR2.7 — 片方だけ通るテストを
//! 残さない)。定義の Repository は 2026-08-31 のオーナー裁定でイベントストア形になり、
//! intent / intent-execution の 2 つと**手順が 1 行も違わない**ので、課す約束も同型である。
//!
//! 破損 (`MissingSnapshot` / 復号不能な行など) は、行を直接壊す手段がバックエンドごとに
//! 違い、ポートの面からは作れない。契約テストからは外す。

use core_command_use_case::orchestration::{RepositoryError, WorkflowDefinitionRepository};

use super::{
    DefinitionStoreFixture, absent_definition_id, at, definition_bundle, definition_genesis,
    definition_id, store_definition_genesis,
};

/// `open()` は毎回**空のストア**を指す新しい Repository を返す (BR2.7 — 実装によらない)。
pub(crate) async fn open_twice_yields_independent_empty_stores<F: DefinitionStoreFixture>(
    fixture: &F,
) {
    let mut first = fixture.open();
    store_definition_genesis(&mut first).await;

    let mut second = fixture.open();
    let err = second
        .find_by_id(&definition_id())
        .await
        .expect_err("2 度目の open は空のストアを指す");
    assert!(matches!(
        err,
        RepositoryError::NotFound { id } if id == definition_id()
    ));

    store_definition_genesis(&mut second).await;
    let found = first
        .find_by_id(&definition_id())
        .await
        .expect("読み直せる");
    assert_eq!(
        found.revision(),
        definition_bundle(3).revision(),
        "1 つ目は 2 つ目の書込に影響されない"
    );
}

/// 確立した定義は、同じストアを開き直した別インスタンスから同じ状態で読み直せる。
///
/// 再構成は必ずイベント由来である — スナップショット行 (ある時点の集約) を基底に、差分
/// イベントを畳み込んだ結果が返る。**ファイルは 1 バイトも読まれない**（2026-08-31 の裁定の
/// 本体であり、この契約が守られていることがその証拠である）。
pub(crate) async fn round_trip<F: DefinitionStoreFixture>(fixture: &F) {
    let mut repository = fixture.open();
    let expected = store_definition_genesis(&mut repository).await;

    let reopened = fixture.reopen(&repository);
    let found = reopened
        .find_by_id(&definition_id())
        .await
        .expect("確立した定義は読み直せる");

    assert_eq!(found, expected, "全状態が一致する");
    assert_eq!(
        found.graph(),
        definition_genesis().0.graph(),
        "誕生の材料そのものが再構成される"
    );
    assert_eq!(found.seq_nr(), 1, "誕生の通番は 1");
}

/// 確立していない定義は `NotFound` (部分データを返さない)。
pub(crate) async fn not_found<F: DefinitionStoreFixture>(fixture: &F) {
    let mut repository = fixture.open();
    store_definition_genesis(&mut repository).await;

    let err = repository
        .find_by_id(&absent_definition_id())
        .await
        .expect_err("確立していない定義は読めない");
    assert!(matches!(
        err,
        RepositoryError::NotFound { id } if id == absent_definition_id()
    ));
}

/// 同じ定義を二度確立しようとすると 2 回目は `Conflict` (genesis の重複)。
pub(crate) async fn a_duplicate_genesis_is_a_conflict<F: DefinitionStoreFixture>(fixture: &F) {
    let mut repository = fixture.open();
    store_definition_genesis(&mut repository).await;

    let (aggregate, event) = definition_genesis();
    let err = repository
        .store(&event, &aggregate)
        .await
        .expect_err("重複作成は拒否される");
    assert!(matches!(err, RepositoryError::Conflict { expected: 0, .. }));
}

/// 改訂は差分イベントとして書かれ、読み直すと改訂後の内容が返る。
///
/// 定義の Repository が intent (誕生 1 件だけ) と違う点はここである — 通番が 2 へ進み、
/// 差分再生の経路が実際に踏まれる。
pub(crate) async fn a_redefinition_advances_the_stream<F: DefinitionStoreFixture>(fixture: &F) {
    let mut repository = fixture.open();
    let mut held = store_definition_genesis(&mut repository).await;

    let event = held
        .redefine(&definition_bundle(5), at())
        .expect("内容版が違えば改訂できる");
    repository.store(&event, &held).await.expect("改訂は書ける");

    let reopened = fixture.reopen(&repository);
    let found = reopened
        .find_by_id(&definition_id())
        .await
        .expect("改訂後の定義は読み直せる");
    assert_eq!(found.revision(), definition_bundle(5).revision());
    assert_eq!(found.graph().len(), 5, "内容が入れ替わっている");
    assert_eq!(found.seq_nr(), 2, "改訂は次の通番になる");
    assert_eq!(found.id(), &definition_id(), "系譜 ID は不変");
}

/// 古い版を提示した改訂は `Conflict` (楽観ロック — BR5.3)。
///
/// 版は**読んだ時点のもの**を提示する。別の書き手が先に改訂していれば、その版は古くなって
/// いるので弾かれる。
pub(crate) async fn a_write_that_presents_a_stale_version_conflicts<F: DefinitionStoreFixture>(
    fixture: &F,
) {
    let mut repository = fixture.open();
    let held = store_definition_genesis(&mut repository).await;

    // 同じ版を握った 2 人が順に改訂する。
    let mut first = held.clone();
    let event = first
        .redefine(&definition_bundle(5), at())
        .expect("改訂できる");
    repository
        .store(&event, &first)
        .await
        .expect("先に書いたほうは通る");

    let mut second = held;
    let event = second
        .redefine(&definition_bundle(7), at())
        .expect("改訂そのものは組める");
    let err = repository
        .store(&event, &second)
        .await
        .expect_err("古い版を提示した改訂は弾かれる");
    assert!(
        matches!(
            err,
            RepositoryError::Conflict {
                expected: 1,
                actual: 2
            }
        ),
        "{err:?}"
    );
}
