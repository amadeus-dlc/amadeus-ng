//! `WorkflowExecutionRepository` ポート — 集約 `WorkflowExecution` の ES 形 Repository (C3 / ADR-010)。

use core_command_domain::orchestration::{IntentId, WorkflowExecution, WorkflowExecutionEvent};

use super::rehydrated_workflow_execution::RehydratedWorkflowExecution;
use super::repository_error::RepositoryError;

/// 集約 `WorkflowExecution` の Repository (イベントソーシング形 — ADR-010 / C3)。
///
/// 動詞は本家ライブラリ (event-store-adapter-rs) の語彙に従い `store` / `find_by_id`。
/// ステートソーシング Repository の `save` は持たない
/// (`coding-rules/gateway-taxonomy.md` §2b の ES 拡張語彙)。
///
/// トランザクションの所有は**実装側**であり、ユースケースは Tx を持たない (C3 ②)。
/// `Conflict` 以外は再試行しない — 再試行の政策はユースケースの責務である (C3 ③)。
///
/// レシーバは CQS に従う (`coding-rules/command-query-separation.md`) — 再構成 (Query) は
/// `&self`、永続化 (Command) は `&mut self` である。可変操作を `&self` に見せて内部可変性
/// (`RefCell` 等) で隠すのは「`&self` への偽装」であり禁止されている
/// (`coding-rules/interior-mutability.md`)。したがって実装は 1 つのストアを単一所有し、
/// 書込中の排他は借用チェッカが保証する。
///
/// 実装は `core-interface-adapter` の `orchestration::WorkflowExecutionRepositoryImpl`
/// 1 つで、内包するイベントストア (本家 event-store-adapter-rs のバックエンド) だけが
/// 違う — SQLite ならファイル、memory なら揮発である。
///
/// # 楽観 version はポートを往復する (ADR-010 / B7)
///
/// 集約は楽観 version を持たない (正本はスナップショット行の列)。代わりに
/// [`find_by_id`](WorkflowExecutionRepository::find_by_id) が読んだ版を
/// [`RehydratedWorkflowExecution`] に載せて返し、呼出側がそれを
/// [`store`](WorkflowExecutionRepository::store) へ提示する。**読んだ時点の版で書く**こと
/// そのものが楽観ロックであり、実装が書込直前に版を読み直すと成立しなくなる。
///
/// この版は `usize` で運ぶが、**数ではなく不透明なトークンである**。守るべきことは 3 つ:
///
/// - **ストアが採番したトークンである。** 解釈・比較・算術をしない。読んだ値をそのまま返すだけ
///   である (BR5.3)
/// - **`seq_nr` と混同するな。** 集約の順序番号はドメインが採番する別物であり、値がたまたま
///   一致することがあっても意味は違う (オーナー裁定「seq_nr と version を混ぜない」)
/// - **集約へ入れるな。** 版を集約に載せた瞬間、ストアの採番規則がドメインへ戻る。version が
///   通ってよいのはこのポートの戻り値と引数だけである
///
/// どちらも `usize` なので**型では取り違えを止められない**。それでも newtype で衣を着せる案は
/// **却下済み**である (オーナー裁定 2026-08-29) — 楽観 version は本家 event-store-adapter-rs
/// v3.0.0 が `expected_version: usize` で定めた語彙であり、こちらの都合で包み直すのは
/// Conformist 方針 (`=3.0.0` ピン・腐敗防止層なし) への違反にあたる
/// (`coding-rules/upstream-contracts.md` — 借り物の契約を自分のドメインに合わせて曲げない)。
#[allow(
    async_fn_in_trait,
    reason = "Send 境界を意図的に要求しない設計 (C3 / Q3 = A — tokio current_thread)。\
              自動 trait 境界を書けないという注意喚起は本 trait では設計どおりである。"
)]
pub trait WorkflowExecutionRepository {
    /// 集約を**完全に**再構成して返す (部分データを返さない — C3 ①)。
    ///
    /// 最新スナップショットを復元し、その `seq_nr` より後のイベントを昇順に適用して返す
    /// (BR1.2)。同時に**ストアが載せていた楽観 version** を返す — 不透明なトークンであり、
    /// `seq_nr` から導かない (BR5.3)。
    ///
    /// # Errors
    ///
    /// 集約が無い (`NotFound`)、ストア I/O (`Io`)、スナップショット欠落・復号不能・
    /// 不変条件違反・`seq_nr` の不連続 (`Corrupt`) を返す。
    async fn find_by_id(
        &self,
        id: &IntentId,
    ) -> Result<RehydratedWorkflowExecution, RepositoryError>;

