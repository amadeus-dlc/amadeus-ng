//! `ReviewCapValueView` — scope frontmatter の `review_cap:` (3 値)。

use super::unknown_value::UnknownValue;

/// レビュー重量の上限。`None` 変種は「`none` と**宣言された**」ことを表す値であり、
/// 「宣言が無い」は `Option<ReviewCapValueView>` の `None` 側で表す (2 つを混同しないこと)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReviewCapValueView {
    /// 上限を課さない。ステージ宣言のレビュー重量がそのまま通る (upstream 01 §6.2)。
    Adversarial,
    /// adversarial なステージを advisory 1 パスへ格下げする。
    Advisory,
    /// レビュアーのディスパッチ自体を無効化する。
    None,
}

impl ReviewCapValueView {
    /// 宣言順の全値 (3 値の網羅走査の正本)。並びは上限の緩い側から厳しい側へ。
    pub const ALL: [ReviewCapValueView; 3] = [
        ReviewCapValueView::Adversarial,
        ReviewCapValueView::Advisory,
        ReviewCapValueView::None,
    ];

    /// # Errors
    ///
    /// `adversarial` / `advisory` / `none` 以外は [`UnknownValue`] で拒否する。
    pub fn parse(s: &str) -> Result<ReviewCapValueView, UnknownValue> {
        Ok(match s {
            "adversarial" => ReviewCapValueView::Adversarial,
            "advisory" => ReviewCapValueView::Advisory,
            "none" => ReviewCapValueView::None,
            other => return Err(UnknownValue::new(other)),
        })
    }

    /// frontmatter 上の正準綴り (`parse` の逆写像)。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ReviewCapValueView::Adversarial => "adversarial",
            ReviewCapValueView::Advisory => "advisory",
            ReviewCapValueView::None => "none",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_declared_values_round_trip_and_the_rest_are_rejected() {
        for c in ReviewCapValueView::ALL {
            assert_eq!(ReviewCapValueView::parse(c.as_str()).unwrap(), c);
        }
        let rejected = ReviewCapValueView::parse("strict").unwrap_err();
        assert_eq!(rejected.as_str(), "strict");
    }
}
