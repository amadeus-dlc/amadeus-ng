//! `ReviewCapValue` — scope frontmatter の `review_cap:` の値 (3 値の閉集合)。

use super::unknown_review_cap::UnknownReviewCap;

/// `review_cap:` の値。`None` は「`none` と**宣言された**」ことを表す値であり、
/// 「宣言が無い」は `Option<ReviewCapValue>` の `None` 側で表す (2 つを混同しないこと)。
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
