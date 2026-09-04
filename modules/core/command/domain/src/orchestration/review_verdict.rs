//! `ReviewVerdict` — レビュアーが返す判定の 2 値（upstream `VALID_VERDICTS`）。

use super::unknown_review_verdict::UnknownReviewVerdict;

/// レビュアーの判定。`aidlc-log review --verdict <READY|NOT-READY>` の値であり、
/// 監査行 `REVIEW_COMPLETED` の `**Verdict**:` 欄に載る。
///
/// report の結末を表す [`Verdict`] とは**別物**である — あちらはステージ遷移の語
/// （`approved` / `rejected` / …）で、こちらはレビュアーの所見の語である。
///
/// [`Verdict`]: super::verdict::Verdict
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReviewVerdict {
    /// そのステージの成果物はゲートへ出せる。
    Ready,
    /// まだ出せない。反駁ループが残っていれば lead が直して再依頼する。
    NotReady,
}

impl ReviewVerdict {
    /// 宣言順の全値（2 値の網羅走査の正本）。
    pub const ALL: [ReviewVerdict; 2] = [ReviewVerdict::Ready, ReviewVerdict::NotReady];

    /// 閉集合パース。**大小は無視する** — upstream は `flags.verdict.toUpperCase()` の
    /// 結果を集合と照合するので `ready` も `Ready` も通る（ピン `3c3146cf` `:1129-1134`）。
    ///
    /// # Errors
    ///
    /// `READY` / `NOT-READY` 以外は `UnknownReviewVerdict` で拒否する（生値を持ち帰る）。
    pub fn parse(s: &str) -> Result<ReviewVerdict, UnknownReviewVerdict> {
        match s.to_uppercase().as_str() {
            "READY" => Ok(ReviewVerdict::Ready),
            "NOT-READY" => Ok(ReviewVerdict::NotReady),
            _ => Err(UnknownReviewVerdict::new(s)),
        }
    }

    /// 監査行に載る正準綴り（`parse` の逆写像）。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ReviewVerdict::Ready => "READY",
            ReviewVerdict::NotReady => "NOT-READY",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_values_round_trip() {
        for verdict in ReviewVerdict::ALL {
            assert_eq!(ReviewVerdict::parse(verdict.as_str()).unwrap(), verdict);
        }
    }

    /// upstream は `toUpperCase()` してから閉集合に当てるので、小文字も通る。
    #[test]
    fn the_closed_set_is_matched_after_upcasing() {
        assert_eq!(ReviewVerdict::parse("ready").unwrap(), ReviewVerdict::Ready);
        assert_eq!(
            ReviewVerdict::parse("not-ready").unwrap(),
            ReviewVerdict::NotReady
        );
        assert_eq!(
            ReviewVerdict::parse("Not-Ready").unwrap(),
            ReviewVerdict::NotReady
        );
    }

    /// 閉集合の外は**生値のまま**持ち帰る（文言は出す側が組む）。
    #[test]
    fn anything_outside_the_closed_set_is_rejected_with_the_raw_value() {
        let rejected = ReviewVerdict::parse("maybe").unwrap_err();
        assert_eq!(rejected.as_str(), "maybe");
        assert_eq!(rejected, UnknownReviewVerdict::new("maybe"));
        // `NOT READY` (ハイフン無し) も外である。
        assert!(ReviewVerdict::parse("NOT READY").is_err());
    }
}
