//! 移動イベントとその前後から構築する結果行。
use super::ReadTablesError;
use crate::orchestration::JournalBatch;
use core_command_domain::{
    orchestration::{IntentExecution, IntentExecutionEvent, JumpDirection},
    workflow_definition::PlanAction,
    workspace::CheckboxState,
};
use core_infrastructure::canon_json::{JsonValue, ObjectMembers, SerializationProfile, serialize};
use std::collections::BTreeMap;
/// 呼出側のイベント識別子で取得する過去の移動結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JumpResultRow {
    id: String,
    payload: String,
}
impl JumpResultRow {
    /// SQLの主キー。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// 公開言語で投影した結果。Queryは現在状態から再計算しない。
    #[must_use]
    pub fn payload(&self) -> &str {
        &self.payload
    }
    pub(super) fn project(history: &JournalBatch) -> Result<Vec<Self>, ReadTablesError> {
        let mut executions = BTreeMap::<String, IntentExecution>::new();
        let mut rows = Vec::new();
        for entry in history.executions() {
            let key = entry.execution_id().as_str();
            if let IntentExecutionEvent::Started(started) = entry.event() {
                executions.insert(
                    key.to_string(),
                    IntentExecution::from((started.clone(), *entry.occurred_at())),
                );
                continue;
            }
            let before = executions
                .remove(key)
                .ok_or_else(|| ReadTablesError::MissingGenesis {
                    aggregate_id: key.into(),
                })?;
            let mut fields = ObjectMembers::new();
            if let IntentExecutionEvent::Jumped(jump) = entry.event() {
                let target = before.resolve_jump_target(jump.target()).map_err(|_| {
                    ReadTablesError::MissingGenesis {
                        aggregate_id: key.into(),
                    }
                })?;
                let source = before.cursor().to_usize();
                let target_at = target.to_usize();
                let direction = jump.direction();
                let (skipped, reset) = before.slots().fold_left(
                    (Vec::new(), Vec::new()),
                    |(mut skipped, mut reset), slot| {
                        let index = before.resolve_jump_target(slot.key().slug()).ok();
                        let position = index.map_or(usize::MAX, |i| i.to_usize());
                        // 跳躍に使う計画 — 別 scope が名指されていればその静的な列、無ければ
                        // この実行の実効計画 (集約の `apply_jump` と同じ導出)。
                        let in_plan = match jump.scope() {
                            Some(scope) => scope.contains(slot.key().slug()),
                            None => {
                                index.and_then(|i| before.effective_plan(i))
                                    == Some(PlanAction::Execute)
                            }
                        };
                        if direction == JumpDirection::Forward
                            && position > source
                            && position < target_at
                            && in_plan
                            && slot.checkbox().is_in_flight()
                        {
                            skipped.push(slot.key().slug().as_str().to_string());
                        }
                        if direction == JumpDirection::Backward
                            && position >= target_at
                            && in_plan
                            && slot.checkbox() != CheckboxState::Pending
                        {
                            reset.push(slot.key().slug().as_str().to_string());
                        }
                        (skipped, reset)
                    },
                );
                let mut skipped = skipped;
                let reset = if direction == JumpDirection::Redo {
                    vec![jump.target().as_str().to_string()]
                } else {
                    reset
                };
                if direction == JumpDirection::Forward
                    && source != target_at
                    && before
                        .checkbox(before.cursor())
                        .is_some_and(CheckboxState::is_active)
                    && let Some(slug) = before.cursor_slug()
                {
                    skipped.push(slug.as_str().to_string());
                }
                fields.insert(
                    "direction",
                    JsonValue::String(
                        match direction {
                            JumpDirection::Forward => "forward",
                            JumpDirection::Backward => "backward",
                            JumpDirection::Redo => "redo",
                        }
                        .into(),
                    ),
                );
                fields.insert("target", JsonValue::String(jump.target().as_str().into()));
                let phase = before
                    .slots()
                    .at(target)
                    .ok_or_else(|| ReadTablesError::MissingGenesis {
                        aggregate_id: key.into(),
                    })?
                    .key()
                    .phase();
                fields.insert(
                    "target_phase",
                    JsonValue::String(phase.as_str().to_uppercase()),
                );
                fields.insert(
                    "stages_skipped",
                    JsonValue::Array(skipped.into_iter().map(JsonValue::String).collect()),
                );
                fields.insert(
                    "stages_reset",
                    JsonValue::Array(reset.into_iter().map(JsonValue::String).collect()),
                );
                fields.insert("state_updated", JsonValue::Bool(true));
                fields.insert("audit_appended", JsonValue::Bool(true));
            }
            let after = IntentExecution::replay(
                before,
                std::iter::once((entry.seq_nr(), *entry.occurred_at(), entry.event().clone())),
            );
            if let IntentExecutionEvent::Jumped(jump) = entry.event() {
                let count = after.slots().fold_left(0u64, |count, slot| {
                    count + u64::from(slot.checkbox() == CheckboxState::Completed)
                });
                fields.insert(
                    "completed_count",
                    JsonValue::Number(core_infrastructure::canon_json::Number::PosInt(count)),
                );
                fields.insert("workflow_stopped", JsonValue::Bool(false));
                fields.insert(
                    "timestamp",
                    JsonValue::String(
                        entry
                            .occurred_at()
                            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                    ),
                );
                rows.push(Self {
                    id: jump.id().as_str().into(),
                    payload: serialize(
                        &JsonValue::Object(fields),
                        SerializationProfile::ContractCompact,
                    ),
                });
            }
            executions.insert(key.to_string(), after);
        }
        Ok(rows)
    }
}
