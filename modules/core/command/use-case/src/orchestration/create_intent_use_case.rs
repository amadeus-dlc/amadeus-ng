//! `CreateIntent` — intent の鋳造（upstream `intent-create` の決定論部分）。
//!
//! **書き込む動詞なのでコマンド側にある**（`coding-rules/cqrs-boundaries.md` 規則 5 —
//! 「コマンド側のユースケースは最終的に書き込む」）。形は追補裁定の逐語どおり
//! 「定義を `find_by_id` → `Intent::create` → 新生集約を `store`」であり、そこに実行の
//! genesis が続く。
//!
//! # なぜ 1 つのユースケースが 2 つの集約を書くのか
//!
//! upstream の `intent-create` が 1 つの決定論的な移動として「intent を鋳造し、その最初の
//! 実行を開始する」からである（`aidlc-utility.ts:3833` の逐語「mint intent + scan +
//! state-init」）。`Intent` だけを書いて実行を開始しないと、直後の `next` が読む
//! `aidlc-state.md` がまだ描かれておらず、ワークフローが始まったのに進められない。
//! **1 トランザクション 1 集約**の規範は守っている — 2 つの集約はそれぞれの Repository で
//! 別々に書かれ、束ねているのはトランザクションではなくこの動詞の意図である。
//!
//! # 採番と時計は持たない
//!
//! 識別子（`IntentId` / `IntentExecutionId`）と発生時刻は**引数で受ける**。集約が乱数も
//! 時計も持たないのと同じ理由で、ユースケースも持たない — 決定的にテストできなくなる。
//! 採番と時計は合成ルート（U7）の持ち物である。引数は集約 ID と値オブジェクトだけで、
//! 集約インスタンスは渡らない（`coding-rules/use-case-rules.md` §2b）。
//!
//! # 登録簿は書かない
//!
//! upstream は同じ移動の中で `intents.json`（登録簿）と `active-intent` カーソルも動かすが、
//! 登録簿の直列化機構は未決である（`11-workspace.md` §10 — ADR-010 が「SQLite のテーブルへ
//! 移して RMU が投影する」を筋と書くが裁定は別 Bolt）。カーソルと record ディレクトリの
//! 用意はマシンローカルな構造の I/O なので合成ルートが行う。

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    Intent, IntentExecution, IntentExecutionId, IntentId, StartRequest, WorkspaceScan,
};
use core_command_domain::workflow_definition::WorkflowDefinitionId;

use super::create_intent_error::CreateIntentError;
use super::port::{IntentExecutionRepository, IntentRepository, WorkflowDefinitionRepository};

/// intent を鋳造し、その最初の実行を開始する。
///
/// ポートを 3 本保持し、`execute` の内部で使う（`coding-rules/use-case-rules.md` §2b —
/// リポジトリをユースケースの外で使わない）。束縛はスタティック（単相化）である。
#[derive(Debug)]
pub struct CreateIntentUseCase<D, I, E> {
    definition_repository: D,
    intent_repository: I,
    execution_repository: E,
}

