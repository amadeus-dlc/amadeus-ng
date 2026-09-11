//! read_intentに保存された開始結果の引当。
use super::read_model_store::ReadModelStore;
use core_query_use_case::orchestration::{
    InitializationDao, InitializationView, ReadModelReadError,
};
use std::rc::Rc;
/// 保存された開始結果を1表から取得する。
#[derive(Debug)]
pub struct InitializationDaoImpl {
    store: Rc<ReadModelStore>,
}
impl InitializationDaoImpl {
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> Self {
        Self { store }
    }
}
impl InitializationDao for InitializationDaoImpl {
    fn find(&self, intent_id: &str) -> Result<Option<InitializationView>, ReadModelReadError> {
        self.store.find_one("SELECT scope, depth, execute_count, project_type, languages, frameworks, build_system, first_stage, first_phase FROM read_intent WHERE id=?1", &[&intent_id], |row| Ok(InitializationView::new(row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?)))
    }
}
