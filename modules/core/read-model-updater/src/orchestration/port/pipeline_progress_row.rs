//! `PipelineProgressRow` — `read_pipeline_progress` の 1 行 (実行・ステージ・単独実行の別ごとの
//! Pipeline 進捗)。

use crate::orchestration::GlobalSeqNr;

/// `read_pipeline_progress` の 1 行。履歴と現在の handoff から得た、公開してよい完了 link の列を持つ。
///
/// 行は値を運ぶだけである。履歴と handoff から行を組む投影 (代理主キーの導出を含む) は
/// [`crate::read_tables::PipelineTables::project`] が持つ。同じ実行の行はどれも同じ出所
/// (`source_digest` と `event_position`) を名乗る。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineProgressRow {
    id: String,
    execution_id: String,
    stage: String,
    single: bool,
    completed: String,
    source_digest: String,
    event_position: GlobalSeqNr,
}

impl PipelineProgressRow {
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        id: String,
        execution_id: String,
        stage: String,
        single: bool,
        completed: String,
        source_digest: String,
        event_position: GlobalSeqNr,
    ) -> Self {
        Self {
            id,
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

    /// 対象ステージ。
    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }

    /// 単独実行の受領か。
    #[must_use]
    pub const fn is_single(&self) -> bool {
        self.single
    }

    /// 完了 link の JSON 列。
    #[must_use]
    pub fn completed(&self) -> &str {
        &self.completed
    }

    /// 履歴位置と handoff の観測の照合子。
    #[must_use]
    pub fn source_digest(&self) -> &str {
        &self.source_digest
    }

    /// この投影の履歴位置。
    #[must_use]
    pub const fn event_position(&self) -> GlobalSeqNr {
        self.event_position
    }
}
