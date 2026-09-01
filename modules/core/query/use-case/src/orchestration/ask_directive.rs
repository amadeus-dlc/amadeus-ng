//! `AskDirective` — `ask` の中身 (構造化質問の提示)。
//!
//! エンジンは人間ターンを conductor へ委ねる。質問文は逐語で運び、`new-work-routing` の
//! 材料 (提案 scope と新規作業の記述) は伴わせる形にして「材料だけあって種別が違う」を
//! 作らない。

use super::ask_kind::AskKind;

/// `ask` — 構造化質問の提示 (エンジンは人間ターンを conductor へ委ねる)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskDirective {
    kind: AskKind,
    question: String,
    proposed_scope: Option<String>,
    new_work_description: Option<String>,
}

impl AskDirective {
    /// 質問を組む (基本コンストラクタ)。
    #[must_use]
    pub const fn new(kind: AskKind, question: String) -> AskDirective {
        AskDirective {
            kind,
            question,
            proposed_scope: None,
            new_work_description: None,
        }
    }

    /// `new-work-routing` の材料 (提案 scope と新規作業の記述) を伴う。
    #[must_use]
    pub fn with_new_work(
        mut self,
        proposed_scope: impl Into<String>,
        description: impl Into<String>,
    ) -> AskDirective {
        self.proposed_scope = Some(proposed_scope.into());
        self.new_work_description = Some(description.into());
        self
    }

    /// 質問の種別。
    #[must_use]
    pub const fn ask_kind(&self) -> AskKind {
        self.kind
    }

    /// 質問文 (逐語)。
    #[must_use]
    pub fn question(&self) -> &str {
        &self.question
    }

    /// 提案 scope (`new-work-routing` / scope 確認)。
    #[must_use]
    pub fn proposed_scope(&self) -> Option<&str> {
        self.proposed_scope.as_deref()
    }

    /// 新規作業の記述 (`new-work-routing`)。
    #[must_use]
    pub fn new_work_description(&self) -> Option<&str> {
        self.new_work_description.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ask_can_carry_the_new_work_material() {
        let ask = AskDirective::new(AskKind::NewWorkRouting, "route?".to_string())
            .with_new_work("bugfix", "fix the login crash");
        assert_eq!(ask.ask_kind(), AskKind::NewWorkRouting);
        assert_eq!(ask.question(), "route?");
        assert_eq!(ask.proposed_scope(), Some("bugfix"));
        assert_eq!(ask.new_work_description(), Some("fix the login crash"));
    }
}
