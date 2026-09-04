//! `UnknownReviewVerdict` — 閉集合外のレビュー判定が運ぶ材料。

/// `READY` / `NOT-READY` の外の生値。文言化（`Unknown --verdict "<v>". Accepted: …`）は
/// 出す側の責務なので、ここは拒否された綴りだけを運ぶ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownReviewVerdict {
    given: String,
}

impl UnknownReviewVerdict {
    /// 拒否された生値を束ねる。
    #[must_use]
    pub fn new(given: impl Into<String>) -> UnknownReviewVerdict {
        UnknownReviewVerdict {
            given: given.into(),
        }
    }

    /// 拒否された生値（**正規化前**の綴りをそのまま返す）。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.given
    }
}