    /// 1 コマンドが返した単一イベントと適用後の集約を、同一トランザクションで永続化する。
    ///
    /// 輸送のメタデータ (集約識別子・通番・発生時刻・型判別子) は**実装が封筒に組む** —
    /// 通番と発生時刻は適用後の集約が持っているので、引数で二重に受け取らない (BR1.3)。
    ///
    /// `expected_version` は再構成時に受け取った版で、新規作成 (`Started`) では
    /// [`WorkflowExecutionRepository::UNPERSISTED_VERSION`] である。一致しなければ `Conflict`
    /// で、ストアの状態は変わらない (BR1.3)。引数は `&` なので呼出側の集約は変更されない。
    ///
    /// この引数は**ストア採番の不透明トークン**である — `seq_nr` と混同してはならず、集約へ
    /// 入れてもならない (trait doc の「楽観 version はポートを往復する」を参照)。渡すのは
    /// [`RehydratedWorkflowExecution::version`] が返した値そのものであり、`aggregate.seq_nr()`
    /// から導いてはならない。
    ///
    /// # Errors
    ///
    /// 楽観 version の不一致 (`Conflict`)、ストア I/O (`Io`)、符号化の失敗 (`Corrupt`) を返す。
    async fn store(
        &mut self,
        event: &WorkflowExecutionEvent,
        aggregate: &WorkflowExecution,
        expected_version: usize,
    ) -> Result<(), RepositoryError>;

    /// まだ 1 度も永続化していない集約が提示する版 (新規作成の `expected_version`)。
    ///
    /// 本家 v3 の規約「新規作成は `seq_nr == 1` かつ `expected_version == 0`」の 0 に名前を
    /// 与えたものである — 呼出側に裸の `0` を書かせない。
    const UNPERSISTED_VERSION: usize = 0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::RepositoryError;
    use crate::orchestration::test_support::{
        InMemoryWorkflowExecutionRepository, absent_intent, genesis, intent,
    };
    use core_command_domain::orchestration::IntentId;

    /// ジェネリック関数からポート越しに使えること (静的束縛 — ユースケースはこの形で組む)。
    async fn rehydrate<R: WorkflowExecutionRepository>(
        repository: &R,
        id: &IntentId,
    ) -> Result<RehydratedWorkflowExecution, RepositoryError> {
        repository.find_by_id(id).await
    }

    #[tokio::test]
    async fn an_unknown_aggregate_is_not_found() {
        let repository = InMemoryWorkflowExecutionRepository::empty();
        let err = rehydrate(&repository, &absent_intent()).await.unwrap_err();
        assert_eq!(
            err,
            RepositoryError::NotFound {
                intent_id: absent_intent()
            }
        );
    }

    #[tokio::test]
    async fn a_stored_aggregate_is_rehydrated_by_its_identifier() {
        let mut repository = InMemoryWorkflowExecutionRepository::empty();
        let (aggregate, event) = genesis(1);
        repository
            .store(
                &event,
                &aggregate,
                InMemoryWorkflowExecutionRepository::UNPERSISTED_VERSION,
            )
            .await
            .unwrap();
        let found = rehydrate(&repository, &intent()).await.unwrap();
        assert_eq!(found.aggregate().intent_id(), &intent());
    }

    #[tokio::test]
    async fn the_version_a_rehydration_carries_is_the_one_the_store_assigned() {
        let mut repository = InMemoryWorkflowExecutionRepository::empty();
        let (aggregate, event) = genesis(1);
        repository
            .store(
                &event,
                &aggregate,
                InMemoryWorkflowExecutionRepository::UNPERSISTED_VERSION,
            )
            .await
            .unwrap();
        let found = rehydrate(&repository, &intent()).await.unwrap();
        assert_eq!(
            found.version(),
            1,
            "採番したのはストアであって seq_nr ではない"
        );
    }

    #[tokio::test]
    async fn a_write_that_presents_a_stale_version_conflicts() {
        // 楽観ロックの本体 — 読んだ版で書くから、その間の書込を検出できる。
        let mut repository = InMemoryWorkflowExecutionRepository::empty();
        let (aggregate, event) = genesis(1);
        repository
            .store(
                &event,
                &aggregate,
                InMemoryWorkflowExecutionRepository::UNPERSISTED_VERSION,
            )
            .await
            .unwrap();
        let err = repository
            .store(
                &event,
                &aggregate,
                InMemoryWorkflowExecutionRepository::UNPERSISTED_VERSION,
            )
            .await
            .unwrap_err();
        assert_eq!(
            err,
            RepositoryError::Conflict {
                expected: 0,
                actual: 1
            }
        );
    }

    #[tokio::test]
    async fn the_port_takes_the_aggregate_by_reference_so_the_caller_keeps_it() {
        let mut repository = InMemoryWorkflowExecutionRepository::empty();
        let (aggregate, event) = genesis(1);
        repository
            .store(
                &event,
                &aggregate,
                InMemoryWorkflowExecutionRepository::UNPERSISTED_VERSION,
            )
            .await
            .unwrap();
        // 引数は `&` — store は呼出側の集約を変更しない (BR1.3)。
        assert_eq!(aggregate.seq_nr(), 1);
    }

    #[tokio::test]
    async fn the_repository_face_reports_its_failures_as_repository_errors() {
        let repository = InMemoryWorkflowExecutionRepository::empty();
        let err = repository.find_by_id(&intent()).await.unwrap_err();
        assert!(matches!(err, RepositoryError::NotFound { .. }));
    }
}
