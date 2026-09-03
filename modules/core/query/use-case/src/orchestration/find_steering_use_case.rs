//! `FindSteeringUseCase` — 配信計画とその第 1 部を FK でたどって組む。

use crate::orchestration::{
    ReadModelReadError, SteeringDeliveryView, SteeringPartDao, SteeringPlanDao,
};

/// run-stage が指す配信計画と、その第 1 部を引く。
///
/// # 引くのは 2 表、たどるのは FK である
///
/// | 順 | 表 | 鍵の出所 | 無いとき |
/// | --- | --- | --- | --- |
/// | 1 | `read_steering_plan` | run-stage の行の `steering_plan_id` | 未パック (別トランザクション) |
/// | 2 | `read_steering_part` | 行 1 の `id` + [`SteeringPartDao::FIRST_PART`] | 空計画 |
///
/// [`super::FindNextAnswerUseCase`] は答えの行を起点に同じ 2 段をたどるが、答えの行を
/// 持たない要求 (`--single`・state なしの jump) はここから入る。どちらも鍵を渡すだけで
/// 判断は無い。
///
/// バインディングはスタティックが既定なので DAO は型パラメータで保持する
/// (`coding-rules/use-case-rules.md` §2)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindSteeringUseCase<P: SteeringPlanDao, S: SteeringPartDao> {
    steering_plan_dao: P,
    steering_part_dao: S,
}

impl<P: SteeringPlanDao, S: SteeringPartDao> FindSteeringUseCase<P, S> {
    /// 2 つの引当の口を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(steering_plan_dao: P, steering_part_dao: S) -> FindSteeringUseCase<P, S> {
        FindSteeringUseCase {
            steering_plan_dao,
            steering_part_dao,
        }
    }

    /// 配信計画 1 本とその第 1 部を引く。
    ///
    /// `steering_plan_id` は run-stage の行が運ぶ FK である。計画がまだパックされていない
    /// のは正常なので `Ok(None)` になる (steering の 2 表は別トランザクション)。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    pub fn execute(
        &self,
        steering_plan_id: &str,
    ) -> Result<Option<SteeringDeliveryView>, ReadModelReadError> {
        let Some(plan) = self.steering_plan_dao.find(steering_plan_id)? else {
            return Ok(None);
        };
        let first_part = self.steering_part_dao.find(plan.id(), S::FIRST_PART)?;
        Ok(Some(SteeringDeliveryView::new(plan, first_part)))
    }
}
