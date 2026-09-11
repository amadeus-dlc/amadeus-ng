//! 計画指紋の1表引当。
use super::read_model_store::ReadModelStore;
use core_query_use_case::orchestration::{
    PlanFingerprintDao, PlanFingerprintView, ReadModelReadError,
};
use std::rc::Rc;
/// 実行と対象のキーで保存済みの行だけを読む。
#[derive(Debug)]
pub struct PlanFingerprintDaoImpl {
    store: Rc<ReadModelStore>,
}
impl PlanFingerprintDaoImpl {
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> Self {
        Self { store }
    }
}
impl PlanFingerprintDao for PlanFingerprintDaoImpl {
    fn find(
        &self,
        execution_id: &str,
        target_id: &str,
    ) -> Result<Option<PlanFingerprintView>, ReadModelReadError> {
        self.store.find_one("SELECT fingerprint,error FROM read_plan_fingerprint WHERE execution_id=?1 AND target_id=?2", &[&execution_id, &target_id], |row| Ok(PlanFingerprintView::new(row.get(0)?, row.get(1)?)))
    }
}
