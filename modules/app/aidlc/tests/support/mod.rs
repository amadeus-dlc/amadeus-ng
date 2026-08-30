//! 合成ルート側の試験装置 — クラッシュ再構成テストが集約を 1 件ずつ書き進めるための足場。
//!
//! コマンド側の契約テスト（`core-command-interface-adapter` の `tests/support/`）とは別物で
//! ある。あちらは「どのバックエンドでも同じ約束を満たす」ことを共有するための `StoreFixture`
//! 抽象を持つが、こちらが要るのは「1 件書いて握り直す」という手順だけである。統合テストの
//! モジュールはクレートを跨げないので、必要な分だけをここに置く。

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    CommandError, Created, Intent, IntentExecution, IntentExecutionEvent, IntentExecutionId,
    IntentId, StageDisplay, StageEntry, StartRequest, WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
    WorkflowDefinitionId,
};
use core_command_use_case::orchestration::IntentExecutionRepository;

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

/// 契約テストの実行識別子 (ストアの集約キー)。
pub(crate) const EXECUTION: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";

/// 契約テストの集約識別子。
#[must_use]
pub(crate) fn intent_id() -> IntentId {
    IntentId::parse(INTENT).expect("契約テストの IntentId は UUIDv7")
}

/// 契約テストの実行識別子。
#[must_use]
pub(crate) fn execution_id() -> IntentExecutionId {
    IntentExecutionId::parse(EXECUTION).expect("契約テストの IntentExecutionId は UUIDv7")
}

/// ストアに存在しない実行識別子。
#[must_use]
pub(crate) fn absent_execution_id() -> IntentExecutionId {
    IntentExecutionId::parse(ABSENT_INTENT).expect("契約テストの IntentExecutionId は UUIDv7")
}

/// 合成計画の表示属性 (投影の検収は RMU 側の専用テストが持つので固定値でよい)。
fn display(number: &str, name: &str) -> StageDisplay {
    StageDisplay::new(
        StageNumber::parse(number).expect("契約テストのステージ番号は文法内"),
        name,
        "orchestrator",
    )
    .expect("単一行")
}

/// 合成計画の走査結果。
#[must_use]
pub(crate) fn scan() -> WorkspaceScan {
    WorkspaceScan::new(
        BrownfieldGreenfield::Greenfield,
        "Unknown",
        "Unknown",
        "Unknown",
    )
    .expect("単一行")
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
            display("0.1", "State Init"),
        ),
        StageEntry::new(
            slug("intent-capture"),
            PhaseId::Ideation,
            PlanAction::Execute,
            false,
            display("1.1", "Intent Capture"),
        ),
        StageEntry::new(
            slug("scope-definition"),
            PhaseId::Ideation,
            PlanAction::Execute,
            false,
            display("1.4", "Scope Definition"),
        ),
    ]
}

/// genesis の集約と `Started` イベント (`seq_nr` = 1。版はまだストアに無い)。
#[must_use]
pub(crate) fn genesis() -> (IntentExecution, IntentExecutionEvent) {
    genesis_for(execution_id())
}

/// 契約テストの intent (解決済み合成計画)。
#[must_use]
pub(crate) fn intent() -> Intent {
    Intent::from(Created::new(
        intent_id(),
        WorkflowDefinitionId::parse("claude").expect("契約テストの定義 id"),
        DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64)))
            .expect("契約テストの定義 revision"),
        StartRequest::new("classic", "contract").with_depth("standard"),
        stages(),
        scan(),
    ))
}

/// 指定した集約識別子の genesis (横断読取のテストが 2 集約を並べるのに使う)。
#[must_use]
pub(crate) fn genesis_for(execution: IntentExecutionId) -> (IntentExecution, IntentExecutionEvent) {
    IntentExecution::start(execution, intent(), at())
}

/// 1 件書いてから**握り直す** (書込後の楽観 version を知っているのはストアだけ — BR5.3)。
///
/// `store` は引数の集約を変更しないので、次のコマンドを打つ前に再水和するのが唯一の作法で
/// ある。`find_by_id` は書いた集約を、ストアが採番した version を刻んだ姿で返す。
pub(crate) async fn store_and_reload<R: IntentExecutionRepository>(
    repository: &mut R,
    event: &IntentExecutionEvent,
    aggregate: &IntentExecution,
) -> IntentExecution {
    repository.store(event, aggregate).await.expect("store");
    repository
        .find_by_id(aggregate.id())
        .await
        .expect("書いた集約は握り直せる")
}

/// genesis (`Started`) を 1 件書き、握り直した結果を返す。
pub(crate) async fn store_genesis<R: IntentExecutionRepository>(
    repository: &mut R,
) -> IntentExecution {
    store_genesis_for(repository, execution_id()).await
}

/// 指定した集約識別子の genesis を 1 件書き、握り直した結果を返す。
pub(crate) async fn store_genesis_for<R: IntentExecutionRepository>(
    repository: &mut R,
    execution: IntentExecutionId,
) -> IntentExecution {
    let (aggregate, event) = genesis_for(execution);
    store_and_reload(repository, &event, &aggregate).await
}

/// 握っている集約へコマンドを 1 つ打ち、書いて握り直す。
///
/// 版は**握っている集約が運んでいるもの**を提示する — 書込直前に読み直さないのが楽観ロックの
/// 本体である。
pub(crate) async fn advance<R, F>(
    repository: &mut R,
    held: &IntentExecution,
    command: F,
) -> IntentExecution
where
    R: IntentExecutionRepository,
    F: FnOnce(&mut IntentExecution) -> Result<IntentExecutionEvent, CommandError>,
{
    let mut aggregate = held.clone();
    let event = command(&mut aggregate).expect("コマンドは受理される");
    store_and_reload(repository, &event, &aggregate).await
}

/// 続きの 1 件 (`StageCompleted`) を書き、握り直した結果を返す。
pub(crate) async fn store_stage_completed<R: IntentExecutionRepository>(
    repository: &mut R,
    held: &IntentExecution,
) -> IntentExecution {
    advance(repository, held, |aggregate| {
        aggregate.complete_stage(&intent(), at())
    })
    .await
}
