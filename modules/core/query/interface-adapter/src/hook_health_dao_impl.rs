//! read_hook_healthをIDで読むDAO。
use super::read_model_store::ReadModelStore;
use core_query_use_case::orchestration::{HookHealthDao, HookHealthView, ReadModelReadError};
use std::rc::Rc;
/// read_hook_healthを共有接続から読むDAO。
#[derive(Debug)]
pub struct HookHealthDaoImpl {
    store: Rc<ReadModelStore>,
}
impl HookHealthDaoImpl {
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> Self {
        Self { store }
    }
}
impl HookHealthDao for HookHealthDaoImpl {
    fn find(&self, id: &str) -> Result<Option<HookHealthView>, ReadModelReadError> {
        self.store.find_one("SELECT id,target,hook,heartbeat,seq_nr,drops,latest_drop FROM read_hook_health WHERE id=?1",&[&id],|row|Ok(HookHealthView::new(row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,u64::try_from(row.get::<_,i64>(4)?).map_err(|_|rusqlite::Error::InvalidQuery)?,u64::try_from(row.get::<_,i64>(5)?).map_err(|_|rusqlite::Error::InvalidQuery)?,row.get(6)?)))
    }
}
