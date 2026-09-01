//! `IntentExecutionRepository` の契約 (BR1.2 / BR1.3)。
//!
//! ここに書いた関数群が「実装が満たすべき約束」の**唯一の記述**である。本家の memory
//! バックエンドと SQLite バックエンドは同じ関数を通す (BR2.7 — 片方だけ通るテストを残さない)。
//!
//! 破損 (`MissingSnapshot` / `UndecodablePayload`) は、行を直接壊す手段がバックエンドごとに
//! 違い、ポートの面からは作れない。契約テストからは外し、実装固有のテスト
//! (`intent_repository_impl_test.rs` の生 SQL) に置く。
//!
//! 全集約横断の読取とチェックポイント (`JournalReader`) は SQLite にしか無いので、
//! `journal_reader_impl_test.rs` が単独で持つ。

use core_command_domain::orchestration::{AutonomyMode, IntentExecution};

use core_command_use_case::orchestration::{IntentExecutionRepository, RepositoryError};

use super::{
    StoreFixture, absent_execution_id, advance, at, execution_id, genesis, intent,
    store_gate_opened, store_genesis,
};

/// genesis から 4 イベントぶん書き進め、最後の再水和結果を返す。
///
/// 内訳: `Started` → `GateOpened` → `GateApproved` → `AutonomyModeSet`。
///
/// 誕生 = 初期化完了済み (issue #76) により、かつて先頭にあった `StageCompleted`
/// (索引 0 = 非ゲートの initialization を完了させる 1 件) は**構成不能**になった —
/// 誕生の時点でその checkbox は既に completed で、カーソルは索引 1 のゲート付き
/// ステージに立っている。前置きが 1 件消えたぶん、以後の通番と版が 1 つずつ詰まる。
pub(crate) async fn seed<R: IntentExecutionRepository>(repository: &mut R) -> IntentExecution {
    let mut held = store_genesis(repository).await;
    held = advance(repository, &held, |aggregate| {
        aggregate.open_gate(&intent(), vec!["intent.md".to_string()], at())
    })
    .await;
    held = advance(repository, &held, |aggregate| {
        aggregate.approve_gate(&intent(), Some("ok".to_string()), at())
    })
    .await;
    advance(repository, &held, |aggregate| {
        aggregate.switch_autonomy(&intent(), AutonomyMode::Autonomous, at())
    })
    .await
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
        .find_by_id(&execution_id())
        .await
        .expect_err("2 度目の open は空のストアを指す");
    assert!(matches!(
        err,
        RepositoryError::NotFound { id } if id == execution_id()
    ));

    let found = store_genesis(&mut second).await;
    assert_eq!(found.version(), 1, "2 つ目のストアは独立して書ける");

    let found = first.find_by_id(&execution_id()).await.expect("読み直せる");
    assert_eq!(found.version(), 1, "1 つ目は 2 つ目の書込に影響されない");
}

/// `reopen()` は**開き直した時点までに書き終えた行**を見せる (BR2.7 の共通保証)。
pub(crate) async fn reopen_reflects_the_writes_completed_before_it_was_reopened<F: StoreFixture>(
    fixture: &F,
) {
    let mut repository = fixture.open();
    let held = store_genesis(&mut repository).await;

    let early = fixture.reopen(&repository);
    let found = early.find_by_id(&execution_id()).await.expect("読み直せる");
    assert_eq!(found.version(), 1, "genesis まで書き終えた時点の開き直し");

    store_gate_opened(&mut repository, &held).await;

    let late = fixture.reopen(&repository);
    let found = late.find_by_id(&execution_id()).await.expect("読み直せる");
    assert_eq!(found.version(), 2, "2 件目を書き終えた後の開き直し");
}

/// 書いた集約は、同じストアを開き直した別インスタンスから同じ状態で読み直せる (BR1.2)。
pub(crate) async fn round_trip<F: StoreFixture>(fixture: &F) {
    let mut repository = fixture.open();
    let expected = seed(&mut repository).await;

    let reopened = fixture.reopen(&repository);
    let found = reopened
        .find_by_id(&execution_id())
        .await
        .expect("書いた集約は読み直せる");

    assert_eq!(found, expected, "全状態が一致する");
    assert_eq!(found.version(), 4, "4 回の書込ぶんストアが採番した版");
    assert_eq!(found.seq_nr(), 4, "順序番号は適用済みイベント数");
}

/// 書いていない集約は `NotFound` (部分データを返さない — C3 ①)。
pub(crate) async fn not_found<F: StoreFixture>(fixture: &F) {
    let mut repository = fixture.open();
    seed(&mut repository).await;

    let err = repository
        .find_by_id(&absent_execution_id())
        .await
        .expect_err("未知の集約は NotFound");
    assert!(matches!(
        err,
        RepositoryError::NotFound { id } if id == absent_execution_id()
    ));
}

