//! `CodekbInterruptedPublicationSettled` — 中断した公開に決着がついた、という事実のペイロード。

use crate::workspace::codekb_event_id::CodekbEventId;
use crate::workspace::codekb_repo_id::CodekbRepoId;

/// 中断した公開が畳まれ、ストアが「いま在る」姿へ確定した、という事実の材料。
///
/// **運ぶのは「どの codekb の決着か」だけ**である。どこに何が取り残されていたか、それを
/// どう畳むかは媒体の話であり、Repository 実装の内部詳細に属する
/// (`coding-rules/gateway-taxonomy.md` §2 — 媒体名を契約に漏らさない)。決着後の世代も
/// 運ばない: 世代はディスクに置かれたバイトから**観測**する値であり、畳み方 (木のハッシュ) は
/// ドメインの持ち物ではないためである ([`CodekbPublished`](super::CodekbPublished) と同じ理由)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodekbInterruptedPublicationSettled {
    id: CodekbEventId,
    aggregate_id: CodekbRepoId,
}

impl CodekbInterruptedPublicationSettled {
    /// イベント識別子と集約の識別子を束ねる。
    #[must_use]
    pub const fn new(
        id: CodekbEventId,
        aggregate_id: CodekbRepoId,
    ) -> CodekbInterruptedPublicationSettled {
        CodekbInterruptedPublicationSettled { id, aggregate_id }
    }

    /// このイベント自身の識別子。
    #[must_use]
    pub const fn id(&self) -> &CodekbEventId {
        &self.id
    }

    /// **どの集約の事実か** — codekb を鍵付けるリポジトリ識別子。
    #[must_use]
    pub const fn aggregate_id(&self) -> &CodekbRepoId {
        &self.aggregate_id
    }
}
