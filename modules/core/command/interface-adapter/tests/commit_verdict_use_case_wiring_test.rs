//! `ReportUseCase` が**実物の Repository 実装**と組めることを示す結線テスト (契約 C3 ④)。
//!
//! # なぜこれが 1 本だけなのか
//!
//! C3 ④（ADR-010 改訂）は「テストダブル型は無く、テストは
//! `XxxUseCase<WorkflowExecutionRepositoryImpl<…>>` で組む」と定める。一方
//! `coding-rules/use-case-rules.md` §1 の機械強制は「`core-command-use-case` の `Cargo.toml` に
//! `core-command-interface-adapter` が無いこと」であり、ユースケース側のテストから実物の実装は
//! 触れない（触れば依存が循環する）。
//!
//! そこで**結線だけを実物で示す場所をこちら側に置く**。合成ルート（U7）が実際に書く形
//! — 実物の `WorkflowExecutionRepositoryImpl` を `ReportUseCase` に注入して 1 遷移を
//! コミットする — が型として成立し、行が本当にストアへ載ることを固定する。経路の網羅・
//! 異常系・`Conflict` の再試行は use-case クレート内の fake テストが持つ（`Conflict` を
//! 意図的に起こすには、どのみち応答をスクリプトできるダブルが要る）。

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
// panic! は「想定した変種でなければ即失敗」という検証用途である。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use core_command_domain::orchestration::WorkflowExecutionEvent;
use core_command_domain::workspace::CheckboxState;
use core_command_interface_adapter::orchestration::WorkflowExecutionRepositoryImpl;
use core_command_use_case::orchestration::{
    ReportOutcome, ReportUseCase, ReportedTransition, ReportedVerdict, WorkflowExecutionRepository,
};

use support::{at, intent_id, store_genesis};

#[tokio::test]
async fn the_use_case_commits_a_transition_through_the_real_repository() {
    // 合成ルート（U7）が書くのと同じ結線 — ポートの実装を注入するだけで、ユースケースは
    // 実装の型を知らない（静的束縛。`dyn` は使わない）。
    let mut repository = WorkflowExecutionRepositoryImpl::in_memory();
    let held = store_genesis(&mut repository).await;
    assert_eq!(
        held.aggregate().checkbox(held.aggregate().cursor()),
        Some(CheckboxState::InProgress),
        "genesis のカーソルは initialization（非ゲート）"
    );
    // 同じストアを指す別の口。ユースケースが書いた行を外から観測するために先に取っておく。
    let observer = repository.reopened();

    let mut use_case = ReportUseCase::new(repository);
    let outcome = use_case
        .execute(
            &intent_id(),
            None,
            ReportedVerdict::Transition(ReportedTransition::Forward { user_input: None }),
            at(),
        )
        .await
        .expect("非ゲートのカーソルは完了できる");

    let ReportOutcome::Committed { event } = outcome else {
        panic!("コミットを期待した: {outcome:?}");
    };
    let WorkflowExecutionEvent::StageCompleted(completed) = &event else {
        panic!("StageCompleted を期待した: {event:?}");
    };
    assert_eq!(completed.stage().as_str(), "state-init");
    assert_eq!(
        completed
            .next_stage()
            .map(core_command_domain::workflow_definition::StageSlug::as_str),
        Some("intent-capture")
    );

    // 実物のストアに載ったことを、別の口から再構成して確かめる。
    let after = observer
        .find_by_id(&intent_id())
        .await
        .expect("書いた集約は握り直せる");
    assert_eq!(after.aggregate().seq_nr(), 2, "genesis に 1 件積んだ");
    assert_eq!(after.version(), 2, "版を採番したのはストアである");
    assert_eq!(
        after
            .aggregate()
            .checkbox(after.aggregate().cursor())
            .expect("カーソルは範囲内"),
        CheckboxState::InProgress,
        "カーソルは次のステージへ進み、そのステージは着手済みになる"
    );
}
