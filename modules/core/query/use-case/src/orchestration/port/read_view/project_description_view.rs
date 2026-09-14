//! `ProjectDescriptionView` — 依頼原文の正本 1 件の写し (`project-description` が描く 2 列)。
//!
//! 状態ファイルとサイドカーはどちらも upstream 互換の人間可読リードモデルであり、クエリ側が
//! 読んで写す (`coding-rules/cqrs-boundaries.md` 規則 6/7)。1 件は 2 つの読取源を
//! **`Project Description Source` の綴りで選び分けて**組んだもので、組むのはユースケースの
//! 仕事である。
//!
//! 表現は隠し、契約 (アクセサ) だけを公開する (`coding-rules/field-visibility.md`)。

/// 依頼原文の正本と、その出所。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDescriptionView {
    description: String,
    source: String,
}

impl ProjectDescriptionView {
    /// 2 列をそのまま束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(description: String, source: String) -> ProjectDescriptionView {
        ProjectDescriptionView {
            description,
            source,
        }
    }

    /// 依頼原文（逐語）。
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// 出所の綴り（`project-description.json` か `aidlc-state.md#Project`）。
    ///
    /// どちらも upstream の**逐語トークン**であり、1 バイトも変えられない。
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
}
