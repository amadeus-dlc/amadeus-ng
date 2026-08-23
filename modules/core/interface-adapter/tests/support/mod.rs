//! 契約テストの試験装置 — `WorkflowExecutionRepository` / `JournalReader` の実装が
//! **どれも同じ約束を満たす**ことを 1 度だけ書いて共有するための足場 (BR2.7)。
//!
//! 実装ごとの差 (in-memory / SQLite) はこの [`StoreFixture`] に閉じ、契約そのものは
//! [`contract`] のジェネリック関数が持つ。

#![allow(dead_code)]

pub(crate) mod contract;

use core_domain::orchestration::{
    IntentId, StageEntry, StartRequest, WorkflowExecution, WorkflowExecutionEvent,
};
use core_domain::workflow_definition::{
    DefinitionRevision, PhaseId, PlanAction, StageSlug, WorkflowDefinitionId,
};
use core_use_case::orchestration::{JournalReader, WorkflowExecutionRepository};

/// イベント封筒の `occurred_at` (集約は値を素通しするので固定値でよい)。
pub(crate) const AT: &str = "2026-08-23T00:00:00Z";

/// 契約テストが使う集約識別子 (UUIDv7)。
pub(crate) const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

/// ストアに存在しない集約識別子。
pub(crate) const ABSENT_INTENT: &str = "018f3b2c-4d5e-7f60-8abc-def012345678";

/// 契約テストが実装ごとの差を吸収するための試験装置。
///
/// 「新しいインスタンスで読み直す」= 別プロセスからの再オープン相当を、実装によらず同じ形で
/// 書けるようにするための唯一の抽象である。
///
/// 開き直し・Reader が引数に**書き終えた Repository** を取るのは、ストアが共有ハンドルでは
/// なく単一所有になったからである (coding-rules/interior-mutability.md)。どこのストアを
/// 指すかは引数の Repository が決め、試験装置は「唯一のストア」を抱え込まない。
///
/// # この trait が課す約束 (BR2.7 — 両実装に同じ約束を課す)
///
/// - [`open`](StoreFixture::open) は**空のストア**を指す新しい Repository を返す。同じ
///   試験装置から 2 度呼べば、互いに独立した 2 つの空のストアになる。
/// - [`reopen`](StoreFixture::reopen) / [`reader`](StoreFixture::reader) は、その呼出しの
///   **時点までに `repository` が書き終えた行**が見える別インスタンスを返す。
///
/// # 契約の外 — 開いた後の書込 (適用範囲の明示)
///
/// `reopen` / `reader` で得たインスタンスが、それ**以降**に `repository` が書いた行を観測
/// するかどうかは**実装依存**であり、BR2.7 の適用範囲外である。内部可変性を禁じている以上
/// (coding-rules/interior-mutability.md)、in-memory 実装は 3 表の写しを渡すしかなく、
/// SQLite 実装の「同じファイルへの生きた接続」と揃えるには共有可変状態が要るためである。
///
/// したがって契約テストは必ず**「書き終えてから開く」順序**で書くこと。逆順 (先に開いて後から
/// 書く) を契約テストに書くと、片方の実装だけ通るテストになる。
///
/// 実装ごとの実際の挙動は、契約の外であっても
/// `workflow_execution_repository_contract.rs` の実装固有テスト 4 本
/// (`in_memory_*` / `sqlite_*` の `..._writes_made_after_...`) が固定している。挙動が変われば
/// 必ずそのどれかが落ちる。
pub(crate) trait StoreFixture {
    /// 試験対象の Repository 実装。
    type Repository: WorkflowExecutionRepository;
    /// 同じストアを読む `JournalReader` 実装。
    type Reader: JournalReader;

    /// **空のストア**を指す新しい Repository を開く (呼ぶたびに独立した空のストア)。
    fn open(&self) -> Self::Repository;

    /// `repository` が**この呼出しの時点までに**書き終えたストアを、別のインスタンスから
    /// 開き直す。
    fn reopen(&self, repository: &Self::Repository) -> Self::Repository;

    /// `repository` が**この呼出しの時点までに**書き終えたストアを読む `JournalReader` を
    /// 開く。
    fn reader(&self, repository: &Self::Repository) -> Self::Reader;
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

/// genesis の集約と `Started` イベント (`seq_nr` = 1、`version` = 0)。
#[must_use]
pub(crate) fn genesis() -> (WorkflowExecution, WorkflowExecutionEvent) {
    WorkflowExecution::start_with_entries(
        intent_id(),
        WorkflowDefinitionId::parse("claude").expect("契約テストの定義 id"),
        DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64)))
            .expect("契約テストの定義 revision"),
        &StartRequest::new("classic", "contract").with_depth("standard"),
        stages(),
        AT,
    )
    .expect("合成計画は start の前提を満たす")
}

/// genesis (`Started`) を 1 件書き、版を載せ替えた集約を返す。
pub(crate) async fn store_genesis<R: WorkflowExecutionRepository>(
    repository: &mut R,
) -> WorkflowExecution {
    let (aggregate, event) = genesis();
    repository
        .store(&event, &aggregate)
        .await
        .expect("genesis の store は通る");
    advanced(aggregate, &event)
}

/// 続きの 1 件 (`StageCompleted`) を書き、版を載せ替えた集約を返す。
pub(crate) async fn store_stage_completed<R: WorkflowExecutionRepository>(
    repository: &mut R,
    mut aggregate: WorkflowExecution,
) -> WorkflowExecution {
    let event = aggregate.complete_stage(AT).expect("索引 0 は非ゲート");
    repository.store(&event, &aggregate).await.expect("store");
    advanced(aggregate, &event)
}

/// 書込後に呼出側が行う版の載せ替え (BR1.3 — `store` は引数の集約を変更しない)。
#[must_use]
pub(crate) const fn advanced(
    aggregate: WorkflowExecution,
    event: &WorkflowExecutionEvent,
) -> WorkflowExecution {
    aggregate.with_version(event.seq_nr())
}
