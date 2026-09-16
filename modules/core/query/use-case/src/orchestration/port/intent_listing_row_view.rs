//! `IntentListingRowView` — 依頼一覧の 1 行。
//!
//! # 活動中の印は持たない
//!
//! どの記録が活動中かはカーソルが決めるので、境界で解決して上位から渡す
//! (`coding-rules/tell-dont-ask.md`)。行が自分で `active` を持つと、同じ事実が行ごとと
//! 一覧全体の 2 箇所に現れて食い違いうる。

/// 依頼一覧の 1 行 (登録簿の行、または登録簿に無い記録ディレクトリ)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentListingRowView {
    uuid: String,
    slug: String,
    status: String,
    repos: Vec<String>,
    directory: Option<String>,
}

impl IntentListingRowView {
    /// 行の 5 つの値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        uuid: String,
        slug: String,
        status: String,
        repos: Vec<String>,
        directory: Option<String>,
    ) -> IntentListingRowView {
        IntentListingRowView {
            uuid,
            slug,
            status,
            repos,
            directory,
        }
    }

    /// 登録簿が持つ識別子。登録簿に行が無い記録は空文字である (upstream 逐語)。
    #[must_use]
    pub fn uuid(&self) -> &str {
        &self.uuid
    }

    /// 表示用の短い名前。
    #[must_use]
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// 登録簿が持つ状態。登録簿に行が無い記録は `unknown` である (upstream 逐語)。
    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }

    /// 登録された repo 識別子。
    #[must_use]
    pub fn repos(&self) -> &[String] {
        &self.repos
    }

    /// 対応する記録ディレクトリ名。**登録簿に在るのに記録が無い**行は `None` である。
    #[must_use]
    pub fn directory(&self) -> Option<&str> {
        self.directory.as_deref()
    }
}
