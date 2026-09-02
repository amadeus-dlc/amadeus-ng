//! `NextJumpRow` — `read_next_jump` の 1 行 (ジャンプ先ごとの受理判定と方向)。

use core_command_domain::orchestration::{Intent, IntentExecution, StageIndex, StageKey};

use super::spelling;

/// `read_next_jump` の 1 行。主キーは (`execution_id`, `target_index`)。
///
/// 値は集約のクエリ [`IntentExecution::jump_resolve`] の答えである。行は**全 target を
/// 網羅する** — 読取側が「跳べるか」を自分で判定しないための非正規化であり、拒否も
/// 1 つの答えとして行になる (裁定 §10-1)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextJumpRow {
    execution_id: String,
    target_index: usize,
    target_slug: String,
    outcome: String,
    refusal: Option<String>,
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
            execution_id: execution.id().as_str().to_string(),
            target_index: target.to_usize(),
            target_slug: key.slug().as_str().to_string(),
            outcome,
            refusal,
        }
    }

    /// 実行の識別子。
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
