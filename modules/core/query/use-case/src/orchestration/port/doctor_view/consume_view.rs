//! 配布グラフのステージが消費する成果物 1 件の写し。

/// `consumes[]` の 1 要素。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumeView {
    artifact: String,
    required: bool,
    conditional_on: Option<String>,
}

impl ConsumeView {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(artifact: String, required: bool, conditional_on: Option<String>) -> Self {
        Self {
            artifact,
            required,
            conditional_on,
        }
    }

    /// 成果物名。
    #[must_use]
    pub fn artifact(&self) -> &str {
        &self.artifact
    }

    /// 必須か。
    #[must_use]
    pub const fn required(&self) -> bool {
        self.required
    }

    /// 条件 (`brownfield` / `greenfield`)。
    #[must_use]
    pub fn conditional_on(&self) -> Option<&str> {
        self.conditional_on.as_deref()
    }
}
