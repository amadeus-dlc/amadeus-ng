//! `NextTurnView` — `next` 1 要求ぶんの組み立て結果 (ユースケースが FK をたどって組む)。

use crate::orchestration::{
    ExecutionView, NextAnswerView, RunStageView, SteeringPartView, SteeringPlanView,
};

/// `read_next_answer` を起点に FK をたどって集めた 1 ターンぶんの材料。
///
/// # 行の写しを埋め込む — 列を写し直さない
///
/// 各面は引いた表の行そのもの ([`NextAnswerView`] / [`ExecutionView`] / [`RunStageView`] /
/// [`SteeringPlanView`] / [`SteeringPartView`]) である。列を選び直して平らにすると、どの表の
/// どの列だったかが読めなくなり、行との突合せが効かなくなる。
///
/// # `None` は判断ではなく不在である
///
/// - `run_stage` — 答えの `run_stage_id` が NULL のとき (RMU が「材料は無い」と書いた)
/// - `plan` — steering の 2 表がまだパックされていないとき (別トランザクション)
/// - `first_part` — 空計画のとき (部の行が無い)
///
/// いずれも**行の有無をそのまま伝播した**ものであって、値を見て決めた分岐ではない
/// (オーナー裁定 2026-09-03 — 「null の FK は『無し』」)。
///
/// この型は述語も導出も持たない。どう描くか (素の run-stage か load-steering か) を決める
/// のはプレゼンタである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextTurnView {
    answer: NextAnswerView,
    execution: ExecutionView,
    run_stage: Option<RunStageView>,
    plan: Option<SteeringPlanView>,
    first_part: Option<SteeringPartView>,
}

impl NextTurnView {
    /// 5 段の引当結果を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        answer: NextAnswerView,
        execution: ExecutionView,
        run_stage: Option<RunStageView>,
        plan: Option<SteeringPlanView>,
        first_part: Option<SteeringPartView>,
    ) -> NextTurnView {
        NextTurnView {
            answer,
            execution,
            run_stage,
            plan,
            first_part,
        }
    }

    /// 答えの行 (`decision_kind` とその材料)。
    #[must_use]
    pub const fn answer(&self) -> &NextAnswerView {
        &self.answer
    }

    /// 答えを出した実行の現在地。
    #[must_use]
    pub const fn execution(&self) -> &ExecutionView {
        &self.execution
    }

    /// 答えが名指す run-stage の材料 (答えが材料を指していなければ `None`)。
    #[must_use]
    pub const fn run_stage(&self) -> Option<&RunStageView> {
        self.run_stage.as_ref()
    }

    /// そのステージのフェーズに配る steering 計画 (未パックなら `None`)。
    #[must_use]
    pub const fn plan(&self) -> Option<&SteeringPlanView> {
        self.plan.as_ref()
    }

    /// 計画の 1 部目 (空計画なら `None`)。
    #[must_use]
    pub const fn first_part(&self) -> Option<&SteeringPartView> {
        self.first_part.as_ref()
    }
}
