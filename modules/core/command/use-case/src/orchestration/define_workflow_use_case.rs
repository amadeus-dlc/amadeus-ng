//! `DefineWorkflow` — コンパイル済み定義を読み、ジャーナルの定義をそれに合わせる。
//!
//! **書き込む動詞なのでコマンド側にある** (`coding-rules/cqrs-boundaries.md` 規則 5)。形は
//! 「集約 A ([`CompiledDefinition`]) をその Repository で読む → ビジネスロジック →
//! 集約 B ([`WorkflowDefinition`]) を `store`」— `CreateIntentUseCase` が定義を読んで
//! `Intent` を書くのと同型の正規形であり、`find_by_id` だけを呼んで何も書かない使い方
//! (= クエリ側の仕事) には当たらない。
//!
//! # 「取込境界」「暫定の足場」は退役した (オーナー裁定 2026-09-02、b36)
//!
//! 配布束は**同一システムのドメインモデル**なので、外部システムクライアントでも暫定の
//! 播種口でもなく、集約 `CompiledDefinition` + その Repository である (#79 §1-4 / #80 の
//! 帰結 — 「クライアントをリポジトリに、クライアントが扱うデータを集約に昇格」)。
//! compile コンテキストが実装されたら (slice 2)、compile がこの集約の**書き手**になる —
//! 消えるのではなく、書き手を得る。
//!
//! # 時計は持たない
//!
//! 発生時刻は**引数で受ける**。集約が時計を持たないのと同じ理由でユースケースも持たない —
//! 決定的にテストできなくなる。時計は合成ルート (U7) の持ち物である。

use chrono::{DateTime, Utc};
use core_command_domain::workflow_definition::{
    CompiledDefinition, CompiledDefinitionId, RedefineError, WorkflowDefinition,
    WorkflowDefinitionId,
};

use super::define_workflow_error::DefineWorkflowError;
use super::port::{CompiledDefinitionRepository, RepositoryError, WorkflowDefinitionRepository};

/// コンパイル済み定義 (配布束) を読み、ジャーナルの定義をそれに合わせる。
///
/// ポートを 2 本保持し、`execute` の内部で使う (`coding-rules/use-case-rules.md` §2b —
/// リポジトリをユースケースの外で使わない)。束縛はスタティック (単相化) である。
#[derive(Debug)]
pub struct DefineWorkflowUseCase<C, R> {
    compiled_definition_repository: C,
    workflow_definition_repository: R,
}

