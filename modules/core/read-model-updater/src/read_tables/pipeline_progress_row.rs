//! 実行別pipeline進捗の参照投影行。
/// 履歴と現在のhandoffから得た、公開可能な完了link列。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineProgressRow {
    id: String,
    execution_id: String,
    stage: String,
    single: bool,
    completed: String,
    source_digest: String,
    event_position: u64,
}
impl PipelineProgressRow {
    pub(super) fn new(
        execution_id: String,
        stage: String,
        single: bool,
        completed: String,
        source_digest: String,
        event_position: u64,
    ) -> Self {
        Self {
            id: format!("pipeline:{execution_id}:{stage}:{single}"),
            execution_id,
            stage,
            single,
            completed,
            source_digest,
            event_position,
        }
    }
    /// 代理主キー。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// 実行識別子。
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }
    /// 対象stage。
    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }
    /// 単独実行の受領か。
    #[must_use]
    pub const fn is_single(&self) -> bool {
        self.single
    }
    /// 完了linkのJSON列。
    #[must_use]
    pub fn completed(&self) -> &str {
        &self.completed
    }
    /// 履歴位置とファイル観測の照合子。
    #[must_use]
    pub fn source_digest(&self) -> &str {
        &self.source_digest
    }
    /// この投影の履歴位置。
    #[must_use]
    pub const fn event_position(&self) -> u64 {
        self.event_position
    }
}
