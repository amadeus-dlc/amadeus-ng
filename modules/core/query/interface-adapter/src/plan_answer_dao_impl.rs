//! 指定操作IDによる、計画回答リードモデルの単純読取り。
use super::read_model_store::ReadModelStore;
use core_query_use_case::orchestration::{PlanAnswerDao, PlanAnswerView, ReadModelReadError};
use std::rc::Rc;
/// 読取専用接続のDAO。
#[derive(Debug)]
pub struct PlanAnswerDaoImpl {
    store: Rc<ReadModelStore>,
}
impl PlanAnswerDaoImpl {
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> Self {
        Self { store }
    }
}
impl PlanAnswerDao for PlanAnswerDaoImpl {
    fn find(&self, id: &str) -> Result<Option<PlanAnswerView>, ReadModelReadError> {
        self.store.find_one("SELECT operation_id,status,emitted,stage,error,as_of FROM read_plan_answer WHERE operation_id=?1",&[&id],|row|{
 let as_of=u64::try_from(row.get::<_,i64>(5)?).map_err(|error|rusqlite::Error::FromSqlConversionFailure(5,rusqlite::types::Type::Integer,Box::new(error)))?;
 Ok(PlanAnswerView::new(row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,as_of))
 })
    }
}
