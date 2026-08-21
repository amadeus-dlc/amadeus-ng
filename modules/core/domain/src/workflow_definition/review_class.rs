//! `ReviewClass` — `adversarial` / `advisory` (upstream 01 §3.2)。
//!
//! 列挙と契約的意味の**正準所有者は verification コンテキスト** (裁定 B7)。
//! workflow-definition 側は外部キー参照として保持するだけで、意味論の解釈はここでは行わない。
//! TODO(B7: 項目 3 スライス B): verification クレート新設時に正準 3 値束
//! (`none < advisory < adversarial` — 01 §125) へ統合し、本型は依存参照へ移行する。

/// レビュークラス。`reviewer` を宣言したステージのみが持つ。
///
/// `derive(Ord)` は宣言順に従うため、正準の強度順 (`advisory < adversarial` — low-wins 束の
/// 上位 2 値) に合わせて **`Advisory` を先に宣言**する。`min` / ソートが強度順に一致する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReviewClass {
    Advisory,
    Adversarial,
}

/// 閉集合外の値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownReviewClass(String);

impl UnknownReviewClass {
    #[must_use]
    pub fn new(value: impl Into<String>) -> UnknownReviewClass {
        UnknownReviewClass(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ReviewClass {
    pub const ALL: [ReviewClass; 2] = [ReviewClass::Advisory, ReviewClass::Adversarial];

    /// # Errors
    ///
    /// 2 値以外は `UnknownReviewClass` で拒否する。
    pub fn parse(s: &str) -> Result<ReviewClass, UnknownReviewClass> {
        Ok(match s {
            "adversarial" => ReviewClass::Adversarial,
            "advisory" => ReviewClass::Advisory,
            other => return Err(UnknownReviewClass::new(other)),
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
    fn the_order_matches_the_canonical_strength_lattice() {
        // low-wins 束 (none < advisory < adversarial — 01 §125) の上位 2 値の順序
        assert!(ReviewClass::Advisory < ReviewClass::Adversarial);
        assert_eq!(ReviewClass::ALL.iter().min(), Some(&ReviewClass::Advisory));
    }

    #[test]
    fn both_values_round_trip_and_unknown_is_rejected() {
        for r in ReviewClass::ALL {
            assert_eq!(ReviewClass::parse(r.as_str()).unwrap(), r);
        }
        // 大文字始まりは閉集合外。生値を逐語で持ち帰る
        let rejected = ReviewClass::parse("Adversarial").unwrap_err();
        assert_eq!(rejected.as_str(), "Adversarial");
        assert_eq!(rejected, UnknownReviewClass::new("Adversarial"));
    }

    proptest! {
        #[test]
        fn rejects_anything_outside_the_closed_set(s in "[a-z]{1,14}") {
            let known = ReviewClass::ALL.iter().any(|r| r.as_str() == s);
            prop_assert_eq!(ReviewClass::parse(&s).is_ok(), known);
        }
    }
}
