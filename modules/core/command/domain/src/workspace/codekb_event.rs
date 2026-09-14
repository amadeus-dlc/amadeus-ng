//! `CodekbEvent` — 集約 [`Codekb`](super::Codekb) のドメインイベント。

// 変種ペイロードは 1 ファイル 1 公開型で本ファイル同名のサブツリーに置き、ここで連鎖
// 再輸出する (所有サブツリーのファサード — `coding-rules/module-visibility.md`
// §追記 2026-09-01)。
use super::codekb_event_id::CodekbEventId;
use super::codekb_repo_id::CodekbRepoId;

mod interrupted_publication_settled;
mod published;

pub use interrupted_publication_settled::CodekbInterruptedPublicationSettled;
pub use published::CodekbPublished;

/// codekb ストアに起きた事実。
///
/// 1 コマンド 1 イベント (`coding-rules/aggregate-commands.md`)。遷移は 2 つである —
/// **公開** (ストアは常に 9 成果物ごと置き換わる。部分更新も削除も upstream の動詞に無い) と、
/// **中断した公開の決着** (途中で落ちた公開を畳んで、ストアを「いま在る」姿へ確定させる)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodekbEvent {
    /// ストアの中身が候補で置き換わった。
    Published(CodekbPublished),
    /// 中断した公開に決着がついた。
    InterruptedPublicationSettled(CodekbInterruptedPublicationSettled),
}

impl CodekbEvent {
    /// このイベント自身の識別子 (全変種が持つ — イベントはエンティティ)。
    #[must_use]
    pub const fn id(&self) -> &CodekbEventId {
        match self {
            CodekbEvent::Published(payload) => payload.id(),
            CodekbEvent::InterruptedPublicationSettled(payload) => payload.id(),
        }
    }

    /// **どの集約の事実か** — codekb を鍵付けるリポジトリ識別子。
    #[must_use]
    pub const fn aggregate_id(&self) -> &CodekbRepoId {
        match self {
            CodekbEvent::Published(payload) => payload.aggregate_id(),
            CodekbEvent::InterruptedPublicationSettled(payload) => payload.aggregate_id(),
        }
    }
}
