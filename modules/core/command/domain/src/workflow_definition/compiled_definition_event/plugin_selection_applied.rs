//! `PluginSelectionApplied` — プラグイン選択が配布束に適用されたイベントのペイロード。

use std::collections::BTreeSet;

use crate::workflow_definition::compiled_definition_id::CompiledDefinitionId;

/// プラグインの有効・無効の選択が適用された、という事実の材料。
///
/// 運ぶのは**有効にしたプラグン名の集合**である — 適用先のノードは配布束のグラフが
/// 知っているので、イベントは選択そのものだけを記録する (upstream `select-plugins` の
/// 意味論: 選択に無いプラグインのステージだけ `enabled: false` が立つ)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginSelectionApplied {
    id: CompiledDefinitionId,
    enabled_plugins: BTreeSet<String>,
}

impl PluginSelectionApplied {
    /// 材料をそのまま束ねる。
    #[must_use]
    pub const fn new(
        id: CompiledDefinitionId,
        enabled_plugins: BTreeSet<String>,
    ) -> PluginSelectionApplied {
        PluginSelectionApplied {
            id,
            enabled_plugins,
        }
    }

    /// 配布束の識別子。
    #[must_use]
    pub const fn id(&self) -> &CompiledDefinitionId {
        &self.id
    }

    /// 有効にしたプラグイン名の集合 (辞書順)。
    #[must_use]
    pub const fn enabled_plugins(&self) -> &BTreeSet<String> {
        &self.enabled_plugins
    }
}
