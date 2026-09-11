//! `LearningsCaptured` — `IntentExecutionEvent::LearningsCaptured` のペイロード。

use crate::orchestration::{
    CapturedLearnings, IntentExecutionEventId, IntentExecutionId, LearningProvenance,
};
use crate::workflow_definition::StageSlug;

/// §13 の学びの儀式が、確定した学びをメモリ層の正本へ**書き写すと決めた**事実。
///
/// 材料は書込みの内容そのものである — どの学びを、どの層のどの見出しへ、どちらの側だけを
/// 補うのか。投影はこの材料だけで実践行と監査行 `RULE_LEARNED` を描けるので、描く時点で
/// ディスクを読み直さない（`coding-rules/aggregate-commands.md`「イベントが材料の複製を
/// 運ぶのは歴史である」）。
///
/// 素性 (`provenance`) は **surface の時点で固定した** space と intent である。持ち回るのは
/// 実行時のカーソルを読み直さないためであり、書込先も印もこの値だけで決まる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningsCaptured {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    stage: StageSlug,
    provenance: LearningProvenance,
    learnings: CapturedLearnings,
}

impl LearningsCaptured {
    /// 学びの書込みの材料を束ねる完全コンストラクタ。
    #[must_use]
    pub const fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        stage: StageSlug,
        provenance: LearningProvenance,
        learnings: CapturedLearnings,
    ) -> LearningsCaptured {
        LearningsCaptured {
            id,
            aggregate_id,
            stage,
            provenance,
            learnings,
        }
    }

    /// 儀式を走らせたステージ（監査行 `**Stage**:`）。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// surface の時点で固定した書込先の素性。
    #[must_use]
    pub const fn provenance(&self) -> &LearningProvenance {
        &self.provenance
    }

    /// この回に書く学びの列。
    #[must_use]
    pub const fn learnings(&self) -> &CapturedLearnings {
        &self.learnings
    }

    /// このイベント自身の識別子。
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
