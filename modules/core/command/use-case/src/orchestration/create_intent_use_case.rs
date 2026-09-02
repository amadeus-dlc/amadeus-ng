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
    workflow_definition_repository: D,
    intent_repository: I,
    intent_execution_repository: E,
}

impl<D, I, E> CreateIntentUseCase<D, I, E>
where
    D: WorkflowDefinitionRepository,
    I: IntentRepository,
    E: IntentExecutionRepository,
{
    /// ポートの実装を 3 つ注入する。
    pub const fn new(
        workflow_definition_repository: D,
        intent_repository: I,
        intent_execution_repository: E,
    ) -> CreateIntentUseCase<D, I, E> {
        CreateIntentUseCase {
            workflow_definition_repository,
            intent_repository,
            intent_execution_repository,
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
        let definition = self
            .workflow_definition_repository
            .find_by_id(definition_id)
            .await?;
        let (intent, born) = Intent::create(intent_id, &definition, request, scan, occurred_at)?;
        self.intent_repository.store(&born, &intent).await?;
        // intent が着地してから実行を開始する。逆順にすると、実行だけがストアに居て
        // 指す先の intent が無い状態が残りうる（RMU の `PlanUnavailable` に落ちる）。
        let (execution, started) = IntentExecution::start(execution_id, &intent, occurred_at);
        // ここで倒れると intent だけが着地した部分失敗になる。孤児の識別子を材料として
        // 包み、出す側が「intent は作られたが実行が始まっていない」と復旧手順を組めるように
        // する（issue #77 の先行改善 — 恒久対応は doctor の検出・修復）。
        self.intent_execution_repository
            .store(&started, &execution)
            .await
            .map_err(|error| CreateIntentError::ExecutionRepository {
                orphan: intent.id().clone(),
                error,
            })?;
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

    /// 鋳造に続けて最初の実行が開始され、カーソルは最初のゲート付きステージに置かれる。
    ///
    /// これが無いと直後の `next` が読む `aidlc-state.md` を RMU が描けない — intent だけを
    /// 書いて実行を開始しない状態は、ワークフローが始まったのに進められない中途半端である。
    ///
    /// 誕生 = 初期化完了済み（issue #76）なので、着地点は索引 0（initialization）ではなく
    /// 索引 1 である — 書き面が誕生の投影と同じ位置から始まり、両面が一致する。
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
            .intent_execution_repository
            .find_by_id(&execution_id())
            .await
            .expect("開始した実行はストアに居る");
        assert_eq!(execution.id(), &execution_id());
        assert_eq!(
            execution.intent_id(),
            &intent(),
            "実行は鋳造した intent を指す"
        );
        assert_eq!(
            execution.cursor().to_usize(),
            1,
            "initialization は誕生で完了済みなので、カーソルは最初のゲート付きステージ"
        );
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
                .intent_execution_repository
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

    /// intent 着地後に実行の永続化が失敗すると、エラーが孤児 intent の識別子を材料として
    /// 運ぶ（issue #77 の先行改善 — 出す側が「intent だけが着地した」ことと復旧手順を
    /// 組めるようにする）。
    #[tokio::test]
    async fn a_partial_failure_names_the_orphaned_intent() {
        let held_definition = definition(3);
        let (held_intent, _) = Intent::create(intent(), &held_definition, request(), scan(), at())
            .expect("フィクスチャの intent");
        let (held_execution, _) = IntentExecution::start(execution_id(), &held_intent, at());
        let mut use_case = CreateIntentUseCase::new(
            InMemoryWorkflowDefinitionRepository::holding(held_definition),
            InMemoryIntentRepository::empty(),
            // 最初の store に別の書き手が割り込む台本 — intent 着地後の実行書込だけが倒れる。
            InMemoryIntentExecutionRepository::holding_behind_concurrent_writes(
                held_execution,
                0,
                1,
            ),
        );

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
            .expect_err("実行の永続化が失敗する");

        let CreateIntentError::ExecutionRepository { orphan, .. } = &error else {
            panic!("実行ポートの変種で失敗する: {error:?}");
        };
        assert_eq!(orphan, &intent(), "孤児 intent の識別子を材料として運ぶ");
        // 部分失敗の証拠: intent 自体は既に着地している（これが孤児）。
        use_case
            .intent_repository
            .find_by_id(&intent())
            .await
            .expect("intent だけが着地している");
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

        let error = CreateIntentError::ExecutionRepository {
            orphan: intent(),
            error: RepositoryError::Conflict {
                expected: 0,
                actual: 1,
            },
        };
        // 孤児 id は前置 (出す側 `chained` の ends_with 重複抑止が効くよう、末尾は
        // ポートの失敗文言で終える — PR #87 Bugbot 指摘)。
        assert!(error.to_string().starts_with("execution repository ("));
        assert!(
            error.to_string().contains(intent().as_str()),
            "孤児 intent の識別子を材料として語る: {error}"
        );

        let error = CreateIntentError::Intent(IntentError::Empty);
        assert!(error.to_string().starts_with("intent: "));
    }
}
