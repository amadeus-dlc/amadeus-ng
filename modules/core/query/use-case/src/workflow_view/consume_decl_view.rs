//! `ConsumeDeclView` — 入力成果物の宣言 (`consumes[]`)。

use super::brownfield_greenfield_view::BrownfieldGreenfieldView;

/// 入力成果物の宣言。`required: false` は欠損しても無言で落ちる (12 §2.2 #15)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumeDeclView {
    artifact: String,
    required: bool,
    conditional_on: Option<BrownfieldGreenfieldView>,
}

impl ConsumeDeclView {
    /// 入力宣言 1 件を組む。`conditional_on` が `None` なら常に適用される宣言。
    #[must_use]
    pub fn new(
        artifact: impl Into<String>,
        required: bool,
        conditional_on: Option<BrownfieldGreenfieldView>,
    ) -> ConsumeDeclView {
        ConsumeDeclView {
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
    pub const fn conditional_on(&self) -> Option<BrownfieldGreenfieldView> {
        self.conditional_on
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_and_conditional_on_are_separate_axes() {
        let always = ConsumeDeclView::new("requirements", true, None);
        assert_eq!(always.artifact(), "requirements");
        assert!(always.required());
        assert_eq!(always.conditional_on(), None);

        let conditional = ConsumeDeclView::new(
            "legacy-survey",
            false,
            Some(BrownfieldGreenfieldView::Brownfield),
        );
        assert!(!conditional.required());
        assert_eq!(
            conditional.conditional_on(),
            Some(BrownfieldGreenfieldView::Brownfield)
        );
    }
}
