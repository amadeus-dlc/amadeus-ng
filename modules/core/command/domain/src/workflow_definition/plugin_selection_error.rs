//! `PluginSelectionError` — `CompiledDefinition::apply_plugin_selection` のガードが拒否する形。

use std::fmt;

/// プラグイン選択を受け付けられない形 (材料のみ — 利用者向け文言はアダプタ層)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginSelectionError {
    /// 選択が、どのステージも宣言していないプラグインを名指している。
    UnknownPlugin {
        /// 宣言されていないプラグイン名。
        name: String,
    },
    /// 選択を適用してもグラフが変わらない — 書くべき事実が無い。
    Unchanged,
}

impl fmt::Display for PluginSelectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginSelectionError::UnknownPlugin { name } => write!(f, "unknown plugin {name}"),
            PluginSelectionError::Unchanged => f.write_str("plugin selection unchanged"),
        }
    }
}

impl std::error::Error for PluginSelectionError {}
