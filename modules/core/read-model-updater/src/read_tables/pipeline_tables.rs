//! Pipeline履歴と外部handoffを合わせた純粋な参照投影。
use super::{PipelineProgressRow, ReadTablesError};
use crate::orchestration::JournalBatch;
use core_command_domain::orchestration::{IntentExecutionId, PipelineHandoff};
use core_command_domain::workflow_definition::StageMode;
use core_infrastructure::canon_json::{JsonValue, hash_compact};
/// 一つの実行に属する通常・単独実行の進捗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineTables {
    execution_id: String,
    rows: Vec<PipelineProgressRow>,
}
impl PipelineTables {
    /// 受領の有効性は集約の履歴へ委譲し、答えだけを行に写す。
    /// # Errors
    /// 履歴が再生できない場合。
    pub fn project(
        history: &JournalBatch,
        id: &IntentExecutionId,
        current: Option<&PipelineHandoff>,
    ) -> Result<Self, ReadTablesError> {
        let definitions = super::replay_definitions(history)?;
        let executions = super::replay_executions(history)?;
        let mut rows = Vec::new();
        let position = history.scanned_to().map_or(0, |v| v.to_u64());
        let source = hash_compact(&JsonValue::Array(vec![
            JsonValue::String(position.to_string()),
            JsonValue::String(id.as_str().into()),
            JsonValue::String(current.map_or("", PipelineHandoff::path).into()),
            JsonValue::String(current.map_or("", PipelineHandoff::sha256).into()),
            JsonValue::String(current.map_or("", PipelineHandoff::mtime_ms).into()),
        ]))
        .rendered();
        if let Some(execution) = executions.iter().find(|execution| execution.id() == id)
            && let Some(intent) = history
                .intents()
                .iter()
                .find(|intent| intent.id() == execution.intent_id())
            && let Some(definition) = definitions
                .iter()
                .find(|definition| definition.id() == intent.definition_id())
        {
            for node in definition
                .graph()
                .nodes()
                .iter()
                .filter(|node| node.mode() == StageMode::Pipeline)
            {
                for single in [false, true] {
                    let completed = execution.pipeline_history().fold_completed(
                        node,
                        current,
                        single,
                        Vec::new(),
                        |mut names, receipt| {
                            names.push(receipt.link().to_string());
                            names
                        },
                    );
                    rows.push(PipelineProgressRow::new(
                        id.as_str().into(),
                        node.slug().as_str().into(),
                        single,
                        super::json_column::strings(&completed),
                        source.clone(),
                        position,
                    ));
                }
            }
        }
        Ok(Self {
            execution_id: id.as_str().into(),
            rows,
        })
    }
    /// 対象の実行。
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }
    /// 同じ参照入力から得た行。
    #[must_use]
    pub fn rows(&self) -> &[PipelineProgressRow] {
        &self.rows
    }
}
