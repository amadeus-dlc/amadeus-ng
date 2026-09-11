//! `ReviewPolicy` — 1 ステージ 1 実行分のレビュー方針（定義から解決した静的材料）。

use crate::orchestration::{ArtifactTarget, ReviewVerdict};

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

    /// per-unit ステージか（`for_each: unit-of-work`）。
    ///
    /// 受領証の**射程**がこの値で決まる — per-unit ステージの受領証は Unit ごとなので、
    /// Unit 名の取れない書込みを覆わない（[`ReviewPolicy::receipt_covers`]）。`--unit` を
    /// 伴う受領証の**記録**は本 build にまだ無い（`aidlc log review --unit` は拒否する）ので、
    /// per-unit ステージのステージ水準の書込みは凍結されない — upstream の観測と同じである。
    #[must_use]
    pub const fn per_unit(&self) -> bool {
        self.per_unit
    }

    /// この方針の受領証が、その書込み先を覆うか。
    ///
    /// upstream の `judgeFreeze` は `for_each: unit-of-work` の枝で `unitVerdicts` /
    /// `unitPending` **だけ**を見て `stageVerdict` を読まない（ピン `a277af21`
    /// `hooks/aidlc-review-freeze.ts:147-193`）。つまり per-unit ステージの受領証は Unit ごと
    /// であり、Unit 名の取れない書込み（ゼロ Unit の実行がステージ直下へ置く成果物）は
    /// 覆わない。反復軸を持たないステージでは、宣言成果物のすべてを 1 つの受領証が覆う。
    ///
    /// 宣言外（[`ArtifactTarget::Foreign`]）はそもそも受領証の対象ではない。
    ///
    /// # Unit 宛先は「その Unit の受領証」ではなく試行で決まる
    ///
    /// upstream は Unit 宛先の凍結に `unitVerdicts.has(targetUnit)` を要求する。本 build に
    /// Unit 鍵の受領証は無い（`aidlc log review --unit` は未配線）ので、ステージ 1 つの試行を
    /// その代わりに使う。したがってステージ水準の受領証しか無いときの Unit 宛先は、upstream が
    /// 通すのに対し本 build は凍結する — **本 build のほうが厳しい**既知の差である。
    #[must_use]
    pub const fn receipt_covers(&self, target: &ArtifactTarget) -> bool {
        match target {
            ArtifactTarget::Foreign => false,
            ArtifactTarget::Unit(_) => true,
            ArtifactTarget::Stage => !self.per_unit,
        }
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

    /// 反復軸を持たないステージの受領証は、宣言成果物のすべてを覆う。
    #[test]
    fn a_stage_level_receipt_covers_every_declared_target() {
        let stage_level = policy(ReviewCapValue::Adversarial, 2);
        assert!(!stage_level.per_unit());
        assert!(stage_level.receipt_covers(&ArtifactTarget::Stage));
        assert!(!stage_level.receipt_covers(&ArtifactTarget::Foreign));
    }

    /// per-unit ステージの受領証は Unit ごとである — ステージ水準の宛先は覆わない。
    ///
    /// upstream `judgeFreeze` の `for_each: unit-of-work` 枝が `stageVerdict` を読まない
    /// ことの写しである（`hooks/aidlc-review-freeze.ts:147-193`）。
    #[test]
    fn a_per_unit_receipt_covers_only_a_unit_scoped_target() {
        let per_unit = ReviewPolicy::new(
            "aidlc-architecture-reviewer-agent",
            ReviewCapValue::Adversarial,
            2,
            true,
        );
        assert!(per_unit.per_unit());
        assert!(
            !per_unit.receipt_covers(&ArtifactTarget::Stage),
            "ゼロ Unit の実行がステージ直下へ置く成果物は覆わない"
        );
        assert!(
            per_unit.receipt_covers(&ArtifactTarget::Unit(
                crate::orchestration::UnitName::parse("u2-workflow-authority").unwrap()
            )),
            "Unit を名指した書込みは覆う"
        );
        assert!(!per_unit.receipt_covers(&ArtifactTarget::Foreign));
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
