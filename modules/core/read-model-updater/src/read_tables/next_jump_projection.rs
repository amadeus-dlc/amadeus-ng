//! `read_next_jump` の行を組む投影 — ジャンプ先ごとの受理判定と方向を [`NextJumpRow`] へ写す。

use core_command_domain::orchestration::{Intent, IntentExecution, StageIndex, StageKey};

use super::row_id;
use super::spelling;
use crate::orchestration::NextJumpRow;

/// 1 つのジャンプ先に対する答えを 1 行へ写す。
///
/// 受理・拒否の判断は集約のクエリ `jump_resolve` が持つ。ここはその答えの綴りと、
/// 受理のときの公開結果 (`resolution`) を組むだけである。
pub(super) fn row(
    execution: &IntentExecution,
    intent: &Intent,
    target: StageIndex,
    key: &StageKey,
) -> NextJumpRow {
    let (outcome, refusal) = match execution.jump_resolve(intent, target) {
        Ok(direction) => (spelling::jump_direction(direction).to_string(), None),
        Err(error) => (
            spelling::JUMP_REFUSED.to_string(),
            Some(spelling::jump_refusal(&error).to_string()),
        ),
    };
    let resolution = resolution(execution, intent, target, &outcome);
    NextJumpRow::new(
        row_id::next_jump(execution.id().as_str(), target.to_usize()),
        execution.id().as_str().to_string(),
        target.to_usize(),
        key.slug().as_str().to_string(),
        outcome,
        refusal,
        resolution,
    )
}

fn resolution(
    execution: &IntentExecution,
    intent: &Intent,
    target: StageIndex,
    outcome: &str,
) -> Option<String> {
    use core_command_domain::workflow_definition::PlanAction;
    use core_infrastructure::canon_json::{
        JsonValue, ObjectMembers, SerializationProfile, serialize,
    };
    if outcome == "refused" {
        return None;
    }
    let target_stage = intent.stages().at(target)?;
    let source_stage = intent.stages().at(execution.cursor())?;
    let mut fields = ObjectMembers::new();
    for (key, value) in [
        ("target_slug", target_stage.slug().as_str()),
        ("target_phase", target_stage.phase().as_str()),
        ("target_number", target_stage.display().number().as_str()),
        ("target_name", target_stage.display().name()),
        ("current_slug", source_stage.slug().as_str()),
        ("current_number", source_stage.display().number().as_str()),
        ("direction", outcome),
    ] {
        fields.insert(
            key,
            JsonValue::String(if key == "target_phase" {
                value.to_uppercase()
            } else {
                value.to_string()
            }),
        );
    }
    let (_, affected) =
        execution
            .slots()
            .fold_left((0usize, Vec::new()), |(position, mut names), slot| {
                if slot.plan_action() == PlanAction::Execute
                    && (outcome == "forward"
                        && position > execution.cursor().to_usize()
                        && position < target.to_usize()
                        || outcome == "backward" && position >= target.to_usize())
                {
                    names.push(JsonValue::String(slot.key().slug().as_str().into()));
                }
                (position + 1, names)
            });
    fields.insert("affected_stages", JsonValue::Array(affected));
    fields.insert("valid", JsonValue::Bool(true));
    Some(serialize(
        &JsonValue::Object(fields),
        SerializationProfile::ContractCompact,
    ))
}
