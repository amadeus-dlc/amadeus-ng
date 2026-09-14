//! D2 の観測 — Claude 設定ファイルが語るフックの配線。

use super::{HookBindingDeclaration, HookBindingView, ObservationFailure, WiredHookView};

/// `settings.json` とその優先層から読んだフック配線の事実。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookWiringView {
    settings_present: bool,
    wired_hooks: Result<Vec<WiredHookView>, ObservationFailure>,
    hooks_disabled_by: Option<String>,
    managed_hooks_only: bool,
    bindings: Result<Vec<HookBindingView>, ObservationFailure>,
    native_hook_names: Vec<String>,
    declaration: HookBindingDeclaration,
}

impl HookWiringView {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(
        settings_present: bool,
        wired_hooks: Result<Vec<WiredHookView>, ObservationFailure>,
        hooks_disabled_by: Option<String>,
        managed_hooks_only: bool,
        bindings: Result<Vec<HookBindingView>, ObservationFailure>,
        native_hook_names: Vec<String>,
        declaration: HookBindingDeclaration,
    ) -> Self {
        Self {
            settings_present,
            wired_hooks,
            hooks_disabled_by,
            managed_hooks_only,
            bindings,
            native_hook_names,
            declaration,
        }
    }

    /// `.claude/settings.json` が存在するか。
    #[must_use]
    pub const fn settings_present(&self) -> bool {
        self.settings_present
    }

    /// 設定が名指す `aidlc-*.ts` (名前順・重複なし) と各実体の有無。読めなければ原因。
    pub const fn wired_hooks(&self) -> &Result<Vec<WiredHookView>, ObservationFailure> {
        &self.wired_hooks
    }

    /// `disableAllHooks: true` を明示した最上位の層 (無ければ有効)。
    #[must_use]
    pub fn hooks_disabled_by(&self) -> Option<&str> {
        self.hooks_disabled_by.as_deref()
    }

    /// 管理設定が `allowManagedHooksOnly=true` を明示しているか。
    #[must_use]
    pub const fn managed_hooks_only(&self) -> bool {
        self.managed_hooks_only
    }

    /// `hooks` ブロックの登録一覧 (JSON として読めなければ原因)。
    pub const fn bindings(&self) -> &Result<Vec<HookBindingView>, ObservationFailure> {
        &self.bindings
    }

    /// この build がフック面で受けるフック名。
    #[must_use]
    pub fn native_hook_names(&self) -> &[String] {
        &self.native_hook_names
    }

    /// 接続定義 (無い作業ツリーでは [`HookBindingDeclaration::Absent`])。
    #[must_use]
    pub const fn declaration(&self) -> &HookBindingDeclaration {
        &self.declaration
    }
}
