//! `ContinuationView` — `continue` 1 要求ぶんの組み立て結果 (ユースケースが FK をたどって組む)。

use crate::orchestration::{RunStageView, SteeringPartView, SteeringPlanView};

/// token の束縛と部番号で引いた run-stage・配信計画・次の部。
///
/// # 引けたこと自体が照合の答えである
///
/// token が運ぶ束縛 (state / route / directive / bundle) はすべて**鍵の一部**として
/// 各表の `WHERE` に並ぶ。1 つでもずれた token はどこかの段で行に当たらず、この型は
/// 組み立たない — `None` が返ったら fail-closed の文言を出す、というのがプレゼンタの
/// 仕事である。クエリ側に「合っているか」を判定する経路は無い。
///
/// # 終端は行の有無で表す
///
/// [`ContinuationView::next_part`] が `None` なら、その部番号の行が無い = もう配る部が
/// 無いということである。「全部配り終えた」という判断をこの型は持たない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuationView {
    run_stage: RunStageView,
    plan: SteeringPlanView,
    next_part: Option<SteeringPartView>,
}

impl ContinuationView {
    /// 3 段の引当結果を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        run_stage: RunStageView,
        plan: SteeringPlanView,
        next_part: Option<SteeringPartView>,
    ) -> ContinuationView {
        ContinuationView {
            run_stage,
            plan,
            next_part,
        }
    }

    /// 継続先の run-stage 材料。
    #[must_use]
    pub const fn run_stage(&self) -> &RunStageView {
        &self.run_stage
    }

    /// そのフェーズの配信計画 (部の総数と配信済みパス台帳)。
    #[must_use]
    pub const fn plan(&self) -> &SteeringPlanView {
        &self.plan
    }

    /// 要求された部番号の中身 (その番号の行が無ければ `None`)。
    #[must_use]
    pub const fn next_part(&self) -> Option<&SteeringPartView> {
        self.next_part.as_ref()
    }
}
