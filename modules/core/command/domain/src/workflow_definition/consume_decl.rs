//! `ConsumeDecl` — ステージが上流から受け取る成果物の宣言 1 件。

use super::brownfield_greenfield::BrownfieldGreenfield;

/// 入力成果物の宣言。`required: false` は欠損しても無言で落ちる (レポート §2.2 #15)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumeDecl {
    artifact: String,
    required: bool,
    conditional_on: Option<BrownfieldGreenfield>,
}

impl ConsumeDecl {
    /// 入力宣言 1 件を組む。`conditional_on` が `None` なら常に適用される宣言。
    #[must_use]
    pub fn new(
        artifact: impl Into<String>,
        required: bool,
        conditional_on: Option<BrownfieldGreenfield>,
    ) -> ConsumeDecl {
        ConsumeDecl {
            artifact: artifact.into(),
            required,
            conditional_on,
        }
    }

    /// 成果物の語彙名 (パスではない)。
    #[must_use]
    pub fn artifact(&self) -> &str {
        &self.artifact
    }

    /// `false` は欠損しても無言で落ちる。
    #[must_use]
    pub const fn required(&self) -> bool {
        self.required
    }

    /// この入力を要求するプロジェクト種別。`None` は種別を問わないという意味であって、
    /// 「不明」ではない。
    #[must_use]
    pub const fn conditional_on(&self) -> Option<BrownfieldGreenfield> {
        self.conditional_on
    }
}
