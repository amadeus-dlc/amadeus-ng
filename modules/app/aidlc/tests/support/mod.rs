//! 合成ルート側の試験装置 — クラッシュ再構成テストが集約を 1 件ずつ書き進めるための足場。
//!
//! コマンド側の契約テスト（`core-command-interface-adapter` の `tests/support/`）とは別物で
//! ある。あちらは「どのバックエンドでも同じ約束を満たす」ことを共有するための `StoreFixture`
//! 抽象を持つが、こちらが要るのは「1 件書いて握り直す」という手順だけである。統合テストの
//! モジュールはクレートを跨げないので、必要な分だけをここに置く。

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use core_command_use_case::orchestration::{
    RehydratedWorkflowExecution, WorkflowExecutionRepository,
};
use core_domain::orchestration::{
    CommandError, IntentId, StageEntry, StartRequest, WorkflowExecution, WorkflowExecutionEvent,
};
use core_domain::workflow_definition::{
    DefinitionRevision, PhaseId, PlanAction, StageSlug, WorkflowDefinitionId,
};

/// イベントの `occurred_at` の逐語形 (集約は値を素通しするので固定値でよい)。
pub(crate) const AT_TEXT: &str = "2026-08-23T00:00:00Z";

/// イベントの `occurred_at`。
pub(crate) fn at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(AT_TEXT)
        .expect("固定の ISO 8601 UTC")
        .with_timezone(&Utc)
}

/// 契約テストが使う集約識別子 (UUIDv7)。
pub(crate) const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

/// ストアに存在しない集約識別子。
pub(crate) const ABSENT_INTENT: &str = "018f3b2c-4d5e-7f60-8abc-def012345678";

/// 契約テストの集約識別子。
#[must_use]
pub(crate) fn intent_id() -> IntentId {
    IntentId::parse(INTENT).expect("契約テストの IntentId は UUIDv7")
}

/// ストアに存在しない集約識別子。
#[must_use]
pub(crate) fn absent_intent_id() -> IntentId {
    IntentId::parse(ABSENT_INTENT).expect("契約テストの IntentId は UUIDv7")
}

fn slug(value: &str) -> StageSlug {
    StageSlug::parse(value).expect("契約テストの slug は文法内")
}

/// 3 ステージの合成計画 (索引 0 = initialization、1〜2 = ideation)。
#[must_use]
pub(crate) fn stages() -> Vec<StageEntry> {
    vec![
        StageEntry::new(
            slug("state-init"),
            PhaseId::Initialization,
            PlanAction::Execute,
            false,
        ),
        StageEntry::new(
            slug("intent-capture"),
            PhaseId::Ideation,
            PlanAction::Execute,
            false,
        ),
        StageEntry::new(
            slug("scope-definition"),
            PhaseId::Ideation,
            PlanAction::Execute,
            false,
        ),
    ]
}

/// genesis の集約と `Started` イベント (`seq_nr` = 1。版はまだストアに無い)。
#[must_use]
pub(crate) fn genesis() -> (WorkflowExecution, WorkflowExecutionEvent) {
    genesis_for(intent_id())
}

/// 指定した集約識別子の genesis (横断読取のテストが 2 集約を並べるのに使う)。
#[must_use]
pub(crate) fn genesis_for(intent: IntentId) -> (WorkflowExecution, WorkflowExecutionEvent) {
    WorkflowExecution::start_from_plan_unchecked(
        intent,
        WorkflowDefinitionId::parse("claude").expect("契約テストの定義 id"),
        DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64)))
            .expect("契約テストの定義 revision"),
        &StartRequest::new("classic", "contract").with_depth("standard"),
        stages(),
        at(),
    )
    .expect("合成計画は start の前提を満たす")
}

/// 1 件書いてから**握り直す** (書込後の楽観 version を知っているのはストアだけ — BR5.3)。
///
/// `store` は引数の集約を変更しないので、次のコマンドを打つ前に再水和するのが唯一の作法で
/// ある。`find_by_id` は「書いた集約 + ストアが採番した version」を返す。
pub(crate) async fn store_and_reload<R: WorkflowExecutionRepository>(
    repository: &mut R,
    event: &WorkflowExecutionEvent,
    aggregate: &WorkflowExecution,
    expected_version: usize,
) -> RehydratedWorkflowExecution {
    repository
        .store(event, aggregate, expected_version)
        .await
        .expect("store");
    repository
        .find_by_id(aggregate.intent_id())
        .await
        .expect("書いた集約は握り直せる")
}

/// genesis (`Started`) を 1 件書き、握り直した結果を返す。
pub(crate) async fn store_genesis<R: WorkflowExecutionRepository>(
    repository: &mut R,
) -> RehydratedWorkflowExecution {
    store_genesis_for(repository, intent_id()).await
}

/// 指定した集約識別子の genesis を 1 件書き、握り直した結果を返す。
pub(crate) async fn store_genesis_for<R: WorkflowExecutionRepository>(
    repository: &mut R,
    intent: IntentId,
) -> RehydratedWorkflowExecution {
    let (aggregate, event) = genesis_for(intent);
    store_and_reload(repository, &event, &aggregate, R::UNPERSISTED_VERSION).await
}

/// 握っている再水和結果へコマンドを 1 つ打ち、書いて握り直す。
///
/// 版は**握っているものを提示する** — 書込直前に読み直さないのが楽観ロックの本体である。
pub(crate) async fn advance<R, F>(
    repository: &mut R,
    held: &RehydratedWorkflowExecution,
    command: F,
) -> RehydratedWorkflowExecution
where
    R: WorkflowExecutionRepository,
    F: FnOnce(&mut WorkflowExecution) -> Result<WorkflowExecutionEvent, CommandError>,
{
    let mut aggregate = held.aggregate().clone();
    let event = command(&mut aggregate).expect("コマンドは受理される");
    store_and_reload(repository, &event, &aggregate, held.version()).await
}

/// 続きの 1 件 (`StageCompleted`) を書き、握り直した結果を返す。
pub(crate) async fn store_stage_completed<R: WorkflowExecutionRepository>(
    repository: &mut R,
    held: &RehydratedWorkflowExecution,
) -> RehydratedWorkflowExecution {
    advance(repository, held, |aggregate| aggregate.complete_stage(at())).await
}
