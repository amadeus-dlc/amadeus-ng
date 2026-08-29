//! 契約テストの試験装置 — `IntentRepository` の実装が**どのバックエンドでも
//! 同じ約束を満たす**ことを 1 度だけ書いて共有するための足場 (BR2.7)。
//!
//! バックエンドごとの差 (本家の memory / SQLite) はこの [`StoreFixture`] に閉じ、契約
//! そのものは [`contract`] のジェネリック関数が持つ。

#![allow(dead_code)]

pub(crate) mod contract;

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    CommandError, Intent, IntentEvent, IntentId, StageDisplay, StageEntry, StartRequest,
    WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
    WorkflowDefinitionId,
};
use core_command_use_case::orchestration::{IntentRepository, RehydratedIntent};

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

/// 契約テストがバックエンドごとの差を吸収するための試験装置。
///
/// 「同じストアを別のインスタンスから開き直す」を、バックエンドによらず同じ形で書けるように
/// するための唯一の抽象である。
///
/// # この trait が課す約束 (BR2.7 — どのバックエンドにも同じ約束を課す)
///
/// - [`open`](StoreFixture::open) は**空のストア**を指す新しい Repository を返す。同じ
///   試験装置から 2 度呼べば、互いに独立した 2 つの空のストアになる。
/// - [`reopen`](StoreFixture::reopen) は、その呼出しの**時点までに `repository` が書き終えた
///   行**が見える別インスタンスを返す。
pub(crate) trait StoreFixture {
    /// 試験対象の Repository (内包するバックエンドだけが違う)。
    type Repository: IntentRepository;

    /// **空のストア**を指す新しい Repository を開く (呼ぶたびに独立した空のストア)。
    fn open(&self) -> Self::Repository;

    /// `repository` が書いているストアを、別のインスタンスから開き直す。
    fn reopen(&self, repository: &Self::Repository) -> Self::Repository;
}

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
pub(crate) fn genesis() -> (Intent, IntentEvent) {
    genesis_for(intent_id())
}

/// 指定した集約識別子の genesis (横断読取のテストが 2 集約を並べるのに使う)。
#[must_use]
pub(crate) fn genesis_for(intent: IntentId) -> (Intent, IntentEvent) {
    Intent::start_from_plan_unchecked(
        intent,
        WorkflowDefinitionId::parse("claude").expect("契約テストの定義 id"),
        DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64)))
            .expect("契約テストの定義 revision"),
        &StartRequest::new("classic", "contract").with_depth("standard"),
        stages(),
        scan(),
        at(),
    )
    .expect("合成計画は start の前提を満たす")
}

/// 1 件書いてから**握り直す** (書込後の楽観 version を知っているのはストアだけ — BR5.3)。
///
/// `store` は引数の集約を変更しないので、次のコマンドを打つ前に再水和するのが唯一の作法で
/// ある。`find_by_id` は「書いた集約 + ストアが採番した version」を返す。
pub(crate) async fn store_and_reload<R: IntentRepository>(
    repository: &mut R,
    event: &IntentEvent,
    aggregate: &Intent,
    expected_version: usize,
) -> RehydratedIntent {
    repository
        .store(event, aggregate, expected_version)
        .await
        .expect("store");
    repository
        .find_by_id(aggregate.id())
        .await
        .expect("書いた集約は握り直せる")
}

/// genesis (`Started`) を 1 件書き、握り直した結果を返す。
pub(crate) async fn store_genesis<R: IntentRepository>(repository: &mut R) -> RehydratedIntent {
    store_genesis_for(repository, intent_id()).await
}

/// 指定した集約識別子の genesis を 1 件書き、握り直した結果を返す。
pub(crate) async fn store_genesis_for<R: IntentRepository>(
    repository: &mut R,
    intent: IntentId,
) -> RehydratedIntent {
    let (aggregate, event) = genesis_for(intent);
    store_and_reload(repository, &event, &aggregate, R::UNPERSISTED_VERSION).await
}

/// 握っている再水和結果へコマンドを 1 つ打ち、書いて握り直す。
///
/// 版は**握っているものを提示する** — 書込直前に読み直さないのが楽観ロックの本体である。
pub(crate) async fn advance<R, F>(
    repository: &mut R,
    held: &RehydratedIntent,
    command: F,
) -> RehydratedIntent
where
    R: IntentRepository,
    F: FnOnce(&mut Intent) -> Result<IntentEvent, CommandError>,
{
    let mut aggregate = held.aggregate().clone();
    let event = command(&mut aggregate).expect("コマンドは受理される");
    store_and_reload(repository, &event, &aggregate, held.version()).await
}

/// 続きの 1 件 (`StageCompleted`) を書き、握り直した結果を返す。
pub(crate) async fn store_stage_completed<R: IntentRepository>(
    repository: &mut R,
    held: &RehydratedIntent,
) -> RehydratedIntent {
    advance(repository, held, |aggregate| aggregate.complete_stage(at())).await
}
