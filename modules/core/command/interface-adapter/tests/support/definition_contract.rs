//! `WorkflowDefinitionRepository` の契約 (BR2.7 の形)。
//!
//! ここに書いた関数群が「実装が満たすべき約束」の**唯一の記述**である。本家の memory
//! バックエンドと SQLite バックエンドは同じ関数を通す (BR2.7 — 片方だけ通るテストを
//! 残さない)。定義の Repository は 2026-08-31 のオーナー裁定でイベントストア形になり、
//! intent / intent-execution の 2 つと**手順が 1 行も違わない**ので、課す約束も同型である。
//!
//! 破損 (`MissingSnapshot` / 復号不能な行など) は、行を直接壊す手段がバックエンドごとに
//! 違い、ポートの面からは作れない。契約テストからは外す。

use core_command_domain::orchestration::{Created, Intent, StartRequest};
use core_command_domain::workflow_definition::{
    CompiledDefinition, CompiledDefinitionId, WorkflowDefinition,
};
use core_command_use_case::orchestration::{RepositoryError, WorkflowDefinitionRepository};

use super::{
    DefinitionStoreFixture, absent_definition_id, at, definition_bundle, definition_content,
    definition_genesis, definition_id, intent, intent_event_id, intent_id, scan, stages,
    store_definition_genesis,
};

/// intent の参照先を再構成し、intent 作成後の定義の改訂も反映する。
pub(crate) async fn find_for_intent_returns_the_current_definition<F: DefinitionStoreFixture>(
    fixture: &F,
) {
    let mut repository = fixture.open();
    let (graph, grid, scopes) = definition_content(7);
    let other_bundle = CompiledDefinition::compile(
        CompiledDefinitionId::parse("kiro").expect("別の系譜"),
        graph,
        grid,
        scopes,
    )
    .0;
    let (other, created) = WorkflowDefinition::define(absent_definition_id(), &other_bundle, at())
        .expect("別の定義を確立する");
    repository
        .store(&created, &other)
        .await
        .expect("別の定義も保存する");
    let mut expected = store_definition_genesis(&mut repository).await;
    let event = expected
        .redefine(&definition_bundle(5), at())
        .expect("改訂する");
    repository
        .store(&event, &expected)
        .await
        .expect("改訂を保存する");

    let found = fixture
        .reopen(&repository)
        .find_for_intent(&intent())
        .await
        .expect("intent の参照先を再構成する");

    assert_eq!(found.id(), &definition_id());
    assert_eq!(found.revision(), definition_bundle(5).revision());
    assert_eq!(found.seq_nr(), 2);
}

/// 関連先が保存されていなければ、その定義の ID を伴う `NotFound` を返す。
pub(crate) async fn find_for_intent_reports_the_missing_definition<F: DefinitionStoreFixture>(
    fixture: &F,
) {
    let mut repository = fixture.open();
    store_definition_genesis(&mut repository).await;
    let intent = Intent::from((
        Created::new(
            intent_event_id(),
            intent_id(),
            absent_definition_id(),
            definition_bundle(3).revision().clone(),
            StartRequest::new("classic", "contract"),
            stages(),
            scan(),
        ),
        at(),
    ));

    let error = repository
        .find_for_intent(&intent)
        .await
        .expect_err("参照先は未保存");

    assert!(matches!(error, RepositoryError::NotFound { id } if id == absent_definition_id()));
}

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

/// 別系譜のイベントと集約を組み合わせても、既存の定義を変更しない。
pub(crate) async fn an_event_from_another_definition_is_rejected_before_writing<
    F: DefinitionStoreFixture,
>(
    fixture: &F,
) {
    use core_command_domain::workflow_definition::{
        CompiledDefinition, CompiledDefinitionId, WorkflowDefinition,
    };
    let other_bundle = |count| {
        let (graph, grid, scopes) = super::definition_content(count);
        CompiledDefinition::compile(
            CompiledDefinitionId::parse("kiro").unwrap(),
            graph,
            grid,
            scopes,
        )
        .0
    };
    let mut repository = fixture.open();
    let (aggregate, _) = definition_genesis();
    let (mut other, foreign) =
        WorkflowDefinition::define(absent_definition_id(), &other_bundle(3), at()).unwrap();
    let error = repository
        .store(&foreign, &aggregate)
        .await
        .expect_err("別系譜のイベントを拒否");
    assert!(
        matches!(error, RepositoryError::Corrupt { .. }),
        "{error:?}"
    );
    for id in [definition_id(), absent_definition_id()] {
        assert!(matches!(
            repository.find_by_id(&id).await,
            Err(RepositoryError::NotFound { .. })
        ));
    }
    let held = store_definition_genesis(&mut repository).await;
    let mut candidate = held.clone();
    candidate.redefine(&definition_bundle(5), at()).unwrap();
    let foreign = other.redefine(&other_bundle(5), at()).unwrap();
    assert!(matches!(
        repository.store(&foreign, &candidate).await,
        Err(RepositoryError::Corrupt { .. })
    ));
    assert_eq!(repository.find_by_id(&definition_id()).await.unwrap(), held);
}
