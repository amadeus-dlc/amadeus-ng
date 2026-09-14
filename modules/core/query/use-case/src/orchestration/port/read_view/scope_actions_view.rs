//! `ScopeActionsView` — `scope-grid.json` の 1 scope 分の EXECUTE/SKIP 割当の写し。
//!
//! `scope-grid.json` は compile コンテキストの投影 (リードモデル) であり、クエリ側が読んで
//! 写す (`coding-rules/cqrs-boundaries.md` 規則 7)。行の綴りは「どの slug がこの scope で
//! EXECUTE か」であり、その事実だけを運ぶ。数える・歩くのは読み手 (ユースケース) の仕事で、
//! この写しは行に無い事実を作らない。
//!
//! 表現 (map) は隠し、契約 (アクセサ) だけを公開する (`coding-rules/field-visibility.md`)。

use std::collections::BTreeMap;

/// 1 scope 分のステージ割当 (slug → `EXECUTE` / `SKIP`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeActionsView {
    scope: String,
    actions: BTreeMap<String, String>,
}

impl ScopeActionsView {
    /// scope 名と割当表をそのまま束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(scope: String, actions: BTreeMap<String, String>) -> ScopeActionsView {
        ScopeActionsView { scope, actions }
    }

    /// scope 名。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// 与えた slug のこの scope での割当 (`EXECUTE` / `SKIP`、無ければ `None`)。
    #[must_use]
    pub fn action_of(&self, slug: &str) -> Option<&str> {
        self.actions.get(slug).map(String::as_str)
    }

    /// EXECUTE のステージ数。
    #[must_use]
    pub fn execute_count(&self) -> u32 {
        u32::try_from(
            self.actions
                .values()
                .filter(|action| *action == "EXECUTE")
                .count(),
        )
        .unwrap_or(u32::MAX)
    }

    /// グリッドに載るステージ総数 (EXECUTE + SKIP)。
    #[must_use]
    pub fn total_count(&self) -> u32 {
        u32::try_from(self.actions.len()).unwrap_or(u32::MAX)
    }
}
