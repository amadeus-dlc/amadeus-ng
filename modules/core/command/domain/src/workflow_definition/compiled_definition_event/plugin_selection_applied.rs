//! `PluginSelectionApplied` — プラグイン選択が配布束に適用されたイベントのペイロード。

use std::collections::BTreeSet;

use crate::workflow_definition::compiled_definition_event_id::CompiledDefinitionEventId;
use crate::workflow_definition::compiled_definition_id::CompiledDefinitionId;

/// プラグインの有効・無効の選択が適用された、という事実の材料。
///
/// 運ぶのは**有効にしたプラグン名の集合**である — 適用先のノードは配布束のグラフが
/// 知っているので、イベントは選択そのものだけを記録する (upstream `select-plugins` の
/// 意味論: 選択に無いプラグインのステージだけ `enabled: false` が立つ)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginSelectionApplied {
    id: CompiledDefinitionEventId,
    aggregate_id: CompiledDefinitionId,
    enabled_plugins: BTreeSet<String>,
}

impl PluginSelectionApplied {
    /// イベント識別子・配布束の識別子と材料をそのまま束ねる。
    #[must_use]
    pub const fn new(
        id: CompiledDefinitionEventId,
        aggregate_id: CompiledDefinitionId,
        enabled_plugins: BTreeSet<String>,
    ) -> PluginSelectionApplied {
        PluginSelectionApplied {
            id,
            aggregate_id,
            enabled_plugins,
        }
    }

    /// このイベント自身の識別子 — ドメインイベントはエンティティの一種なので自前の id を
    /// 持つ (`coding-rules/domain-object-kinds.md`)。
    #[must_use]
    pub const fn id(&self) -> &CompiledDefinitionEventId {
        &self.id
    }

    /// **どの集約の事実か** — 配布束の識別子。
    #[must_use]
    pub const fn aggregate_id(&self) -> &CompiledDefinitionId {
        &self.aggregate_id
    }

    /// 有効にしたプラグイン名の集合 (辞書順)。
    #[must_use]
    pub const fn enabled_plugins(&self) -> &BTreeSet<String> {
        &self.enabled_plugins
    }
}
