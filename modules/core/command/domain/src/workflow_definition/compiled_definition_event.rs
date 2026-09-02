//! `CompiledDefinitionEvent` — 集約 [`CompiledDefinition`] のドメインイベント。
//!
//! [`CompiledDefinition`]: super::CompiledDefinition

// 変種ペイロードは 1 ファイル 1 公開型で本ファイル同名のサブツリーに置き、ここで連鎖
// 再輸出する (所有サブツリーのファサード — coding-rules/module-visibility.md §追記
// 2026-09-01)。
use super::compiled_definition_event_id::CompiledDefinitionEventId;
use super::compiled_definition_id::CompiledDefinitionId;

mod compiled;
mod plugin_selection_applied;
mod recompiled;
mod scope_registered;

pub use compiled::Compiled;
pub use plugin_selection_applied::PluginSelectionApplied;
pub use recompiled::Recompiled;
pub use scope_registered::ScopeRegistered;

/// コンパイル済み定義 (配布束) に起きた事実。
///
/// 1 コマンド 1 イベント (`coding-rules/aggregate-commands.md`)。誕生の [`Compiled`] は
/// 内容そのものを運び ([`WorkflowDefinitionEvent::Defined`](super::WorkflowDefinitionEvent)
/// と同じ理由: 事実の自己完結な記録)、変異の 3 種はそれぞれの遷移が変えた分だけを運ぶ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompiledDefinitionEvent {
    /// 配布束がコンパイルされて存在するようになった (genesis)。
    Compiled(Compiled),
    /// 源が変わり、内容が新しいものへ入れ替わった。
    Recompiled(Recompiled),
    /// スコープが 1 つ登記された (identity + グリッド列)。
    ScopeRegistered(ScopeRegistered),
    /// プラグインの有効・無効の選択が適用された。
    PluginSelectionApplied(PluginSelectionApplied),
}

impl CompiledDefinitionEvent {
    /// このイベント自身の識別子 (全変種が持つ — イベントはエンティティ)。
    #[must_use]
    pub const fn id(&self) -> &CompiledDefinitionEventId {
        match self {
            CompiledDefinitionEvent::Compiled(payload) => payload.id(),
            CompiledDefinitionEvent::Recompiled(payload) => payload.id(),
            CompiledDefinitionEvent::ScopeRegistered(payload) => payload.id(),
            CompiledDefinitionEvent::PluginSelectionApplied(payload) => payload.id(),
        }
    }

    /// **どの集約の事実か** — 全変種が運ぶ配布束の識別子。
    #[must_use]
    pub const fn aggregate_id(&self) -> &CompiledDefinitionId {
        match self {
            CompiledDefinitionEvent::Compiled(payload) => payload.aggregate_id(),
            CompiledDefinitionEvent::Recompiled(payload) => payload.aggregate_id(),
            CompiledDefinitionEvent::ScopeRegistered(payload) => payload.aggregate_id(),
            CompiledDefinitionEvent::PluginSelectionApplied(payload) => payload.aggregate_id(),
        }
    }
}
