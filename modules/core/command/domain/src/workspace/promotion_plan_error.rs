//! `PromotionPlanError` — 昇格の計画 ([`super::PracticesPromotion::plan`]) の拒否。

use std::fmt;

/// 昇格の計画が組めなかった理由 (材料のみ — 逐語は出す側が組む)。
///
/// upstream は `replaceSection` / `appendUnderHeading` の throw を捕まえて
/// `practices-promote failed: …` に包むが、こちらは**書く前に**同じ検査を済ませる —
/// 計画は純粋な値であり、投影がそれを描く (設計 §1)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromotionPlanError {
    /// 正本 team.md に、ドラフトが本文を持つ節の見出しが無い。
    TeamHeadingMissing(String),
    /// 正本 project.md に、追記先の見出し (`## Mandated` / `## Forbidden`) が無い。
    ProjectHeadingMissing(String),
}

impl fmt::Display for PromotionPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PromotionPlanError::TeamHeadingMissing(heading) => {
                write!(f, "team.md has no heading {heading}")
            }
            PromotionPlanError::ProjectHeadingMissing(heading) => {
                write!(f, "project.md has no heading {heading}")
            }
        }
    }
}

impl std::error::Error for PromotionPlanError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_refusals_carry_the_heading_spelling() {
        assert_eq!(
            PromotionPlanError::TeamHeadingMissing("## Deployment".to_string()).to_string(),
            "team.md has no heading ## Deployment"
        );
        assert_eq!(
            PromotionPlanError::ProjectHeadingMissing("## Mandated".to_string()).to_string(),
            "project.md has no heading ## Mandated"
        );
    }
}
