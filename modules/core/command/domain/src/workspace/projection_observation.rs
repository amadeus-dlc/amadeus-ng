//! D5.b の観測 — 選択中の実行に対する投影の位置と整合の材料。

/// RMU が書いた投影の所在と、ジャーナル側の位置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionObservation {
    execution_intent_id: Option<String>,
    checkpoint: Option<i64>,
    latest_execution_event: Option<i64>,
    pending_publications: u64,
    audit_shard_count: usize,
}

impl ProjectionObservation {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(
        execution_intent_id: Option<String>,
        checkpoint: Option<i64>,
        latest_execution_event: Option<i64>,
        pending_publications: u64,
        audit_shard_count: usize,
    ) -> Self {
        Self {
            execution_intent_id,
            checkpoint,
            latest_execution_event,
            pending_publications,
            audit_shard_count,
        }
    }

    /// 実行行 (`read_execution`) が名乗る intent (行が無ければ `None`)。
    #[must_use]
    pub fn execution_intent_id(&self) -> Option<&str> {
        self.execution_intent_id.as_deref()
    }

    /// この実行の投影チェックポイント (未登録なら `None`)。
    #[must_use]
    pub const fn checkpoint(&self) -> Option<i64> {
        self.checkpoint
    }

    /// この実行のジャーナル最新行の位置 (行が無ければ `None`)。
    #[must_use]
    pub const fn latest_execution_event(&self) -> Option<i64> {
        self.latest_execution_event
    }

    /// 未確定の公開計画の数。
    #[must_use]
    pub const fn pending_publications(&self) -> u64 {
        self.pending_publications
    }

    /// 記録の監査シャード数。
    #[must_use]
    pub const fn audit_shard_count(&self) -> usize {
        self.audit_shard_count
    }
}
