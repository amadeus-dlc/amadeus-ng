//! runtime-graph の compile がステージ日誌を読んだ事実。
use crate::orchestration::{
    EmptyMemoryStages, IntentExecutionEventId, IntentExecutionId, MemoryJournalSurvey,
};
/// compile 時の日誌観測と、`MEMORY_EMPTY` を記録すると集約が決めた位置。
///
/// 観測 (`survey`) は runtime-graph の `memory_entries` / `memory_breakdown` の材料であり、
/// 判定 (`empty_stages`) は監査行の材料である。作業の進行も承認も変えない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryJournalsObserved {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    survey: MemoryJournalSurvey,
    empty_stages: EmptyMemoryStages,
}
impl MemoryJournalsObserved {
    /// 対象実行・観測・判定を束ねる。
    #[must_use]
    pub const fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        survey: MemoryJournalSurvey,
        empty_stages: EmptyMemoryStages,
    ) -> Self {
        Self {
            id,
            aggregate_id,
            survey,
            empty_stages,
        }
    }
    /// 観測事実の識別子。
    #[must_use]
    pub const fn id(&self) -> &IntentExecutionEventId {
        &self.id
    }
    /// 観測した作業実行。
    #[must_use]
    pub const fn aggregate_id(&self) -> &IntentExecutionId {
        &self.aggregate_id
    }
    /// compile 時に読めた日誌の観測列。
    #[must_use]
    pub const fn survey(&self) -> &MemoryJournalSurvey {
        &self.survey
    }
    /// `MEMORY_EMPTY` を記録する位置。
    #[must_use]
    pub const fn empty_stages(&self) -> &EmptyMemoryStages {
        &self.empty_stages
    }
}
