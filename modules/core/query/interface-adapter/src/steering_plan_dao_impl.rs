//! `SteeringPlanDao` の実 Gateway — 配信計画 1 行を `read_steering_plan` から引く。

use std::rc::Rc;

use core_query_use_case::orchestration::{ReadModelReadError, SteeringPlanDao, SteeringPlanView};
use rusqlite::Row;

use super::read_model_store::ReadModelStore;

/// `read_steering_plan` を 1 表で引く SELECT を、`WHERE` 句の literal から組む。
///
/// `source_digest` は載せない — RMU が再パックの要否を判断するための内部列であり、読み手の
/// クエリモデルではない。
macro_rules! select_steering_plan {
    ($where_clause:literal) => {
        concat!(
            "SELECT id, phase, bundle_digest, part_count, delivered_paths \
             FROM read_steering_plan WHERE ",
            $where_clause
        )
    };
}

/// 主キー (`read_run_stage.steering_plan_id` がこの値を指す) の 1 行引当。
const SELECT_BY_ID: &str = select_steering_plan!("id = ?1");

/// 主キー + token の束のダイジェスト。束縛は照合ではなく**鍵の残余条件**である。
const SELECT_BOUND: &str = select_steering_plan!("id = ?1 AND bundle_digest = ?2");

/// 配信計画 1 行を返す実装 (2 動詞とも同じ 1 表を鍵違いで引く)。
#[derive(Debug)]
pub struct SteeringPlanDaoImpl {
    store: Rc<ReadModelStore>,
}

impl SteeringPlanDaoImpl {
    /// 1 要求ぶんの共有ストアを受け取る (**この型の唯一の構築経路**)。
    ///
    /// 開くのは [`super::ReadModelDaos`] 1 か所で、12 実装はその 1 接続を分け合う。
    /// 実装ごとに開くと、多段の引当が別々のスナップショットを見る余地が残る。
    #[must_use]
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> SteeringPlanDaoImpl {
        SteeringPlanDaoImpl { store }
    }
}

/// 5 列を 1 行の写しへ。
fn steering_plan_row(row: &Row<'_>) -> rusqlite::Result<SteeringPlanView> {
    Ok(SteeringPlanView::new(
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
    ))
}

impl SteeringPlanDao for SteeringPlanDaoImpl {
    fn find(&self, id: &str) -> Result<Option<SteeringPlanView>, ReadModelReadError> {
        self.store.find_one(SELECT_BY_ID, &[&id], steering_plan_row)
    }

    fn find_bound(
        &self,
        id: &str,
        bundle_digest: &str,
    ) -> Result<Option<SteeringPlanView>, ReadModelReadError> {
        self.store
            .find_one(SELECT_BOUND, &[&id, &bundle_digest], steering_plan_row)
    }
}
