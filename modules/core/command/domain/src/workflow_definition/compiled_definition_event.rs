//! `CompiledDefinitionEvent` — 集約 [`CompiledDefinition`] のドメインイベント。
//!
//! [`CompiledDefinition`]: super::CompiledDefinition

// 変種ペイロードは 1 ファイル 1 公開型で本ファイル同名のサブツリーに置き、ここで連鎖
// 再輸出する (所有サブツリーのファサード — coding-rules/module-visibility.md §追記
// 2026-09-01)。
mod compiled;

pub use compiled::Compiled;

/// コンパイル済み定義 (配布束) に起きた事実。
///
/// 現状の変種は genesis の [`Compiled`] 1 種 — 内容そのものを運ぶ
/// ([`WorkflowDefinitionEvent::Defined`](super::WorkflowDefinitionEvent) と同じ理由:
/// 事実の自己完結な記録)。再コンパイル等の変異が要件化したら変種を足し、
/// `coding-rules/aggregate-commands.md` の本則 (1 コマンド 1 イベント) がそのまま適用される。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompiledDefinitionEvent {
    /// 配布束がコンパイルされて存在するようになった (genesis)。
    Compiled(Compiled),
}
