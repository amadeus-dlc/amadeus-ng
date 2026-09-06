//! `PromotionPlanError` — 昇格の計画 ([`super::PracticesPromotion::plan`]) の拒否。

use std::fmt;

use super::promoted_sections_error::PromotedSectionsError;

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
    /// 置き換える節の見出しが重複した。
    ///
    /// 計画は固定 5 種の見出しを順に 1 度ずつ見るので**構成不能**だが、置き換える節の列
    /// ([`super::PromotedSections`]) の構築検査を握り潰さないために変種を持つ
    /// (プロダクトコードで `unwrap` を使わないため)。
    DuplicateSection(String),
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
            PromotionPlanError::DuplicateSection(heading) => {
                write!(f, "duplicate promoted section {heading}")
            }
        }
    }
}

impl std::error::Error for PromotionPlanError {}

impl From<PromotedSectionsError> for PromotionPlanError {
    /// 置き換える節の列が拒んだ形を、昇格の計画の拒否へ写す。
    fn from(error: PromotedSectionsError) -> PromotionPlanError {
        match error {
            PromotedSectionsError::DuplicateHeading { heading } => {
                PromotionPlanError::DuplicateSection(heading)
            }
        }
    }
}

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

    /// 節の列の拒否は計画の拒否へそのまま写り、見出しの綴りを持ったまま届く。
    #[test]
    fn a_duplicate_section_arrives_as_the_plans_own_refusal() {
        let inner = PromotedSectionsError::DuplicateHeading {
            heading: "## Testing Posture".to_string(),
        };
        let error = PromotionPlanError::from(inner);
        assert_eq!(
            error,
            PromotionPlanError::DuplicateSection("## Testing Posture".to_string())
        );
        assert_eq!(
            error.to_string(),
            "duplicate promoted section ## Testing Posture"
        );
    }

    /// 拒否は材料を自分で持つので、原因の連鎖はここで終わる。
    #[test]
    fn the_refusal_owns_its_material_so_the_chain_ends_here() {
        let error = PromotionPlanError::DuplicateSection("## Code Style".to_string());
        assert!(std::error::Error::source(&error).is_none());
    }
}
