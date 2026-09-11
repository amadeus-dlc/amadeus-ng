//! 指定操作IDによる、実装開始リードモデルの単純読取り。
use super::read_model_store::ReadModelStore;
use core_query_use_case::orchestration::{
    PlanGenerationDao, PlanGenerationView, ReadModelReadError,
};
use std::rc::Rc;
/// 読取専用接続のDAO。
#[derive(Debug)]
pub struct PlanGenerationDaoImpl {
    store: Rc<ReadModelStore>,
}
impl PlanGenerationDaoImpl {
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> Self {
        Self { store }
    }
}
impl PlanGenerationDao for PlanGenerationDaoImpl {
    fn find(&self, id: &str) -> Result<Option<PlanGenerationView>, ReadModelReadError> {
        self.store.find_one("SELECT operation_id,status,unit,error,as_of FROM read_plan_generation WHERE operation_id=?1",&[&id],|row|{
 let as_of=u64::try_from(row.get::<_,i64>(4)?).map_err(|error|rusqlite::Error::FromSqlConversionFailure(4,rusqlite::types::Type::Integer,Box::new(error)))?;
 Ok(PlanGenerationView::new(row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,as_of))
 })
    }
}
