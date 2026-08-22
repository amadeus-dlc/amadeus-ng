//! `ScopeMetadata` — `<harness>/scopes/aidlc-<name>.md` の frontmatter 最小モデル。
//!
//! **有効スコープの権威はこのファイルの存在**であってグリッドではない
//! (`validScopes()` — レポート §4.6)。深さ・キーワードなどはグリッドではなくここに入る
//! (レポート §3.1)。

/// `skeleton:` の既定値。scope frontmatter では `on` / `off` の 2 値のみ
/// (orchestration の `SkeletonStance` が持つ `scope-dependent` はここには現れない)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SkeletonDefault {
    /// スコープ既定で walking skeleton Bolt を先に走らせる (upstream 01 §6.4)。
    On,
    /// スコープ既定では walking skeleton Bolt を走らせない。
    Off,
}

/// 閉集合外の値 (upstream: `has invalid skeleton value "..." . Expected "on" or "off".`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownSkeletonDefault(String);

impl UnknownSkeletonDefault {
    /// 拒否された生値をそのまま包む。トリムも小文字化もしない。
    #[must_use]
    pub fn new(value: impl Into<String>) -> UnknownSkeletonDefault {
        UnknownSkeletonDefault(value.into())
    }

    /// 拒否された生値を逐語で持ち帰る (文言化は Presenter 側の責務)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl SkeletonDefault {
    /// 宣言順の全値 (2 値の網羅走査の正本)。
    pub const ALL: [SkeletonDefault; 2] = [SkeletonDefault::On, SkeletonDefault::Off];

    /// # Errors
    ///
    /// `on` / `off` 以外は `UnknownSkeletonDefault` で拒否する。
    pub fn parse(s: &str) -> Result<SkeletonDefault, UnknownSkeletonDefault> {
        Ok(match s {
            "on" => SkeletonDefault::On,
            "off" => SkeletonDefault::Off,
            other => return Err(UnknownSkeletonDefault::new(other)),
        })
    }

    /// frontmatter 上の正準綴り (`parse` の逆写像)。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            SkeletonDefault::On => "on",
            SkeletonDefault::Off => "off",
        }
    }
}

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

/// 閉集合外の値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownReviewCap(String);

impl UnknownReviewCap {
    /// 拒否された生値をそのまま包む。空文字列も既定へ畳まずそのまま保つ。
    #[must_use]
    pub fn new(value: impl Into<String>) -> UnknownReviewCap {
        UnknownReviewCap(value.into())
    }

    /// 拒否された生値を逐語で持ち帰る (文言化は Presenter 側の責務)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
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

/// スコープ identity ファイルの frontmatter (最小モデル)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeMetadata {
    name: String,
    depth: Option<String>,
    keywords: Vec<String>,
    skeleton: Option<SkeletonDefault>,
    review_cap: Option<ReviewCapValue>,
    freeform_default: bool,
}

/// `ScopeMetadata` の構成が拒否される理由。frontmatter の必須キー欠落のみ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeMetadataError {
    /// `name:` が無い / 空 (upstream: `missing required frontmatter: name`)。
    MissingName,
}

