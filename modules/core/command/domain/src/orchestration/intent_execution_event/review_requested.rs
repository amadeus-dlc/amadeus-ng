//! `ReviewRequested` — `IntentExecutionEvent::ReviewRequested` のペイロード。

use crate::orchestration::{IntentExecutionEventId, IntentExecutionId};
use crate::workflow_definition::StageSlug;

/// conductor がレビュアーを差し向けた事実（監査行 `REVIEW_REQUESTED`）。
///
/// `retry` は upstream の `Retry: pending-request` 欄に対応する — 差し向けたのに判定が
/// 返ってこなかった依頼を**もう一度**呼び直す形であり、依頼の回数には数えない
/// （ピン `3c3146cf` `aidlc-log.ts:810-812`）。したがって `retry` が真のイベントの適用は
/// フレーム空である。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewRequested {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    stage: StageSlug,
    reviewer: String,
    iteration: u32,
    retry: bool,
}

impl ReviewRequested {
    /// 依頼の材料を束ねる。
    #[must_use]
    pub fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        stage: StageSlug,
        reviewer: impl Into<String>,
        iteration: u32,
        retry: bool,
    ) -> ReviewRequested {
        ReviewRequested {
            id,
            aggregate_id,
            stage,
            reviewer: reviewer.into(),
            iteration,
            retry,
        }
    }

    /// レビューを依頼したステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// 差し向けたレビュアー（宣言と一致することは集約が確かめている）。
    #[must_use]
    pub fn reviewer(&self) -> &str {
        &self.reviewer
    }

    /// 何回目の依頼か（1 始まり）。
    #[must_use]
    pub const fn iteration(&self) -> u32 {
        self.iteration
    }

    /// 判定待ちの依頼の呼び直しか（`Retry: pending-request`）。
    #[must_use]
    pub const fn is_retry(&self) -> bool {
        self.retry
    }

    /// このイベント自身の識別子 — ドメインイベントはエンティティの一種なので自前の id を
    /// 持つ（`coding-rules/domain-object-kinds.md`）。
    #[must_use]
    pub const fn id(&self) -> &IntentExecutionEventId {
        &self.id
    }

    /// **どの集約の事実か** — この事実が起きた実行の識別子。
    #[must_use]
    pub const fn aggregate_id(&self) -> &IntentExecutionId {
        &self.aggregate_id
    }
}
