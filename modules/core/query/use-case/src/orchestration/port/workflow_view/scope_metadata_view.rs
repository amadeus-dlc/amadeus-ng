//! `ScopeMetadataView` — `<harness>/scopes/aidlc-<name>.md` の frontmatter 最小ビュー。
//!
//! **有効スコープの権威はこのファイルの存在**であってグリッドではない (`validScopes()` —
//! 12 §4.6)。深さ・キーワードなどはグリッドではなくここに入る (12 §3.1)。

use super::review_cap_value_view::ReviewCapValueView;
use super::scope_metadata_error::ScopeMetadataError;
use super::skeleton_default_view::SkeletonDefaultView;

/// スコープ identity ファイルの frontmatter (最小ビュー)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeMetadataView {
    name: String,
    depth: Option<String>,
    keywords: Vec<String>,
    skeleton: Option<SkeletonDefaultView>,
    review_cap: Option<ReviewCapValueView>,
    freeform_default: bool,
}

impl ScopeMetadataView {
    /// `name` だけを持つ最小のメタデータ。残りは `with_*` で積む。
    ///
    /// # Errors
    ///
    /// `name` が空 (空白のみを含む) なら `MissingName`。
    pub fn new(name: &str) -> Result<ScopeMetadataView, ScopeMetadataError> {
        if name.trim().is_empty() {
            return Err(ScopeMetadataError::MissingName);
        }
        Ok(ScopeMetadataView {
            name: name.to_string(),
            depth: None,
            keywords: Vec::new(),
            skeleton: None,
            review_cap: None,
            freeform_default: false,
        })
    }

    /// `depth:` を載せる。成果物の詳細度に効く助言軸であって、どのステージが走るかには
    /// 影響しない (12 §2.2)。
    #[must_use]
    pub fn with_depth(mut self, depth: String) -> ScopeMetadataView {
        self.depth = Some(depth);
        self
    }

    /// `keywords:` を載せる。スコープ推論の材料であり、空のままなら推論では選ばれない。
    #[must_use]
    pub fn with_keywords(mut self, keywords: Vec<String>) -> ScopeMetadataView {
        self.keywords = keywords;
        self
    }

    /// `skeleton:` を載せる。呼ばなければ「宣言なし」のまま (`off` へ畳まない)。
    #[must_use]
    pub const fn with_skeleton(mut self, skeleton: SkeletonDefaultView) -> ScopeMetadataView {
        self.skeleton = Some(skeleton);
        self
    }

    /// `review_cap:` を載せる。`ReviewCapValueView::None` を渡すのは「`none` と**宣言された**」
    /// ことであり、呼ばないこと (= 宣言なし) とは別の値である。
    #[must_use]
    pub const fn with_review_cap(mut self, review_cap: ReviewCapValueView) -> ScopeMetadataView {
        self.review_cap = Some(review_cap);
        self
    }

    /// 有効スコープ中で `true` を持てるのは 1 つまで (upstream の frontmatter 検証)。
    /// 集合レベルの検証は呼出側の責務で、ここでは値を保持するだけ。
    #[must_use]
    pub const fn with_freeform_default(mut self, freeform_default: bool) -> ScopeMetadataView {
        self.freeform_default = freeform_default;
        self
    }

    /// スコープの識別子 (`name:`)。有効スコープ集合を作る軸であり、2 ファイル間の重複は致命。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 深さダイヤル。値域は別語彙のため、ここでは文字列で保持する。
    #[must_use]
    pub fn depth(&self) -> Option<&str> {
        self.depth.as_deref()
    }

    /// スコープ推論に使う語。空はスコープを無効にはせず、「推論では選ばれない」を意味する。
    #[must_use]
    pub fn keywords(&self) -> &[String] {
        &self.keywords
    }

    /// スコープ既定の walking-skeleton 姿勢。`None` は「宣言が無い」であって `off` ではない。
    #[must_use]
    pub const fn skeleton(&self) -> Option<SkeletonDefaultView> {
        self.skeleton
    }

    /// レビュー重量の上限。外側の `None` は「宣言が無い」、`Some(ReviewCapValueView::None)` は
    /// 「`none` と宣言された」。
    #[must_use]
    pub const fn review_cap(&self) -> Option<ReviewCapValueView> {
        self.review_cap
    }

    /// 既定スコープ解決のフォールバック先に自ら名乗り出るか (upstream 01 §6.3)。欠損は `false`。
    #[must_use]
    pub const fn freeform_default(&self) -> bool {
        self.freeform_default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_is_required_and_everything_else_starts_undeclared() {
        let metadata = ScopeMetadataView::new("feature").unwrap();
        assert_eq!(metadata.name(), "feature");
        assert_eq!(metadata.depth(), None);
        assert!(metadata.keywords().is_empty());
        assert_eq!(metadata.skeleton(), None);
        assert_eq!(metadata.review_cap(), None);
        assert!(!metadata.freeform_default());
    }

    #[test]
    fn a_blank_name_cannot_be_constructed() {
        assert_eq!(
            ScopeMetadataView::new(""),
            Err(ScopeMetadataError::MissingName)
        );
        assert_eq!(
            ScopeMetadataView::new("  \t "),
            Err(ScopeMetadataError::MissingName)
        );
        assert_eq!(
            ScopeMetadataError::MissingName.to_string(),
            "missing required frontmatter: name"
        );
    }

    #[test]
    fn the_optional_frontmatter_keys_stack_up_without_disturbing_each_other() {
        let metadata = ScopeMetadataView::new("feature")
            .unwrap()
            .with_depth("standard".to_string())
            .with_keywords(vec!["api".to_string(), "endpoint".to_string()])
            .with_skeleton(SkeletonDefaultView::On)
            .with_review_cap(ReviewCapValueView::None)
            .with_freeform_default(true);
        assert_eq!(metadata.name(), "feature");
        assert_eq!(metadata.depth(), Some("standard"));
        assert_eq!(
            metadata.keywords(),
            ["api".to_string(), "endpoint".to_string()]
        );
        assert_eq!(metadata.skeleton(), Some(SkeletonDefaultView::On));
        // 「`none` と宣言された」は「宣言が無い」とは別の値である。
        assert_eq!(metadata.review_cap(), Some(ReviewCapValueView::None));
        assert!(metadata.freeform_default());
    }
}
