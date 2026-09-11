//! `NextJumpRow` — `read_next_jump` の 1 行 (ジャンプ先ごとの受理判定と方向)。

use core_command_domain::orchestration::{Intent, IntentExecution, StageIndex, StageKey};

use super::row_id;
use super::spelling;

/// `read_next_jump` の 1 行。主キーは 1 列 `id` (自然キー
/// (`execution_id`, `target_index`) から導いた代理キー)。`execution_id` は
/// `read_execution.id` を指す FK である。
///
/// 値は集約のクエリ [`IntentExecution::jump_resolve`] の答えである。行は**全 target を
/// 網羅する** — 読取側が「跳べるか」を自分で判定しないための非正規化であり、拒否も
/// 1 つの答えとして行になる (裁定 §10-1)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextJumpRow {
    id: String,
    execution_id: String,
    target_index: usize,
    target_slug: String,
    outcome: String,
    refusal: Option<String>,
    resolution: Option<String>,
}

impl NextJumpRow {
    /// 1 つのジャンプ先に対する答えを 1 行へ写す (**この型の唯一の構築経路**)。
    #[must_use]
    pub fn of(
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
        NextJumpRow {
            id: row_id::next_jump(execution.id().as_str(), target.to_usize()),
            execution_id: execution.id().as_str().to_string(),
            target_index: target.to_usize(),
            target_slug: key.slug().as_str().to_string(),
            resolution: resolution(execution, intent, target, &outcome),
            outcome,
            refusal,
        }
    }

    /// resolveの公開結果。判断は投影時に確定する。
    #[must_use]
    pub fn resolution(&self) -> Option<&str> {
        self.resolution.as_deref()
    }

    /// 主キー — 自然キー (`execution_id`, `target_index`) から導いた代理キー。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// `read_execution.id` を指す FK。
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// ジャンプ先の位置 (文書順の索引)。
    #[must_use]
    pub const fn target_index(&self) -> usize {
        self.target_index
    }

    /// ジャンプ先の slug。
    #[must_use]
    pub fn target_slug(&self) -> &str {
        &self.target_slug
    }

    /// 受理なら方向 (`forward` / `backward` / `redo`)、非受理なら `refused`。
    #[must_use]
    pub fn outcome(&self) -> &str {
        &self.outcome
    }

    /// 非受理のときだけ在る拒否理由 (受理は NULL)。
    #[must_use]
    pub fn refusal(&self) -> Option<&str> {
        self.refusal.as_deref()
    }
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
