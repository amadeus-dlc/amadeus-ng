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

/// 同じストアを何度でも開き直せる試験装置。
///
/// 「新しいインスタンスで読み直す」= 別プロセスからの再オープン相当を、実装によらず
/// 同じ形で書けるようにするための唯一の抽象である。
///
/// 開き直しの引数に**書き終えた Repository** を取るのは、ストアが共有ハンドルではなく
/// 単一所有になったからである (coding-rules/interior-mutability.md)。SQLite 実装は同じ
/// ファイルへ新しい接続を開いて引数を無視し、in-memory 実装は Repository が持つ 3 表を
/// 引き継いだ別インスタンスを作る。どちらも「それまでに書き終えた行が見える別インスタンス」
/// という同じ観測になる。
pub(crate) trait StoreFixture {
    /// 試験対象の Repository 実装。
    type Repository: WorkflowExecutionRepository;
    /// 同じストアを読む `JournalReader` 実装。
    type Reader: JournalReader;

    /// 空のストアを指す**新しい** Repository を開く。
    fn open(&self) -> Self::Repository;

    /// `repository` が書き終えたストアを、別のインスタンスから開き直す。
    fn reopen(&self, repository: &Self::Repository) -> Self::Repository;

    /// `repository` が書き終えたストアを読む `JournalReader` を開く。
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

/// 書込後に呼出側が行う版の載せ替え (BR1.3 — `store` は引数の集約を変更しない)。
#[must_use]
pub(crate) const fn advanced(
    aggregate: WorkflowExecution,
    event: &WorkflowExecutionEvent,
) -> WorkflowExecution {
    aggregate.with_version(event.seq_nr())
}
