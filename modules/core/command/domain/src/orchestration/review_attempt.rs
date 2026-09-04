//! `ReviewAttempt` — ステージ 1 つの**現在の試行**におけるレビュー会計。

use std::collections::BTreeSet;

use crate::workflow_definition::ReviewPolicy;

use super::review_closure::ReviewClosure;
use super::review_verdict::ReviewVerdict;

/// ステージ 1 つの現在の試行（直近の開始・差し戻し・ジャンプ以降の区間）の会計。
///
/// upstream の `reviewAttemptSummary`（`aidlc-log.ts:677-831`）は監査台帳を毎回読み返して
/// この値を組み立てるが、こちらは**集約の状態そのもの**として持つ — 鮮度の区切り
/// （フロア）は同じ集約の状態遷移が決めるからである（設計 §1）。
///
/// 空の試行（[`ReviewAttempt::default`]）が「まだ 1 度も依頼していない」を表す。フロアは
/// この値を空へ戻すことで表現される。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReviewAttempt {
    requests: u32,
    pending: BTreeSet<u32>,
    closed: Vec<ReviewClosure>,
}

impl ReviewAttempt {
    /// 保存された行から試行を組み直す（**永続化境界からの再構成専用**）。
    ///
    /// 通常の構築は [`ReviewAttempt::default`]（空の試行）と、集約の適用が呼ぶ
    /// `record_request` / `record_verdict` である。この口は DTO の復号だけが使う —
    /// 行の中身が壊れていれば集約の再構成がクラッシュするので、ここでは検査しない
    /// （壊れた歴史は回復せずクラッシュする — オーナー裁定 2026-08-30）。
    #[must_use]
    pub const fn restored(
        requests: u32,
        pending: BTreeSet<u32>,
        closed: Vec<ReviewClosure>,
    ) -> ReviewAttempt {
        ReviewAttempt {
            requests,
            pending,
            closed,
        }
    }

    /// 数え上げ済みの依頼数（`Retry: pending-request` は**数えない** — upstream
    /// `:810-812`）。
    #[must_use]
    pub const fn request_count(&self) -> u32 {
        self.requests
    }

    /// その通し番号の依頼が判定待ちか。
    #[must_use]
    pub fn is_pending(&self, iteration: u32) -> bool {
        self.pending.contains(&iteration)
    }

    /// 判定待ちの通し番号（昇順）。
    #[must_use]
    pub const fn pending(&self) -> &BTreeSet<u32> {
        &self.pending
    }

    /// 判定が返って閉じた依頼（記録順）。
    #[must_use]
    pub fn closed(&self) -> &[ReviewClosure] {
        &self.closed
    }

    /// この試行に**終端の受領証**があるか。
    ///
    /// # 非終端の NOT-READY は読み飛ばす（無効化しない）
    ///
    /// upstream で非終端 NOT-READY が受領証を無効化するのは、成果物 fingerprint が使える
    /// ときだけである（`aidlc-lib.ts:5218` の `if (verdict !== "NOT-READY" ||
    /// !fingerprintUsable) continue;`）。本 build は fingerprint を繰延しているので、
    /// 非終端の判定は単に終端ではないという扱いになる（設計 §2.3）。
    #[must_use]
    pub fn has_terminal(&self, policy: &ReviewPolicy) -> bool {
        self.closed
            .iter()
            .any(|closure| policy.is_terminal(closure.verdict(), closure.iteration()))
    }

    /// 新しい依頼を 1 件数える（通常の `REVIEW_REQUESTED`）。
    pub(super) fn record_request(&mut self, iteration: u32) {
        self.requests = self.requests.saturating_add(1);
        self.pending.insert(iteration);
    }

    /// 判定を 1 件閉じる（`REVIEW_COMPLETED`）。
    pub(super) fn record_verdict(&mut self, iteration: u32, verdict: ReviewVerdict) {
        self.pending.remove(&iteration);
        self.closed.push(ReviewClosure::new(iteration, verdict));
    }

    /// 試行を空へ戻す（フロア — 開始・差し戻し・ジャンプ）。
    pub(super) fn reset(&mut self) {
        *self = ReviewAttempt::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_definition::ReviewCapValue;

    fn policy(effective: ReviewCapValue) -> ReviewPolicy {
        ReviewPolicy::new("r", effective, 2, false)
    }

    #[test]
    fn a_fresh_attempt_is_empty() {
        let attempt = ReviewAttempt::default();
        assert_eq!(attempt.request_count(), 0);
        assert!(attempt.pending().is_empty());
        assert!(attempt.closed().is_empty());
        assert!(!attempt.has_terminal(&policy(ReviewCapValue::Adversarial)));
    }

    #[test]
    fn a_request_is_counted_and_left_pending_until_its_verdict_lands() {
        let mut attempt = ReviewAttempt::default();
        attempt.record_request(1);
        assert_eq!(attempt.request_count(), 1);
        assert!(attempt.is_pending(1));
        assert!(!attempt.is_pending(2));

        attempt.record_verdict(1, ReviewVerdict::Ready);
        assert_eq!(attempt.request_count(), 1);
        assert!(!attempt.is_pending(1));
        assert_eq!(attempt.closed().len(), 1);
        assert_eq!(
            attempt.closed().first().map(ReviewClosure::verdict),
            Some(ReviewVerdict::Ready)
        );
    }

    /// adversarial の 1 回目 NOT-READY は終端ではない（読み飛ばす — 無効化もしない）。
    #[test]
    fn a_below_cap_not_ready_is_not_terminal_and_does_not_invalidate() {
        let mut attempt = ReviewAttempt::default();
        attempt.record_request(1);
        attempt.record_verdict(1, ReviewVerdict::NotReady);
        assert!(!attempt.has_terminal(&policy(ReviewCapValue::Adversarial)));

        // 上限に達した 2 回目の NOT-READY は終端になる。
        attempt.record_request(2);
        attempt.record_verdict(2, ReviewVerdict::NotReady);
        assert!(attempt.has_terminal(&policy(ReviewCapValue::Adversarial)));
    }

    /// advisory は 1 回で終端（verdict によらない）。
    #[test]
    fn an_advisory_pass_is_terminal_at_the_first_verdict() {
        let mut attempt = ReviewAttempt::default();
        attempt.record_request(1);
        attempt.record_verdict(1, ReviewVerdict::NotReady);
        assert!(attempt.has_terminal(&policy(ReviewCapValue::Advisory)));
    }

    /// 実効 `none` は終端を作らない（承認は受領証を要求しないので実害はない）。
    #[test]
    fn an_effective_none_never_yields_a_terminal_receipt() {
        let mut attempt = ReviewAttempt::default();
        attempt.record_request(1);
        attempt.record_verdict(1, ReviewVerdict::Ready);
        assert!(!attempt.has_terminal(&policy(ReviewCapValue::None)));
    }

    #[test]
    fn a_reset_empties_the_attempt() {
        let mut attempt = ReviewAttempt::default();
        attempt.record_request(1);
        attempt.record_verdict(1, ReviewVerdict::Ready);
        attempt.reset();
        assert_eq!(attempt, ReviewAttempt::default());
    }
}
