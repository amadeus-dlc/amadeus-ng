//! `ReviewLogOutcome` — `RecordReviewUseCase` が成功したときの材料。

/// 何を書いたか。合成ルートはこれで stdout の JSON 1 行を組む
/// （`{"emitted":"REVIEW_REQUESTED","stage":"<slug>"}` — 逐語は出す側が持つ）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewLogOutcome {
    /// 依頼を書いた。`retry` は `Retry: pending-request` の呼び直しだったか。
    Requested {
        /// 判定待ちの依頼の呼び直しだったか。
        retry: bool,
    },
    /// 判定を書いた。
    Completed,
}
