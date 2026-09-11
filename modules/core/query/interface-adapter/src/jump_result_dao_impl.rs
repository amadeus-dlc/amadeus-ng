//! 移動結果1表だけの読取り。
use super::read_model_store::ReadModelStore;
use core_query_use_case::orchestration::{JumpResultDao, JumpResultView, ReadModelReadError};
use std::rc::Rc;
#[derive(Debug)]
/// 移動結果の読取り境界。
pub struct JumpResultDaoImpl {
    store: Rc<ReadModelStore>,
}
impl JumpResultDaoImpl {
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> Self {
        Self { store }
    }
}
impl JumpResultDao for JumpResultDaoImpl {
    fn find(&self, id: &str) -> Result<Option<JumpResultView>, ReadModelReadError> {
        self.store.find_one(
            "SELECT payload FROM read_jump_result WHERE id=?1",
            &[&id],
            |row| Ok(JumpResultView::new(row.get(0)?)),
        )
    }
}
