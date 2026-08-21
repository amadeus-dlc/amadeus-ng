//! `ReviewClass` — `adversarial` / `advisory` (upstream 01 §3.2)。
//!
//! 列挙と契約的意味の**正準所有者は verification コンテキスト** (裁定 B7)。
//! workflow-definition 側は外部キー参照として保持するだけで、意味論の解釈はここでは行わない。
//! TODO(B7: 項目 3 スライス B): verification クレート新設時に正準 3 値束
//! (`none < advisory < adversarial` — 01 §125) へ統合し、本型は依存参照へ移行する。

/// レビュークラス。`reviewer` を宣言したステージのみが持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReviewClass {
    Adversarial,
    Advisory,
}

/// 閉集合外の値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownReviewClass(pub String);

impl ReviewClass {
    pub const ALL: [ReviewClass; 2] = [ReviewClass::Adversarial, ReviewClass::Advisory];

    /// # Errors
    ///
    /// 2 値以外は `UnknownReviewClass` で拒否する。
    pub fn parse(s: &str) -> Result<ReviewClass, UnknownReviewClass> {
        Ok(match s {
            "adversarial" => ReviewClass::Adversarial,
            "advisory" => ReviewClass::Advisory,
            other => return Err(UnknownReviewClass(other.to_string())),
        })
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ReviewClass::Adversarial => "adversarial",
            ReviewClass::Advisory => "advisory",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn both_values_round_trip_and_unknown_is_rejected() {
        for r in ReviewClass::ALL {
            assert_eq!(ReviewClass::parse(r.as_str()).unwrap(), r);
        }
        assert_eq!(
            ReviewClass::parse("Adversarial"),
            Err(UnknownReviewClass("Adversarial".to_string()))
        );
    }

    proptest! {
        #[test]
        fn rejects_anything_outside_the_closed_set(s in "[a-z]{1,14}") {
            let known = ReviewClass::ALL.iter().any(|r| r.as_str() == s);
            prop_assert_eq!(ReviewClass::parse(&s).is_ok(), known);
        }
    }
}
