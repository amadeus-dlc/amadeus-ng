//! `PromotedSectionsError` — 昇格が書き替える節の列の不変条件を破った形。

use std::fmt;

/// [`PromotedSections`] が満たすべき不変条件の違反 (材料のみ — 利用者向け文言はアダプタ層)。
///
/// [`PromotedSections`]: super::PromotedSections
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromotedSectionsError {
    /// 同じ見出しが 2 回以上現れる (後勝ちで静かに上書きすると、どちらの本文が
    /// team.md へ書かれたか監査行から追えなくなる)。
    DuplicateHeading {
        /// 列の順で最初に重複した見出し名 (`## ` を含まない)。
        heading: String,
    },
}

impl fmt::Display for PromotedSectionsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PromotedSectionsError::DuplicateHeading { heading } => {
                write!(f, "duplicate promoted section heading: {heading}")
            }
        }
    }
}

impl std::error::Error for PromotedSectionsError {}

#[cfg(test)]
mod tests {
    use super::PromotedSectionsError;

    #[test]
    fn the_violation_renders_its_material() {
        assert_eq!(
            PromotedSectionsError::DuplicateHeading {
                heading: "Way of Working".to_string(),
            }
            .to_string(),
            "duplicate promoted section heading: Way of Working"
        );
    }

    #[test]
    fn the_violation_is_a_std_error() {
        let error: &dyn std::error::Error = &PromotedSectionsError::DuplicateHeading {
            heading: "Testing Posture".to_string(),
        };
        assert!(error.source().is_none(), "材料を自分で持つので連鎖しない");
    }

    #[test]
    fn violations_compare_by_value() {
        let way = PromotedSectionsError::DuplicateHeading {
            heading: "Way of Working".to_string(),
        };
        assert_eq!(way.clone(), way);
        assert_ne!(
            way,
            PromotedSectionsError::DuplicateHeading {
                heading: "Deployment".to_string(),
            }
        );
    }
}
