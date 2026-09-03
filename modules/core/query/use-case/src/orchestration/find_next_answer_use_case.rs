//! `FindNextAnswerUseCase` — `next` 1 ターンぶんを FK をたどって組む。

use crate::orchestration::{
    ExecutionDao, NextAnswerDao, NextTurnView, ReadModelReadError, RunStageDao, RunStageView,
    SteeringPartDao, SteeringPartView, SteeringPlanDao, SteeringPlanView,
};

/// `next` の答えとその材料を引く。
///
/// # 引くのは 5 表、たどるのは FK である
///
/// DAO は 1 表しか引かないので、答えを描くのに要る面はユースケースが**行の FK 列を
/// そのまま次の鍵にして**集める (オーナー裁定 2026-09-03 —
/// `coding-rules/cqrs-boundaries.md` 規則 6)。
///
/// | 順 | 表 | 鍵の出所 | 無いとき |
/// | --- | --- | --- | --- |
/// | 1 | `read_next_answer` | 要求 (コントローラ) | `Ok(None)` — まだ投影されていない |
/// | 2 | `read_execution` | 行 1 の `execution_id` | 壊れた投影 (同一トランザクション) |
/// | 3 | `read_run_stage` | 行 1 の `run_stage_id` | NULL なら材料なし / 宙浮きは壊れた投影 |
/// | 4 | `read_steering_plan` | 行 3 の `steering_plan_id` | 未パック (別トランザクション) |
/// | 5 | `read_steering_part` | 行 4 の `id` + [`SteeringPartDao::FIRST_PART`] | 空計画 |
///
/// # これは判断ではない
///
/// 次の鍵は前の行の値を**そのまま**渡すだけで、比較も変換も分岐もしない。経路は要求に
/// よらず常に同じ 5 段で、行の値で段が増減することはない。唯一の枝分かれは「FK が NULL
/// なら `None`」だが、これは裁定が定義した不在の伝播であって判断ではない。
///
/// バインディングはスタティックが既定なので DAO は型パラメータで保持する
/// (`coding-rules/use-case-rules.md` §2)。実装 (`XxxDaoImpl`) には依存しない — 結線は
/// 合成ルートだけが行う (同 §1 の DIP)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindNextAnswerUseCase<
    A: NextAnswerDao,
    E: ExecutionDao,
    R: RunStageDao,
    P: SteeringPlanDao,
    S: SteeringPartDao,
> {
    answers: A,
    executions: E,
    run_stages: R,
    plans: P,
    parts: S,
}

impl<A: NextAnswerDao, E: ExecutionDao, R: RunStageDao, P: SteeringPlanDao, S: SteeringPartDao>
    FindNextAnswerUseCase<A, E, R, P, S>
{
    /// 5 つの引当の口を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        answers: A,
        executions: E,
        run_stages: R,
        plans: P,
        parts: S,
    ) -> FindNextAnswerUseCase<A, E, R, P, S> {
        FindNextAnswerUseCase {
            answers,
            executions,
            run_stages,
            plans,
            parts,
        }
    }

    /// 実行 1 本 × 要求の形 1 つの答えと、その材料を引く。
    ///
    /// `request_kind` は行のキーになる 4 値の綴り (`bare` / `resume` / `free-text` /
    /// `reentry`)。どの綴りで引くかは要求の形で決まるので**コントローラのルーティング**で
    /// あり、この口は渡された鍵で引くだけである。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない、または同一スナップショットの FK が宙に浮いている
    /// ([`ReadModelReadError`])。
    pub fn execute(
        &self,
        execution_id: &str,
        request_kind: &str,
    ) -> Result<Option<NextTurnView>, ReadModelReadError> {
        let Some(answer) = self.answers.find(execution_id, request_kind)? else {
            return Ok(None);
        };
        let execution = self
            .executions
            .find(answer.execution_id())?
            .ok_or_else(ReadModelReadError::broken_projection)?;
        let run_stage = self.run_stage_of(answer.run_stage_id())?;
        let plan = self.plan_of(run_stage.as_ref())?;
        let first_part = self.first_part_of(plan.as_ref())?;
        Ok(Some(NextTurnView::new(
            answer, execution, run_stage, plan, first_part,
        )))
    }

    /// 答えが指す run-stage の材料 (FK が NULL なら材料なし)。
    fn run_stage_of(&self, id: Option<&str>) -> Result<Option<RunStageView>, ReadModelReadError> {
        match id {
            None => Ok(None),
            Some(id) => self
                .run_stages
                .find_by_id(id)?
                .ok_or_else(ReadModelReadError::broken_projection)
                .map(Some),
        }
    }

    /// run-stage が指す配信計画 (未パックなら `None` — 別トランザクションなので不在は正常)。
    fn plan_of(
        &self,
        run_stage: Option<&RunStageView>,
    ) -> Result<Option<SteeringPlanView>, ReadModelReadError> {
        match run_stage {
            None => Ok(None),
            Some(run_stage) => self.plans.find(run_stage.steering_plan_id()),
        }
    }

    /// 計画の 1 部目 (空計画なら `None`)。
    fn first_part_of(
        &self,
        plan: Option<&SteeringPlanView>,
    ) -> Result<Option<SteeringPartView>, ReadModelReadError> {
        match plan {
            None => Ok(None),
            Some(plan) => self.parts.find(plan.id(), S::FIRST_PART),
        }
    }
}
