//! `Started` — `IntentExecutionEvent::Started` のペイロード。

use crate::orchestration::{IntentExecutionEventId, IntentExecutionId, IntentId, StageEntry};

/// `Started` のペイロード — genesis の材料 (実行の識別子・対象 intent の識別子・
/// 解決済み計画の写し) を運ぶ。
///
/// **集約の歴史は自ストリームだけで再生できる**のが ES の基本である
/// (`coding-rules/aggregate-commands.md`「genesis イベントから集約を導出する変換が
/// リプレイのスナップショット種」)。誕生状態の導出に `&Intent` が要る形は、実行の
/// ストリームだけでは再生できないという意味でこの基本に反していた (issue #56 で計画の
/// 写しを落とした際の見落とし)。したがって genesis に要る材料 —— 誕生の状態を組むのに
/// 必要な最小の静的材料 —— はこのイベントが運ぶ。
///
/// イベントが intent の材料の複製を運ぶのは**歴史**であって集約参照の違反ではない
/// (`coding-rules/aggregate-references.md`「イベントが材料の複製を運ぶのは歴史」)。
/// 禁じられるのは集約の**保持状態**への埋め込みであり、[`IntentExecution`] は従来どおり
/// intent を ID で参照し、計画からは添字帳 (slug + phase) と実効プランだけを取り込む。
///
/// [`IntentExecution`]: crate::orchestration::IntentExecution
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Started {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    intent_id: IntentId,
    stages: Vec<StageEntry>,
}

impl Started {
    /// イベントの識別子と genesis の材料 3 つを束ねる。
    #[must_use]
    pub const fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        intent_id: IntentId,
        stages: Vec<StageEntry>,
    ) -> Started {
        Started {
            id,
            aggregate_id,
            intent_id,
            stages,
        }
    }

    /// このイベント自身の識別子 — ドメインイベントはエンティティの一種なので自前の id を
    /// 持つ (`coding-rules/domain-object-kinds.md`)。
    #[must_use]
    pub const fn id(&self) -> &IntentExecutionEventId {
        &self.id
    }

    /// **どの集約の事実か** — 始まった実行の識別子。集約の ID をイベントの id に流用しない
    /// (オーナー裁定 2026-09-02) ので、こちらは `aggregate_id` と名乗る。
    #[must_use]
    pub const fn aggregate_id(&self) -> &IntentExecutionId {
        &self.aggregate_id
    }

    /// 開始された intent の識別子。
    #[must_use]
    pub const fn intent_id(&self) -> &IntentId {
        &self.intent_id
    }

    /// 開始時点の解決済み計画 (文書順)。
    #[must_use]
    pub fn stages(&self) -> &[StageEntry] {
        &self.stages
    }
}