/// genesis は未永続の版 (`UNPERSISTED_VERSION`) から書き、ストアが最初の版を採番する (BR5.3)。
pub(crate) async fn the_store_assigns_the_first_version_on_genesis<F: StoreFixture>(fixture: &F) {
    let mut repository = fixture.open();
    let (aggregate, event) = genesis();
    assert_eq!(aggregate.seq_nr(), 1, "genesis の通番は 1");
    assert_eq!(
        aggregate.version(),
        IntentExecution::UNPERSISTED_VERSION,
        "genesis はまだ版を持たない"
    );
    repository.store(&event, &aggregate).await.expect("genesis");

    let found = repository
        .find_by_id(&execution_id())
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

    // 手元の集約は未永続の版のままなので、同じものをもう一度書けば競合になる。
    let err = repository
        .store(&event, &aggregate)
        .await
        .expect_err("2 度目は衝突");
    assert!(matches!(
        err,
        RepositoryError::Conflict {
            expected: 0,
            actual: 1,
        }
    ));
}

/// 2 つの再水和が同じ版から書くと、後の 1 つが `Conflict` になる (楽観 version — BR1.3)。
pub(crate) async fn concurrent_rehydration_conflicts<F: StoreFixture>(fixture: &F) {
    let mut repository = fixture.open();
    seed(&mut repository).await;

    let first = repository
        .find_by_id(&execution_id())
        .await
        .expect("再水和 1");
    let second = repository
        .find_by_id(&execution_id())
        .await
        .expect("再水和 2");
    assert_eq!(first.version(), second.version());

    let mut aggregate = first.clone();
    let event = aggregate
        .open_gate(&intent(), vec!["scope.md".to_string()], at())
        .expect("索引 2 はゲート付きで in-progress");
    repository
        .store(&event, &aggregate)
        .await
        .expect("先に書いた方は通る");

    let mut aggregate = second.clone();
    let event = aggregate
        .open_gate(&intent(), vec!["scope.md".to_string()], at())
        .expect("同じコマンド");
    let err = repository
        .store(&event, &aggregate)
        .await
        .expect_err("後から書いた方は衝突");
    assert!(matches!(
        err,
        RepositoryError::Conflict {
            expected: 4,
            actual: 5,
        }
    ));

    // 衝突しても状態は変わらない (rollback — NFR3.3)。
    let found = repository
        .find_by_id(&execution_id())
        .await
        .expect("読み直せる");
    assert_eq!(found.version(), 5);
}

/// 古い版を提示した書込は `Conflict` である (楽観 version はストアの関心 — BR5.3)。
///
/// version は**再水和した集約が握る** (ADR-010 / B7、オーナー裁定 2026-08-30)。握った後に
/// 他者が書けば、提示する版が古くなってそこで止まる。
pub(crate) async fn a_write_from_a_stale_version_conflicts<F: StoreFixture>(fixture: &F) {
    let mut repository = fixture.open();
    let stale = store_genesis(&mut repository).await;

    // 別の書き手が 1 件進めて版が動く。
    store_gate_opened(&mut repository, &stale).await;

    // 握ったままの版 (genesis 時点) で次を書こうとする。
    let mut aggregate = stale.clone();
    let next = aggregate
        .open_gate(&intent(), vec!["intent.md".to_string()], at())
        .expect("誕生のカーソルは索引 1 のゲート付きステージで in-progress");
    let err = repository
        .store(&next, &aggregate)
        .await
        .expect_err("古い版では書けない");
    assert!(matches!(
        err,
        RepositoryError::Conflict {
            expected: 1,
            actual: 2,
        }
    ));
}

/// 版を握り直せば続きが書ける (`Conflict` の裏面 — 再試行の政策はユースケースが持つ)。
pub(crate) async fn a_write_from_the_rehydrated_version_succeeds<F: StoreFixture>(fixture: &F) {
    let mut repository = fixture.open();
    let held = store_genesis(&mut repository).await;
    let next = store_gate_opened(&mut repository, &held).await;
    assert_eq!(next.version(), 2);
    assert_eq!(next.seq_nr(), 2);
}

/// 未永続でない版を genesis に提示すると、本家の書込契約に反するので `Corrupt` になる。
///
/// v3 は「`seq_nr == 1` ⇔ `expected_version == 0`」を要求し、崩れた呼出しを
/// `ContractViolation` で拒否する。ストレージ障害ではなく呼出側の組み立て違反なので、
/// 競合ではなく破損として写す (BR1.5)。
pub(crate) async fn a_genesis_with_a_non_zero_version_is_a_contract_violation<F: StoreFixture>(
    fixture: &F,
) {
    let mut repository = fixture.open();
    let (aggregate, event) = genesis();
    let aggregate = aggregate.with_version(1);
    let err = repository
        .store(&event, &aggregate)
        .await
        .expect_err("seq_nr = 1 に版 1 は対応しない");
    assert!(
        matches!(err, RepositoryError::Corrupt { .. }),
        "実際: {err:?}"
    );

    // 拒否された書込は行を残さない。
    let found = repository.find_by_id(&execution_id()).await;
    assert!(matches!(
        found.expect_err("書かれていない"),
        RepositoryError::NotFound { id } if id == execution_id()
    ));
}
