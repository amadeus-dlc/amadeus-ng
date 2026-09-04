//! `ReviewCapValue` — scope frontmatter の `review_cap:` の値 (3 値の閉集合)。

use super::unknown_review_cap::UnknownReviewCap;

/// `review_cap:` の値。`None` は「`none` と**宣言された**」ことを表す値であり、
/// 「宣言が無い」は `Option<ReviewCapValue>` の `None` 側で表す (2 つを混同しないこと)。
///
/// # 派生した `Ord` は強度順では**ない**
///
/// 宣言順は `ALL` の doc どおり「上限の緩い側から厳しい側へ」なので、`derive(Ord)` は
/// `Adversarial < Advisory < None` になる。正準の強度順は逆 (`none < advisory <
/// adversarial` — upstream `REVIEW_RANK`) なので、**強度の比較には
/// [`ReviewCapValue::rank`] と [`ReviewCapValue::weaker`] を使う**。`min` / `max` を
/// 直に当てないこと ([`ReviewClass`] は宣言順が強度順に一致する別の型である)。
///
/// [`ReviewClass`]: super::review_class::ReviewClass
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReviewCapValue {
    /// 上限を課さない。ステージ宣言のレビュー重量がそのまま通る (upstream 01 §6.2)。
    Adversarial,
    /// adversarial なステージを advisory 1 パスへ格下げする。
    Advisory,
    /// レビュアーのディスパッチ自体を無効化する。
    None,
}

impl ReviewCapValue {
    /// 宣言順の全値 (3 値の網羅走査の正本)。並びは上限の緩い側から厳しい側へ。
    pub const ALL: [ReviewCapValue; 3] = [
        ReviewCapValue::Adversarial,
        ReviewCapValue::Advisory,
        ReviewCapValue::None,
    ];

    /// # Errors
    ///
    /// `adversarial` / `advisory` / `none` 以外は `UnknownReviewCap` で拒否する。
    pub fn parse(s: &str) -> Result<ReviewCapValue, UnknownReviewCap> {
        Ok(match s {
            "adversarial" => ReviewCapValue::Adversarial,
            "advisory" => ReviewCapValue::Advisory,
            "none" => ReviewCapValue::None,
            other => return Err(UnknownReviewCap::new(other)),
        })
    }

    /// 正準の強度順位 (upstream `REVIEW_RANK` — `none` 0 < `advisory` 1 <
    /// `adversarial` 2)。
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            ReviewCapValue::None => 0,
            ReviewCapValue::Advisory => 1,
            ReviewCapValue::Adversarial => 2,
        }
    }

    /// low-wins 合成 — 2 値のうち**弱いほう**を返す。
    ///
    /// scope の `review_cap:` も `--review` override も、ステージの宣言を**下げるだけ**で
    /// あって上げることはできない (upstream `resolveReviewClass` — `aidlc-lib.ts:8753-8770`)。
    /// その合成をこの型が持つ。
    #[must_use]
    pub const fn weaker(self, other: ReviewCapValue) -> ReviewCapValue {
        if other.rank() < self.rank() {
            other
        } else {
            self
        }
    }

    /// frontmatter 上の正準綴り (`parse` の逆写像)。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ReviewCapValue::Adversarial => "adversarial",
            ReviewCapValue::Advisory => "advisory",
            ReviewCapValue::None => "none",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 正準の強度順は `none < advisory < adversarial` である (upstream `REVIEW_RANK`)。
    #[test]
    fn the_rank_follows_the_canonical_strength_lattice() {
        assert!(ReviewCapValue::None.rank() < ReviewCapValue::Advisory.rank());
        assert!(ReviewCapValue::Advisory.rank() < ReviewCapValue::Adversarial.rank());
    }

    /// 派生した `Ord` は強度順の**逆**である — 直に `min` を当ててはならない。
    #[test]
    fn the_derived_order_is_not_the_strength_order() {
        assert!(ReviewCapValue::Adversarial < ReviewCapValue::Advisory);
        assert_eq!(
            ReviewCapValue::Adversarial.min(ReviewCapValue::Advisory),
            ReviewCapValue::Adversarial,
            "min は強度の最小ではない — weaker を使うこと"
        );
    }

    /// low-wins は可換で、同値は自分自身を返す。
    #[test]
    fn the_low_wins_composition_returns_the_weaker_value() {
        for left in ReviewCapValue::ALL {
            for right in ReviewCapValue::ALL {
                let weaker = left.weaker(right);
                assert_eq!(weaker.rank(), left.rank().min(right.rank()));
                assert_eq!(weaker, right.weaker(left), "low-wins は可換である");
            }
        }
    }

    #[test]
    fn every_value_round_trips_and_unknown_is_rejected() {
        for value in ReviewCapValue::ALL {
            assert_eq!(ReviewCapValue::parse(value.as_str()).unwrap(), value);
        }
        assert_eq!(
            ReviewCapValue::parse("Adversarial").unwrap_err(),
            UnknownReviewCap::new("Adversarial")
        );
    }
}
