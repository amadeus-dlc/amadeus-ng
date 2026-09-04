//! `ReviewLogKind` — `aidlc-log review` が書こうとしている行の種類。

use core_command_domain::orchestration::ReviewVerdict;

/// 動詞 1 回が書く行の種類。`--verdict` の有無だけで決まる（upstream `handleReview` の
/// `if (flags.verdict === undefined)` — ピン `3c3146cf` `aidlc-log.ts:983`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewLogKind {
    /// レビュアーの差し向け（`REVIEW_REQUESTED`）。
    Request {
        /// 判定待ちの依頼の呼び直しか（`--retry-pending`）。
        retry_pending: bool,
    },
    /// レビュアーの判定の記録（`REVIEW_COMPLETED`）。
    Verdict(ReviewVerdict),
}
