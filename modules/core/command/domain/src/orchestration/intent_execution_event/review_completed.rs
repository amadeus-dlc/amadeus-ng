//! `ReviewCompleted` — `IntentExecutionEvent::ReviewCompleted` のペイロード。

use crate::orchestration::{IntentExecutionEventId, IntentExecutionId, ReviewVerdict};
use crate::workflow_definition::StageSlug;

/// conductor がレビュアーの判定を読み取った事実（監査行 `REVIEW_COMPLETED`）。
///
/// 依頼（[`ReviewRequested`]）と**対**になって初めて受領証になる — 開いている依頼が無い
/// 判定は集約が拒む（upstream `freshReviewReceipts` が `pendingRequests` に無い
/// `REVIEW_COMPLETED` を捨てるのと同じ会計）。
///
/// 要求本文とReview節を含む完成文書の指紋、および要求時ソースを結合として保存する。
///
/// [`ReviewRequested`]: super::review_requested::ReviewRequested
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewCompleted {
    evidence: crate::orchestration::ReviewCompletion,
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    stage: StageSlug,
    reviewer: String,
    iteration: u32,
    verdict: ReviewVerdict,
}

impl ReviewCompleted {
    /// 判定の材料を束ねる。
    #[must_use]
    pub fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        stage: StageSlug,
        reviewer: impl Into<String>,
        iteration: u32,
        verdict: ReviewVerdict,
        evidence: crate::orchestration::ReviewCompletion,
    ) -> ReviewCompleted {
        ReviewCompleted {
            evidence,
            id,
            aggregate_id,
            stage,
            reviewer: reviewer.into(),
            iteration,
            verdict,
        }
    }

    /// この操作が確定した内容結合。
    #[must_use]
    pub const fn evidence(&self) -> &crate::orchestration::ReviewCompletion {
        &self.evidence
    }
    /// 判定が返ったステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// 判定を返したレビュアー。
    #[must_use]
    pub fn reviewer(&self) -> &str {
        &self.reviewer
    }

    /// 何回目の依頼に対する判定か（1 始まり）。
    #[must_use]
    pub const fn iteration(&self) -> u32 {
        self.iteration
    }

    /// 返ってきた判定。
    #[must_use]
    pub const fn verdict(&self) -> ReviewVerdict {
        self.verdict
    }

    /// このイベント自身の識別子。
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
