//! `FindContinuationUseCase` — `continue` の続きを token の束縛で引き、FK をたどって組む。

use crate::orchestration::{
    ContinuationView, ReadModelReadError, RunStageDao, SteeringPartDao, SteeringPlanDao,
};

/// `continue` の続きを token の束縛で引く。
///
/// # 引くのは 3 表、束縛は鍵の一部である
///
/// | 順 | 表 | 鍵 | 無いとき |
/// | --- | --- | --- | --- |
/// | 1 | `read_run_stage` | 自然キー 3 列 + 経路 / directive の束縛 | `Ok(None)` — fail-closed |
/// | 2 | `read_steering_plan` | 行 1 の `steering_plan_id` + 束のダイジェスト | `Ok(None)` — fail-closed |
/// | 3 | `read_steering_part` | 行 2 の `id` + 部番号 | 終端 (`next_part` が `None`) |
///
/// 束縛は 1 つでもずれれば行に当たらないので、「合っているか」を判定する経路はここに無い。
/// state の束縛はこの口では扱わない — token がそれを運ぶかどうかは**要求の形**で決まる
/// ので、[`super::FindExecutionUseCase::execute_by_state_binding`] をコントローラが先に
/// 呼ぶ (`coding-rules/cqrs-boundaries.md` 規則 6 — 要求フラグの分岐は構文的ルーティング)。
///
/// バインディングはスタティックが既定なので DAO は型パラメータで保持する
/// (`coding-rules/use-case-rules.md` §2)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindContinuationUseCase<R: RunStageDao, P: SteeringPlanDao, S: SteeringPartDao> {
    run_stages: R,
    plans: P,
    parts: S,
}

impl<R: RunStageDao, P: SteeringPlanDao, S: SteeringPartDao> FindContinuationUseCase<R, P, S> {
    /// 3 つの引当の口を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(run_stages: R, plans: P, parts: S) -> FindContinuationUseCase<R, P, S> {
        FindContinuationUseCase {
            run_stages,
            plans,
            parts,
        }
    }

    /// token が名乗るステージと 2 つの束縛で run-stage を引き、その計画と次の部を集める。
    ///
    /// `part_index` は次に届ける部の番号 (1 始まり)。その番号の行が無ければ
    /// [`ContinuationView::next_part`] が `None` になる — 「もう配る部が無い」は行の有無で
    /// 表す。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    #[expect(
        clippy::too_many_arguments,
        reason = "引数は token が運ぶ鍵そのもの — 束ねると鍵の出所が読めなくなる"
    )]
    pub fn execute(
        &self,
        definition_id: &str,
        scope: &str,
        stage_slug: &str,
        route_digest: &str,
        directive_digest: &str,
        bundle_digest: &str,
        part_index: u32,
    ) -> Result<Option<ContinuationView>, ReadModelReadError> {
        let Some(run_stage) = self.run_stages.find_bound(
            definition_id,
            scope,
            stage_slug,
            route_digest,
            directive_digest,
        )?
        else {
            return Ok(None);
        };
        let Some(plan) = self
            .plans
            .find_bound(run_stage.steering_plan_id(), bundle_digest)?
        else {
            return Ok(None);
        };
        let next_part = self.parts.find(plan.id(), part_index)?;
        Ok(Some(ContinuationView::new(run_stage, plan, next_part)))
    }
}
