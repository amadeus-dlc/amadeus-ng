//! `ReviewClosure` — 判定が返って閉じた依頼 1 件。

use super::review_verdict::ReviewVerdict;

/// 「何回目の依頼が、どう判定されて閉じたか」。
///
/// 依頼（`REVIEW_REQUESTED`）と判定（`REVIEW_COMPLETED`）が**対**になって初めて受領証に
/// なるという upstream の会計（`freshReviewReceipts` は `pendingRequests` に無い
/// `REVIEW_COMPLETED` を捨てる — `aidlc-lib.ts:5182-5184`）を、集約の状態としてそのまま
/// 持ったものである。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewClosure {
    iteration: u32,
    verdict: ReviewVerdict,
}

impl ReviewClosure {
    /// 通し番号と判定を束ねる。
    #[must_use]
    pub const fn new(iteration: u32, verdict: ReviewVerdict) -> ReviewClosure {
        ReviewClosure { iteration, verdict }
    }

    /// 何回目の依頼だったか（1 始まり）。
    #[must_use]
    pub const fn iteration(&self) -> u32 {
        self.iteration
    }

    /// 返ってきた判定。
    #[must_use]
    pub const fn verdict(&self) -> ReviewVerdict {
        self.verdict
    }
}