impl<D, I, E> CreateIntentUseCase<D, I, E>
where
    D: WorkflowDefinitionRepository,
    I: IntentRepository,
    E: IntentExecutionRepository,
{
    /// ポートの実装を 3 つ注入する。
    pub const fn new(
        definition_repository: D,
        intent_repository: I,
        execution_repository: E,
    ) -> CreateIntentUseCase<D, I, E> {
        CreateIntentUseCase {
            definition_repository,
            intent_repository,
            execution_repository,
        }
    }

    /// intent を鋳造して永続化し、続けてその最初の実行を開始して永続化する。
    ///
    /// # Errors
    ///
    /// 定義の取得の失敗（`DefinitionRepository`）、genesis の拒否（`Intent` — 未知スコープ
    /// など）、intent の永続化の失敗（`IntentRepository` — 同じ識別子が既にあれば
    /// `Conflict`）、実行の永続化の失敗（`ExecutionRepository`）を返す。いずれも
    /// **そのまま伝播**する。
    pub async fn execute(
        &mut self,
        intent_id: IntentId,
        execution_id: IntentExecutionId,
        definition_id: &WorkflowDefinitionId,
        request: StartRequest,
        scan: WorkspaceScan,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), CreateIntentError> {
        let definition = self.definition_repository.find_by_id(definition_id)?;
        let (intent, born) = Intent::create(intent_id, &definition, request, scan)?;
        self.intent_repository
            .store(&born, &intent, occurred_at)
            .await?;
        // intent が着地してから実行を開始する。逆順にすると、実行だけがストアに居て
        // 指す先の intent が無い状態が残りうる（RMU の `PlanUnavailable` に落ちる）。
        let (execution, started) = IntentExecution::start(execution_id, &intent, occurred_at);
        self.execution_repository
            .store(&started, &execution)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // panic! は想定外バリアントの即時失敗という検証用途で使う。
    #![allow(clippy::panic)]

    use super::*;
    use crate::orchestration::RepositoryError;
    use crate::orchestration::test_support::{
        InMemoryIntentExecutionRepository, InMemoryIntentRepository,
        InMemoryWorkflowDefinitionRepository, at, definition, definition_id, execution_id, intent,
        scan,
    };
    use core_command_domain::orchestration::IntentError;

    fn use_case(
        stage_count: usize,
    ) -> CreateIntentUseCase<
        InMemoryWorkflowDefinitionRepository,
        InMemoryIntentRepository,
        InMemoryIntentExecutionRepository,
    > {
        CreateIntentUseCase::new(
            InMemoryWorkflowDefinitionRepository::holding(definition(stage_count)),
            InMemoryIntentRepository::empty(),
            InMemoryIntentExecutionRepository::empty(),
        )
    }

    fn request() -> StartRequest {
        StartRequest::new("classic", "build the auth service")
    }

    /// 鋳造した intent が Repository に着地し、定義から解決した計画を持っている。
    #[tokio::test]
    async fn creating_an_intent_stores_the_minted_aggregate() {
        let mut use_case = use_case(3);

        use_case
            .execute(
                intent(),
                execution_id(),
                &definition_id(),
                request(),
                scan(),
                at(),
            )
            .await
            .expect("鋳造は成功する");

        let stored = use_case
            .intent_repository
            .find_by_id(&intent())
            .await
            .expect("鋳造した intent はストアに居る");
        assert_eq!(stored.id(), &intent());
        assert_eq!(stored.stage_count(), 3, "定義の 3 段が計画として解決される");
    }

    /// 鋳造に続けて最初の実行が開始され、カーソルは先頭に置かれる。
    ///
    /// これが無いと直後の `next` が読む `aidlc-state.md` を RMU が描けない — intent だけを
    /// 書いて実行を開始しない状態は、ワークフローが始まったのに進められない中途半端である。
    #[tokio::test]
    async fn creating_an_intent_starts_its_first_execution() {
        let mut use_case = use_case(3);

        use_case
            .execute(
                intent(),
                execution_id(),
                &definition_id(),
                request(),
                scan(),
                at(),
            )
            .await
            .expect("鋳造は成功する");

        let execution = use_case
            .execution_repository
            .find_by_id(&execution_id())
            .await
            .expect("開始した実行はストアに居る");
        assert_eq!(execution.id(), &execution_id());
        assert_eq!(
            execution.intent_id(),
            &intent(),
            "実行は鋳造した intent を指す"
        );
        assert_eq!(execution.cursor().to_usize(), 0, "カーソルは先頭");
        assert_eq!(execution.stage_count(), 3);
    }

    /// 定義が知らないスコープは集約の拒否がそのまま伝播する（言い換えない）。
    #[tokio::test]
    async fn an_unknown_scope_propagates_the_aggregate_refusal() {
        let mut use_case = use_case(3);

        let error = use_case
            .execute(
                intent(),
                execution_id(),
                &definition_id(),
                StartRequest::new("no-such-scope", "build the auth service"),
                scan(),
                at(),
            )
            .await
            .expect_err("未知スコープは拒否される");

        assert!(
            matches!(
                error,
                CreateIntentError::Intent(IntentError::UnknownScope(_))
            ),
            "{error:?}"
        );
    }

    /// 拒否された鋳造は**何も書かない** — intent も実行もストアに現れない。
    #[tokio::test]
    async fn a_refused_creation_stores_nothing() {
        let mut use_case = use_case(3);

        let _ = use_case
            .execute(
                intent(),
                execution_id(),
                &definition_id(),
                StartRequest::new("no-such-scope", "build the auth service"),
                scan(),
                at(),
            )
            .await;

        assert!(
            use_case
                .intent_repository
                .find_by_id(&intent())
                .await
                .is_err(),
            "intent は書かれていない"
        );
        assert!(
            use_case
                .execution_repository
                .find_by_id(&execution_id())
                .await
                .is_err(),
            "実行も書かれていない"
        );
    }

    /// このハーネスが提供していない定義 id は Repository の拒否がそのまま伝播する。
    #[tokio::test]
    async fn an_unknown_definition_propagates_the_repository_refusal() {
        let mut use_case = use_case(3);
        let other = WorkflowDefinitionId::parse("kiro").expect("定義 id は文法内");

        let error = use_case
            .execute(intent(), execution_id(), &other, request(), scan(), at())
            .await
            .expect_err("提供していない定義 id は拒否される");

        assert!(
            matches!(
                error,
                CreateIntentError::DefinitionRepository(RepositoryError::NotFound { .. })
            ),
            "{error:?}"
        );
    }

    /// 同じ識別子で二度鋳造すると、2 回目は intent の genesis 重複で拒否される。
    #[tokio::test]
    async fn minting_the_same_intent_twice_conflicts() {
        let mut use_case = use_case(3);

        use_case
            .execute(
                intent(),
                execution_id(),
                &definition_id(),
                request(),
                scan(),
                at(),
            )
            .await
            .expect("1 回目は成功する");
        let error = use_case
            .execute(
                intent(),
                execution_id(),
                &definition_id(),
                request(),
                scan(),
                at(),
            )
            .await
            .expect_err("2 回目は拒否される");

        assert!(
            matches!(
                error,
                CreateIntentError::IntentRepository(RepositoryError::Conflict { .. })
            ),
            "{error:?}"
        );
    }

    /// 失敗の位置が変種で分かる（出す側が復旧手順を選べる材料になっている）。
    #[test]
    fn the_error_envelope_names_the_failing_port() {
        let error = CreateIntentError::DefinitionRepository(RepositoryError::NotFound {
            id: definition_id(),
        });
        assert!(error.to_string().starts_with("definition repository: "));

        let error = CreateIntentError::IntentRepository(RepositoryError::Conflict {
            expected: 0,
            actual: 1,
        });
        assert!(error.to_string().starts_with("intent repository: "));

        let error = CreateIntentError::ExecutionRepository(RepositoryError::Conflict {
            expected: 0,
            actual: 1,
        });
        assert!(error.to_string().starts_with("execution repository: "));

        let error = CreateIntentError::Intent(IntentError::Empty);
        assert!(error.to_string().starts_with("intent: "));
    }
}
