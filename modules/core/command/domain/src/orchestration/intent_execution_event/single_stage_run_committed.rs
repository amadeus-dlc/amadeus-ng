//! `SingleStageRunCommitted` — `IntentExecutionEvent::SingleStageRunCommitted` のペイロード。

use crate::orchestration::{IntentExecutionEventId, IntentExecutionId};
use crate::workflow_definition::StageSlug;

/// 隔離実行 (`report --single`) の疑似ワークフロー ID 付き対を記録した事実。
///
/// この事実は**本流の状態を 1 つも動かさない** (適用はフレーム空) — 隔離実行は
/// 「その intent の記録の中で起きた」監査上の事実であり、カーソル・checkbox・`Status`・
/// park・overlay・autonomy・承認履歴のいずれにも触れない (仕様 I10、オーナー裁定
/// 2026-09-04)。投影側 (RMU) はこの 1 件から `STAGE_STARTED` / `STAGE_COMPLETED` の
/// 監査 2 行を `Workflow: single-stage:<slug>` 付きで描く。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SingleStageRunCommitted {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    stage: StageSlug,
}

impl SingleStageRunCommitted {
    /// 隔離実行したステージを束ねる。
    #[must_use]
    pub const fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        stage: StageSlug,
    ) -> SingleStageRunCommitted {
        SingleStageRunCommitted {
            id,
            aggregate_id,
            stage,
        }
    }

    /// 隔離実行したステージ。
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