impl<C, R> DefineWorkflowUseCase<C, R>
where
    C: CompiledDefinitionRepository,
    R: WorkflowDefinitionRepository,
{
    /// ポートの実装を 2 つ注入する。
    pub const fn new(
        compiled_definition_repository: C,
        workflow_definition_repository: R,
    ) -> DefineWorkflowUseCase<C, R> {
        DefineWorkflowUseCase {
            compiled_definition_repository,
            workflow_definition_repository,
        }
    }

    /// コンパイル済み定義 (配布束) を読み、ジャーナルの定義をそれに合わせる。
    ///
    /// **冪等である** — 配布物の内容版がストアの定義と同じなら何も書かない。判断は集約が
    /// 持っており ([`WorkflowDefinition::redefine`] の `Unchanged` ガード)、ここが内容版を
    /// 比較して分岐することはない (tell-dont-ask.md — 判断を集約の外で再実装しない)。
    /// ここでやっているのは「変化が無い」という拒否を成功へ畳むルーティングだけである。
    ///
    /// 成功では何も返さない (CQS の Command — 裁定 7)。何が起きたかを知りたい呼出側は
    /// 定義を読み直す。
    ///
    /// 2 つの識別子は合成ルートが同じ `harness.json` から鋳造する — 集約は各自の ID 型を
    /// 持ち、Repository は自集約の ID で引くので、系譜の突合せは値 (同じ name) で成立する。
    ///
    /// # Errors
    ///
    /// コンパイル済み定義の取得の失敗 (`CompiledDefinitionRepository`)、ジャーナル側の定義の
    /// 取得ないし永続化の失敗 (`DefinitionRepository` — 別プロセスが先に改訂していれば
    /// `Conflict`)、集約が改訂を拒否した (`Redefine` — 通番の枯渇) を返す。
    pub async fn execute(
        &mut self,
        compiled_definition_id: &CompiledDefinitionId,
        definition_id: &WorkflowDefinitionId,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), DefineWorkflowError> {
        let compiled_definition = self
            .compiled_definition_repository
            .find_by_id(compiled_definition_id)
            .await
            .map_err(DefineWorkflowError::CompiledDefinitionRepository)?;
        match self
            .workflow_definition_repository
            .find_by_id(definition_id)
            .await
        {
            // まだ確立されていない — 誕生させる。
            Err(RepositoryError::NotFound { .. }) => {
                self.define(definition_id, compiled_definition, occurred_at)
                    .await
            }
            Ok(definition) => {
                self.redefine(definition, compiled_definition, occurred_at)
                    .await
            }
            Err(error) => Err(DefineWorkflowError::DefinitionRepository(error)),
        }
    }

    /// 定義を確立して書く (genesis)。
    async fn define(
        &mut self,
        definition_id: &WorkflowDefinitionId,
        compiled_definition: CompiledDefinition,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), DefineWorkflowError> {
        let id = definition_id.clone();
        let revision = compiled_definition.revision().clone();
        let (graph, grid, scopes) = compiled_definition.into_content();
        let (definition, event) =
            WorkflowDefinition::define(id, revision, graph, grid, scopes, occurred_at);
        self.workflow_definition_repository
            .store(&event, &definition)
            .await?;
        Ok(())
    }

    /// 既存の定義を配布束の内容版へ改訂して書く。内容が変わっていなければ何も書かない。
    async fn redefine(
        &mut self,
        mut definition: WorkflowDefinition,
        compiled_definition: CompiledDefinition,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), DefineWorkflowError> {
        let revision = compiled_definition.revision().clone();
        let (graph, grid, scopes) = compiled_definition.into_content();
        match definition.redefine(revision, graph, grid, scopes, occurred_at) {
            Ok(event) => {
                self.workflow_definition_repository
                    .store(&event, &definition)
                    .await?;
                Ok(())
            }
            // 変化が無いのは失敗ではない — 取込が冪等であることの帰結である。
            Err(RedefineError::Unchanged { .. }) => Ok(()),
            Err(error) => Err(DefineWorkflowError::Redefine(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::test_support::{
        InMemoryCompiledDefinitionRepository, InMemoryWorkflowDefinitionRepository, at, compiled,
        compiled_definition_id, definition, definition_id, definition_revision, other_revision,
    };
    use core_command_domain::workflow_definition::WorkflowDefinitionEvent;

    /// ストアが空のときの取込は誕生させる。
    #[tokio::test]
    async fn ingesting_into_an_empty_store_establishes_the_definition() {
        let mut use_case = DefineWorkflowUseCase::new(
            InMemoryCompiledDefinitionRepository::serving(compiled(definition_revision(), 3)),
            InMemoryWorkflowDefinitionRepository::empty(),
        );

        use_case
            .execute(&compiled_definition_id(), &definition_id(), at())
            .await
            .expect("取込は成功する");

        let stored = use_case
            .workflow_definition_repository
            .find_by_id(&definition_id())
            .await
            .expect("確立した定義はストアに居る");
        assert_eq!(stored.revision(), &definition_revision());
        assert_eq!(stored.graph().len(), 3, "配布物の内容がそのまま入る");
        assert_eq!(stored.seq_nr(), 1, "誕生の通番は 1");
        assert!(
            matches!(
                use_case.workflow_definition_repository.committed(),
                [WorkflowDefinitionEvent::Defined(_)]
            ),
            "ジャーナルに書かれるのは誕生 1 件"
        );
    }

    /// 配布物の内容版が変わっていれば改訂して書く。
    #[tokio::test]
    async fn ingesting_a_changed_distribution_redefines_the_definition() {
        let mut use_case = DefineWorkflowUseCase::new(
            InMemoryCompiledDefinitionRepository::serving(compiled(other_revision(), 5)),
            InMemoryWorkflowDefinitionRepository::holding(definition(3)),
        );

        use_case
            .execute(&compiled_definition_id(), &definition_id(), at())
            .await
            .expect("改訂は成功する");

        let stored = use_case
            .workflow_definition_repository
            .find_by_id(&definition_id())
            .await
            .expect("改訂した定義はストアに居る");
        assert_eq!(stored.revision(), &other_revision());
        assert_eq!(stored.graph().len(), 5, "内容が入れ替わる");
        assert_eq!(stored.id(), &definition_id(), "系譜 ID は不変");
        assert!(
            matches!(
                use_case.workflow_definition_repository.committed(),
                [WorkflowDefinitionEvent::Redefined(_)]
            ),
            "ジャーナルに書かれるのは改訂 1 件"
        );
    }

    /// 内容版が同じなら**何も書かない** — 取込は冪等である。
    #[tokio::test]
    async fn ingesting_an_unchanged_distribution_writes_nothing() {
        let mut use_case = DefineWorkflowUseCase::new(
            InMemoryCompiledDefinitionRepository::serving(compiled(definition_revision(), 3)),
            InMemoryWorkflowDefinitionRepository::holding(definition(3)),
        );

        use_case
            .execute(&compiled_definition_id(), &definition_id(), at())
            .await
            .expect("変化なしは成功である");

        assert!(
            use_case
                .workflow_definition_repository
                .committed()
                .is_empty(),
            "書くべき事実が無いのでジャーナルは伸びない"
        );
    }

    /// 何度取り込んでも結果は同じ (2 回目以降は書かない)。
    #[tokio::test]
    async fn ingesting_twice_leaves_a_single_event() {
        let mut use_case = DefineWorkflowUseCase::new(
            InMemoryCompiledDefinitionRepository::serving(compiled(definition_revision(), 3)),
            InMemoryWorkflowDefinitionRepository::empty(),
        );

        use_case
            .execute(&compiled_definition_id(), &definition_id(), at())
            .await
            .expect("1 回目は誕生");
        use_case
            .execute(&compiled_definition_id(), &definition_id(), at())
            .await
            .expect("2 回目は変化なし");

        assert_eq!(
            use_case.workflow_definition_repository.committed().len(),
            1,
            "同じ配布物を 2 度取り込んでも歴史は 1 件のまま"
        );
    }

    /// 配布物が読めなければ取込の失敗がそのまま伝播する (言い換えない)。
    #[tokio::test]
    async fn an_unreadable_distribution_propagates_the_client_refusal() {
        let mut use_case = DefineWorkflowUseCase::new(
            InMemoryCompiledDefinitionRepository::unreadable(),
            InMemoryWorkflowDefinitionRepository::empty(),
        );

        let error = use_case
            .execute(&compiled_definition_id(), &definition_id(), at())
            .await
            .expect_err("読めない配布束は拒否される");

        assert!(
            matches!(error, DefineWorkflowError::CompiledDefinitionRepository(_)),
            "{error:?}"
        );
        assert!(
            use_case
                .workflow_definition_repository
                .committed()
                .is_empty(),
            "拒否された取込は何も書かない"
        );
    }

    /// `NotFound` 以外の読取失敗はそのまま伝播する (取込の判断材料が無いので進まない)。
    #[tokio::test]
    async fn an_unreadable_definition_stops_the_ingestion() {
        let mut use_case = DefineWorkflowUseCase::new(
            InMemoryCompiledDefinitionRepository::serving(compiled(definition_revision(), 3)),
            InMemoryWorkflowDefinitionRepository::corrupt(),
        );

        let error = use_case
            .execute(&compiled_definition_id(), &definition_id(), at())
            .await
            .expect_err("壊れた記録の上には確立しない");

        assert!(
            matches!(
                error,
                DefineWorkflowError::DefinitionRepository(RepositoryError::Corrupt { .. })
            ),
            "{error:?}"
        );
        assert!(
            use_case
                .workflow_definition_repository
                .committed()
                .is_empty()
        );
    }

    /// 書込の失敗もそのまま伝播する (別の書き手が先に改訂していれば `Conflict`)。
    #[tokio::test]
    async fn a_stale_version_propagates_the_repository_conflict() {
        let mut use_case = DefineWorkflowUseCase::new(
            InMemoryCompiledDefinitionRepository::serving(compiled(other_revision(), 5)),
            InMemoryWorkflowDefinitionRepository::holding_behind_a_concurrent_write(definition(3)),
        );

        let error = use_case
            .execute(&compiled_definition_id(), &definition_id(), at())
            .await
            .expect_err("古い版を提示した改訂は弾かれる");

        assert!(
            matches!(
                error,
                DefineWorkflowError::DefinitionRepository(RepositoryError::Conflict { .. })
            ),
            "{error:?}"
        );
    }

    /// 通番が尽きた集約への改訂は、集約の拒否がそのまま伝播する。
    #[tokio::test]
    async fn an_exhausted_sequence_propagates_the_aggregate_refusal() {
        let mut use_case = DefineWorkflowUseCase::new(
            InMemoryCompiledDefinitionRepository::serving(compiled(other_revision(), 5)),
            InMemoryWorkflowDefinitionRepository::holding(definition(3).with_seq_nr(usize::MAX)),
        );

        let error = use_case
            .execute(&compiled_definition_id(), &definition_id(), at())
            .await
            .expect_err("通番の枯渇は拒否される");

        assert!(
            matches!(
                error,
                DefineWorkflowError::Redefine(RedefineError::SequenceExhausted)
            ),
            "{error:?}"
        );
        assert!(
            use_case
                .workflow_definition_repository
                .committed()
                .is_empty()
        );
    }

    /// 失敗の位置が変種で分かる (出す側が復旧手順を選べる材料になっている)。
    #[test]
    fn the_error_envelope_names_the_failing_port() {
        let error = DefineWorkflowError::DefinitionRepository(RepositoryError::NotFound {
            id: definition_id(),
        });
        assert!(error.to_string().starts_with("definition repository: "));

        let error = DefineWorkflowError::Redefine(RedefineError::SequenceExhausted);
        assert_eq!(error.to_string(), "redefine: sequence exhausted");
    }
}
