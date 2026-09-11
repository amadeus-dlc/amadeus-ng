//! レビュー要求で観測した成果物の原文。
/// ファイルI/Oは境界が行う。Noneは欠落、regular=falseは通常ファイル以外を表す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewArtifact {
    path: String,
    body: Option<Vec<u8>>,
    regular: bool,
    required: bool,
    appendix_target: bool,
}
impl ReviewArtifact {
    /// 宣言と実ファイルの観測を束ねる。
    #[must_use]
    pub const fn new(
        path: String,
        body: Option<Vec<u8>>,
        regular: bool,
        required: bool,
        appendix_target: bool,
    ) -> Self {
        Self {
            path,
            body,
            regular,
            required,
            appendix_target,
        }
    }
    /// 記録相対の論理パス。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }
    /// 観測した原文。
    #[must_use]
    pub fn body(&self) -> Option<&[u8]> {
        self.body.as_deref()
    }
    /// 通常ファイルか。
    #[must_use]
    pub const fn is_regular(&self) -> bool {
        self.regular
    }
    /// 宣言上の必須成果物か。
    #[must_use]
    pub const fn is_required(&self) -> bool {
        self.required
    }
    /// Review追記先か。
    #[must_use]
    pub const fn is_appendix_target(&self) -> bool {
        self.appendix_target
    }
}
