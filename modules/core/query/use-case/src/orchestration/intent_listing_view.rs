//! `IntentListingView` — 一覧と、活動中の記録ディレクトリ名を束ねた組み立て View。

use crate::orchestration::IntentListingRowView;

/// 一覧と、活動中の記録ディレクトリ名。
///
/// 行ごとの `active` 真偽は**持たない** — `directory()` と [`Self::active`] の比較で導ける
/// 値を二重に持つと、両者が食い違いうるからである。導出は [`Self::is_active`] 1 箇所に置き、
/// 出す側がそれを呼ぶ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentListingView {
    active: Option<String>,
    intents: Vec<IntentListingRowView>,
}

impl IntentListingView {
    /// 一覧と活動中の記録ディレクトリ名を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        active: Option<String>,
        intents: Vec<IntentListingRowView>,
    ) -> IntentListingView {
        IntentListingView { active, intents }
    }

    /// 活動中の記録ディレクトリ名。カーソルがどの記録も指していなければ `None`。
    #[must_use]
    pub fn active(&self) -> Option<&str> {
        self.active.as_deref()
    }

    /// 一覧の行。
    #[must_use]
    pub fn intents(&self) -> &[IntentListingRowView] {
        &self.intents
    }

    /// この行が活動中か。
    #[must_use]
    pub fn is_active(&self, row: &IntentListingRowView) -> bool {
        row.directory().is_some() && row.directory() == self.active()
    }
}
