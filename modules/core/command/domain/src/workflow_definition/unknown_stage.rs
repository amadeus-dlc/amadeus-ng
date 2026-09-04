//! `UnknownStage` — 定義がその slug を知らないという拒否が運ぶ材料。

/// `stage-graph.json` に載っていないステージ slug。
///
/// upstream `handleReview` は `loadStageGraphAll().find(...)` が空振りしたときと
/// `reviewer` 宣言が無いときを**同じ文言**（`Cannot record review: stage "<slug>" has no
/// declared reviewer.`）で断るが、材料としては別物である — 「定義がその slug を知らない」と
/// 「知っているがレビュアーを宣言していない」を混ぜない（文言化は出す側の責務）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownStage {
    slug: String,
}

impl UnknownStage {
    /// 拒否された slug を束ねる（生値のまま保持する）。
    #[must_use]
    pub fn new(slug: impl Into<String>) -> UnknownStage {
        UnknownStage { slug: slug.into() }
    }

    /// 定義に無かった slug。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.slug
    }
}
