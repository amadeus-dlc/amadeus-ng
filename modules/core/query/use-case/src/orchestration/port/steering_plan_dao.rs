//! `SteeringPlanDao` ポート — 1 フェーズの配信計画 1 行を引く DAO。

use super::read_model_read_error::ReadModelReadError;
use super::read_view::SteeringPlanView;

/// 配信計画の引当 (**読取専用**)。
///
/// 鍵は run-stage が運ぶ FK ([`super::RunStageView::steering_plan_id`]) である — フェーズの
/// 綴りで引き直さないのは、FK をたどるのが裁定の形だからである
/// (`coding-rules/cqrs-boundaries.md` 規則 6)。
///
/// **不在は失敗ではない。** この表は参照入力 (memory 層のルール) 由来でジャーナル由来 15 表
/// とは別トランザクションで差し替わるので、FK が指す計画がまだ無いのは正常な観測である。
pub trait SteeringPlanDao {
    /// 計画の識別子で 1 行を引く。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find(&self, id: &str) -> Result<Option<SteeringPlanView>, ReadModelReadError>;

    /// 計画の識別子と束のダイジェストで 1 行を引く (`continue` の照合)。
    ///
    /// 束縛は**鍵の一部**である。ずれた token は行に当たらず `Ok(None)` になるので、
    /// 「合っているか」を判定する経路はクエリ側に無い。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find_bound(
        &self,
        id: &str,
        bundle_digest: &str,
    ) -> Result<Option<SteeringPlanView>, ReadModelReadError>;
}
