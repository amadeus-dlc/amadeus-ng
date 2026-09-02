//! `StageCompleted` — `IntentExecutionEvent::StageCompleted` のペイロード。

use crate::orchestration::{IntentExecutionEventId, IntentExecutionId};
use crate::workflow_definition::StageSlug;

/// `StageCompleted` のペイロード — 起きた事実 (どのステージが完了したか) だけを運ぶ。
///
/// 次カーソルは載せない — 導出された状態であり、適用側 (集約) とリードモデル側 (RMU) が
/// それぞれ自分の状態から導く (オーナー裁定 2026-08-30「イベントに状態は含めるな」)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageCompleted {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    stage: StageSlug,
}

impl StageCompleted {
    /// 完了したステージ。
    #[must_use]
    pub const fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        stage: StageSlug,
    ) -> StageCompleted {
        StageCompleted {
            id,
            aggregate_id,
            stage,
        }
    }

    /// 完了したステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// このイベント自身の識別子 — ドメインイベントはエンティティの一種なので自前の id を
    /// 持つ (`coding-rules/domain-object-kinds.md`)。
    #[must_use]
    pub const fn id(&self) -> &IntentExecutionEventId {
        &self.id
    }

    /// **どの集約の事実か** — この事実が起きた実行の識別子。
    #[must_use]
    pub const fn aggregate_id(&self) -> &IntentExecutionId {
        &self.aggregate_id
    }
}
