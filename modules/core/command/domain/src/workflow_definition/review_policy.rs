//! `ReviewPolicy` — 1 ステージ 1 実行分のレビュー方針（定義から解決した静的材料）。

use crate::orchestration::ReviewVerdict;

use super::review_cap_value::ReviewCapValue;

/// レビュアーを宣言したステージの、この実行における方針。
///
/// 3 入力（ステージ宣言・スコープの `review_cap:`・実行の `Review Override`）から
/// [`WorkflowDefinition::review_policy`] が解決する値オブジェクトである。解決そのものは
/// 定義集約の判断であり、`IntentExecution` はこの値を**引数で受け取る**
/// （`coding-rules/aggregate-references.md` — 他集約は ID で参照し材料は引数で渡す）。
///
/// [`WorkflowDefinition::review_policy`]: super::workflow_definition::WorkflowDefinition::review_policy
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewPolicy {
    reviewer: String,
    effective: ReviewCapValue,
    max_iterations: u32,
    per_unit: bool,
}

impl ReviewPolicy {
    /// `reviewer_max_iterations:` を宣言しないステージの既定（upstream
    /// `node.reviewer_max_iterations ?? 2`）。
    pub const DEFAULT_MAX_ITERATIONS: u32 = 2;

    /// 解決済みの 4 材料を束ねる（**この型の唯一の構築経路**）。
    #[must_use]
    pub fn new(
        reviewer: impl Into<String>,
        effective: ReviewCapValue,
        max_iterations: u32,
        per_unit: bool,
    ) -> ReviewPolicy {
        ReviewPolicy {
            reviewer: reviewer.into(),
            effective,
            max_iterations,
            per_unit,
        }
    }

    /// 宣言されたレビュアー（`--reviewer` はこれと一致しなければならない）。
    #[must_use]
    pub fn reviewer(&self) -> &str {
        &self.reviewer
    }

    /// 実効クラス — 宣言 × スコープ上限 × override の min()。
    #[must_use]
    pub const fn effective(&self) -> ReviewCapValue {
        self.effective
    }

    /// 反駁ループの上限回数（`adversarial` のときだけ効く）。
    #[must_use]
    pub const fn max_iterations(&self) -> u32 {
        self.max_iterations
    }

    /// per-unit ステージか（`for_each: unit-of-work`）。**本 build では未配線** —
    /// `--unit` の受領証は繰延であり、この値は材料として運ぶだけである（設計 §1 の繰延）。
    #[must_use]
    pub const fn per_unit(&self) -> bool {
        self.per_unit
    }

    /// この試行で許される依頼の回数。
    ///
    /// upstream `handleReview` の budget 導出（ピン `3c3146cf` `:966-968`）の写しである:
    /// `none` → 0（依頼そのものが通らない）、`advisory` → 1、`adversarial` →
    /// `reviewer_max_iterations`。
    #[must_use]
    pub const fn budget(&self) -> u32 {
        match self.effective {
            ReviewCapValue::None => 0,
            ReviewCapValue::Advisory => 1,
            ReviewCapValue::Adversarial => self.max_iterations,
        }
    }

    /// 承認が受領証を要するか（実効クラスが `none` でない）。
    ///
    /// upstream `verifyReviewerPrecondition` は実効クラスが `none` に落ちたら**何も
    /// 要求せず返る**（ピン `:1810-1812`）— 「レビュアーを呼ぶな」と言われた実行に
    /// 受領証を求めるのは矛盾だからである。
    #[must_use]
    pub const fn requires_receipt(&self) -> bool {
        !matches!(self.effective, ReviewCapValue::None)
    }

    /// その判定が**終端**か（それ以上レビューを回さないか）。
    ///
    /// upstream `terminalReviewVerdict`（`aidlc-lib.ts:4760-4778`）の写しである:
    /// `none` はそもそも終端を作らない、`READY` は常に終端、`NOT-READY` は advisory
    /// （1 パスで終わる）か反復上限に達したときだけ終端になる。
    #[must_use]
    pub const fn is_terminal(&self, verdict: ReviewVerdict, iteration: u32) -> bool {
        match self.effective {
            ReviewCapValue::None => false,
            ReviewCapValue::Advisory => true,
            ReviewCapValue::Adversarial => match verdict {
                ReviewVerdict::Ready => true,
                ReviewVerdict::NotReady => iteration >= self.max_iterations,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(effective: ReviewCapValue, max_iterations: u32) -> ReviewPolicy {
        ReviewPolicy::new("aidlc-quality-agent", effective, max_iterations, false)
    }

    /// budget は実効クラスで決まる（upstream `:966-968`）。
    #[test]
    fn the_budget_is_zero_for_none_one_for_advisory_and_the_cap_for_adversarial() {
        assert_eq!(policy(ReviewCapValue::None, 3).budget(), 0);
        assert_eq!(policy(ReviewCapValue::Advisory, 3).budget(), 1);
        assert_eq!(policy(ReviewCapValue::Adversarial, 3).budget(), 3);
        assert_eq!(
            policy(
                ReviewCapValue::Adversarial,
                ReviewPolicy::DEFAULT_MAX_ITERATIONS
            )
            .budget(),
            2
        );
    }

    /// 実効 `none` だけが受領証を要らない（upstream `:1810-1812`）。
    #[test]
    fn only_an_effective_none_waives_the_receipt() {
        assert!(!policy(ReviewCapValue::None, 2).requires_receipt());
        assert!(policy(ReviewCapValue::Advisory, 2).requires_receipt());
        assert!(policy(ReviewCapValue::Adversarial, 2).requires_receipt());
    }

    /// `terminalReviewVerdict` の表（`aidlc-lib.ts:4760-4778`）。
    #[test]
    fn the_terminal_table_matches_the_upstream_helper() {
        // none: 何も終端にならない
        for verdict in ReviewVerdict::ALL {
            for iteration in 1..=3 {
                assert!(!policy(ReviewCapValue::None, 2).is_terminal(verdict, iteration));
            }
        }
        // advisory: 1 パスで終わるので verdict によらず終端
        for verdict in ReviewVerdict::ALL {
            assert!(policy(ReviewCapValue::Advisory, 2).is_terminal(verdict, 1));
        }
        // adversarial: READY は常に終端、NOT-READY は上限到達で終端
        let adversarial = policy(ReviewCapValue::Adversarial, 2);
        assert!(adversarial.is_terminal(ReviewVerdict::Ready, 1));
        assert!(!adversarial.is_terminal(ReviewVerdict::NotReady, 1));
        assert!(adversarial.is_terminal(ReviewVerdict::NotReady, 2));
        assert!(adversarial.is_terminal(ReviewVerdict::NotReady, 3));
    }

    #[test]
    fn the_materials_are_carried_verbatim() {
        let resolved = ReviewPolicy::new(
            "aidlc-architecture-reviewer-agent",
            ReviewCapValue::Advisory,
            4,
            true,
        );
        assert_eq!(resolved.reviewer(), "aidlc-architecture-reviewer-agent");
        assert_eq!(resolved.effective(), ReviewCapValue::Advisory);
        assert_eq!(resolved.max_iterations(), 4);
        assert!(resolved.per_unit());
    }
}
