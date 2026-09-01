//! `RuleInContext` — ステージ実行時に文脈へ載る規則 1 件の参照。

use super::rule_scope::RuleScope;

/// compile 時に確定した rules-in-context の 1 エントリ。
///
/// `path` は `default` スペースに**コンパイル時ピン**されており、実行時に再解決しては
/// ならない (レポート §6.1-18)。active space への rebase は別経路の責務。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleInContext {
    path: String,
    scope: RuleScope,
}

impl RuleInContext {
    /// compile が解決した 1 行を組む。`path` はピン済みの値をそのまま渡す (ここで解決しない)。
    #[must_use]
    pub fn new(path: impl Into<String>, scope: RuleScope) -> RuleInContext {
        RuleInContext {
            path: path.into(),
            scope,
        }
    }

    /// ルールファイルのパス。`default` スペースへのコンパイル時ピンであり、
    /// active space での配送パスへの上書きは transport 側の責務。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// このルールがどの層のものか。strict-additive なので層で落とされることはなく、
    /// 解決済みの行はすべて適用される。
    #[must_use]
    pub const fn scope(&self) -> RuleScope {
        self.scope
    }
}
