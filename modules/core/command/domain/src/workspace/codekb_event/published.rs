//! `CodekbPublished` — codekb ストアの中身が候補で置き換わった、という事実のペイロード。

use crate::workspace::codekb_artifacts::CodekbArtifacts;
use crate::workspace::codekb_event_id::CodekbEventId;
use crate::workspace::codekb_repo_id::CodekbRepoId;

/// codekb ストアが公開された、という事実の材料。
///
/// **内容そのもの (9 成果物のバイト) を運ぶ** — イベントが材料の複製を運ぶのは歴史であり
/// 違反ではない (`coding-rules/aggregate-references.md`)。公開後の世代は運ばない: 世代は
/// ディスクに置かれたバイトから**観測**する値であり、畳み方 (木のハッシュ) は
/// ドメインの持ち物ではないためである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodekbPublished {
    id: CodekbEventId,
    aggregate_id: CodekbRepoId,
    artifacts: CodekbArtifacts,
}

impl CodekbPublished {
    /// イベント識別子・集約の識別子と材料をそのまま束ねる。
    #[must_use]
    pub const fn new(
        id: CodekbEventId,
        aggregate_id: CodekbRepoId,
        artifacts: CodekbArtifacts,
    ) -> CodekbPublished {
        CodekbPublished {
            id,
            aggregate_id,
            artifacts,
        }
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

    /// 公開した 9 成果物 (Gateway が書き出すバイト)。
    #[must_use]
    pub const fn artifacts(&self) -> &CodekbArtifacts {
        &self.artifacts
    }
}