impl ScopeMetadata {
    /// `name` だけを持つ最小のメタデータ。残りは `with_*` で積む。
    ///
    /// # Errors
    ///
    /// `name` が空 (空白のみを含む) なら `MissingName`。
    pub fn new(name: &str) -> Result<ScopeMetadata, ScopeMetadataError> {
        if name.trim().is_empty() {
            return Err(ScopeMetadataError::MissingName);
        }
        Ok(ScopeMetadata {
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
    pub fn with_depth(mut self, depth: String) -> ScopeMetadata {
        self.depth = Some(depth);
        self
    }

    /// `keywords:` を載せる。スコープ推論の材料であり、空のままなら推論では選ばれない
    /// (名指しでのみ選べる — upstream 01 §5.5)。
    #[must_use]
    pub fn with_keywords(mut self, keywords: Vec<String>) -> ScopeMetadata {
        self.keywords = keywords;
        self
    }

    /// `skeleton:` を載せる。呼ばなければ「宣言なし」のまま (`off` へ畳まない)。
    #[must_use]
    pub const fn with_skeleton(mut self, skeleton: SkeletonDefault) -> ScopeMetadata {
        self.skeleton = Some(skeleton);
        self
    }

    /// `review_cap:` を載せる。`ReviewCapValue::None` を渡すのは「`none` と**宣言された**」
    /// ことであり、呼ばないこと (= 宣言なし) とは別の値である。
    #[must_use]
    pub const fn with_review_cap(mut self, review_cap: ReviewCapValue) -> ScopeMetadata {
        self.review_cap = Some(review_cap);
        self
    }

    /// 有効スコープ中で `true` を持てるのは 1 つまで (upstream の frontmatter 検証)。
    /// 集合レベルの検証は呼出側 (Gateway / compile) の責務で、ここでは値を保持するだけ。
    #[must_use]
    pub const fn with_freeform_default(mut self, freeform_default: bool) -> ScopeMetadata {
        self.freeform_default = freeform_default;
        self
    }

    /// スコープの識別子 (`name:`)。有効スコープ集合を作る軸であり、2 ファイル間の重複は致命。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 深さダイヤル。値域は workflow-definition の別語彙のため、ここでは文字列で保持する。
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
    pub const fn skeleton(&self) -> Option<SkeletonDefault> {
        self.skeleton
    }

    /// ワークフロー全体に効くレビュー重量の上限。外側の `None` は「宣言が無い」、
    /// `Some(ReviewCapValue::None)` は「`none` と宣言された」。
    #[must_use]
    pub const fn review_cap(&self) -> Option<ReviewCapValue> {
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
    use proptest::prelude::*;

    #[test]
    fn name_is_the_only_required_field() {
        let m = ScopeMetadata::new("feature").unwrap();
        assert_eq!(m.name(), "feature");
        assert_eq!(m.depth(), None);
        assert!(m.keywords().is_empty());
        assert_eq!(m.skeleton(), None);
        assert_eq!(m.review_cap(), None);
        assert!(!m.freeform_default());
    }

    #[test]
    fn a_missing_name_is_refused() {
        assert_eq!(ScopeMetadata::new(""), Err(ScopeMetadataError::MissingName));
        assert_eq!(
            ScopeMetadata::new("   "),
            Err(ScopeMetadataError::MissingName)
        );
    }

    #[test]
    fn declared_none_review_cap_differs_from_an_absent_declaration() {
        let declared = ScopeMetadata::new("poc")
            .unwrap()
            .with_review_cap(ReviewCapValue::None);
        let absent = ScopeMetadata::new("poc").unwrap();
        assert_eq!(declared.review_cap(), Some(ReviewCapValue::None));
        assert_eq!(absent.review_cap(), None);
        assert_ne!(declared, absent);
    }

    #[test]
    fn skeleton_and_review_cap_are_closed_sets() {
        for s in SkeletonDefault::ALL {
            assert_eq!(SkeletonDefault::parse(s.as_str()).unwrap(), s);
        }
        // 閉集合外は生値を逐語で持ち帰る (upstream 文言 `has invalid skeleton value "..."` の材料)
        let rejected = SkeletonDefault::parse("maybe").unwrap_err();
        assert_eq!(rejected.as_str(), "maybe");
        assert_eq!(rejected, UnknownSkeletonDefault::new("maybe"));
        for r in ReviewCapValue::ALL {
            assert_eq!(ReviewCapValue::parse(r.as_str()).unwrap(), r);
        }
        // 空文字も閉集合外。空のまま逐語保持する (既定値へフォールスルーさせない)
        let rejected = ReviewCapValue::parse("").unwrap_err();
        assert_eq!(rejected.as_str(), "");
        assert_eq!(rejected, UnknownReviewCap::new(""));
    }

    proptest! {
        /// `with_*` は指定したフィールドだけを差し替える。
        #[test]
        fn builders_are_independent(
            name in "[a-z][a-z-]{0,10}",
            depth in "[a-z]{1,8}",
            keywords in proptest::collection::vec("[a-z]{1,6}", 0..5),
            freeform in any::<bool>(),
        ) {
            let m = ScopeMetadata::new(&name)
                .unwrap()
                .with_depth(depth.clone())
                .with_keywords(keywords.clone())
                .with_freeform_default(freeform);
            prop_assert_eq!(m.name(), name.as_str());
            prop_assert_eq!(m.depth(), Some(depth.as_str()));
            prop_assert_eq!(m.keywords(), keywords.as_slice());
            prop_assert_eq!(m.freeform_default(), freeform);
            prop_assert_eq!(m.skeleton(), None);
            prop_assert_eq!(m.review_cap(), None);
        }

        /// 空白以外の文字を含む name は常に受理される。
        #[test]
        fn any_non_blank_name_is_accepted(name in "[!-~]{1,20}") {
            let m = ScopeMetadata::new(&name).unwrap();
            prop_assert_eq!(m.name(), name.as_str());
        }
    }
}
