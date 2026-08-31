//! `RuleInContextView` — compile 時に確定した rules-in-context の 1 エントリ。

use super::rule_scope_view::RuleScopeView;

/// 解決済みのルール行。
///
/// `path` は `default` スペースに**コンパイル時ピン**されており、実行時に再解決しては
/// ならない (12 §6.1-18)。active space への rebase は別経路の責務。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleInContextView {
    path: String,
    scope: RuleScopeView,
}

impl RuleInContextView {
    /// compile が解決した 1 行を組む。`path` はピン済みの値をそのまま渡す (ここで解決しない)。
    #[must_use]
    pub fn new(path: impl Into<String>, scope: RuleScopeView) -> RuleInContextView {
        RuleInContextView {
            path: path.into(),
            scope,
        }
    }

    /// ルールファイルのパス。`default` スペースへのコンパイル時ピンである。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// このルールがどの層のものか。strict-additive なので層で落とされることはない。
    #[must_use]
    pub const fn scope(&self) -> RuleScopeView {
        self.scope
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pinned_path_and_its_layer_are_read_back_verbatim() {
        let rule = RuleInContextView::new("aidlc/spaces/default/memory/org.md", RuleScopeView::Org);
        assert_eq!(rule.path(), "aidlc/spaces/default/memory/org.md");
        assert_eq!(rule.scope(), RuleScopeView::Org);
    }
}
