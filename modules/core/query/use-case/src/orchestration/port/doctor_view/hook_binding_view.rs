//! `settings.json` のフック登録 1 件の写し。

use super::HookBindingTarget;

/// イベント・matcher・コマンドと、その呼出し先の分類。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookBindingView {
    event: String,
    matcher: String,
    command: String,
    target: HookBindingTarget,
}

impl HookBindingView {
    /// 登録 1 件を束ねる。
    #[must_use]
    pub const fn new(
        event: String,
        matcher: String,
        command: String,
        target: HookBindingTarget,
    ) -> Self {
        Self {
            event,
            matcher,
            command,
            target,
        }
    }

    /// フックイベント名 (`PreToolUse` 等)。
    #[must_use]
    pub fn event(&self) -> &str {
        &self.event
    }

    /// matcher (空なら全ツール)。
    #[must_use]
    pub fn matcher(&self) -> &str {
        &self.matcher
    }

    /// 登録されたコマンド行そのもの。
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }

    /// 呼出し先の分類。
    #[must_use]
    pub const fn target(&self) -> &HookBindingTarget {
        &self.target
    }
}
