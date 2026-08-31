//! `ReviewClassView` — `adversarial` / `advisory` (upstream 01 §3.2)。

use super::unknown_value::UnknownValue;

/// レビュークラス。`reviewer` を宣言したステージのみが持つ。
///
/// `derive(Ord)` は宣言順に従うため、正準の強度順 (`advisory < adversarial` — low-wins 束の
/// 上位 2 値) に合わせて **`Advisory` を先に宣言**する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReviewClassView {
    /// 通常フローではちょうど 1 パス。所見は承認ゲートで人間がトリアージする。
    Advisory,
    /// 反駁と修復のループ。`reviewer_max_iterations` 回まで。
    Adversarial,
}

impl ReviewClassView {
    /// 宣言順の全値 = 正準の強度昇順 (`advisory < adversarial`)。
    pub const ALL: [ReviewClassView; 2] = [ReviewClassView::Advisory, ReviewClassView::Adversarial];

    /// # Errors
    ///
    /// 2 値以外は [`UnknownValue`] で拒否する。
    pub fn parse(s: &str) -> Result<ReviewClassView, UnknownValue> {
        Ok(match s {
            "adversarial" => ReviewClassView::Adversarial,
            "advisory" => ReviewClassView::Advisory,
            other => return Err(UnknownValue::new(other)),
        })
    }

    /// `stage-graph.json` 上の語 (`parse` の逆写像)。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ReviewClassView::Adversarial => "adversarial",
            ReviewClassView::Advisory => "advisory",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_order_matches_the_canonical_strength_lattice() {
        assert!(ReviewClassView::Advisory < ReviewClassView::Adversarial);
        assert_eq!(
            ReviewClassView::ALL.iter().min(),
            Some(&ReviewClassView::Advisory)
        );
    }

    #[test]
    fn both_values_round_trip_and_unknown_is_rejected() {
        for r in ReviewClassView::ALL {
            assert_eq!(ReviewClassView::parse(r.as_str()).unwrap(), r);
        }
        let rejected = ReviewClassView::parse("Adversarial").unwrap_err();
        assert_eq!(rejected.as_str(), "Adversarial");
    }
}
