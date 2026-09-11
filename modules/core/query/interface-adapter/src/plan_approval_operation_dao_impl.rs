//! 操作IDによる、共有承認表の読取り。
use super::read_model_store::ReadModelStore;
use core_query_use_case::orchestration::{
    PlanApprovalOperationDao, PlanApprovalOperationView, ReadModelReadError,
};
use std::rc::Rc;
/// 読取専用接続を使用するDAO。
#[derive(Debug)]
pub struct PlanApprovalOperationDaoImpl {
    store: Rc<ReadModelStore>,
}
impl PlanApprovalOperationDaoImpl {
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> Self {
        Self { store }
    }
    fn decode(row: &rusqlite::Row<'_>) -> rusqlite::Result<PlanApprovalOperationView> {
        let as_of = u64::try_from(row.get::<_, i64>(4)?).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Integer,
                Box::new(error),
            )
        })?;
        Ok(PlanApprovalOperationView::new(
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            as_of,
            row.get(5)?,
        ))
    }
}
impl PlanApprovalOperationDao for PlanApprovalOperationDaoImpl {
    fn find_pending(&self) -> Result<Vec<PlanApprovalOperationView>, ReadModelReadError> {
        self.store.find_many("SELECT operation_id,status,space,execution_id,as_of,kind FROM read_plan_operation WHERE status='prepared' ORDER BY operation_id", &[], Self::decode)
    }
    fn find(
        &self,
        operation_id: &str,
    ) -> Result<Option<PlanApprovalOperationView>, ReadModelReadError> {
        self.store.find_one("SELECT operation_id,status,space,execution_id,as_of,kind FROM read_plan_operation WHERE operation_id=?1", &[&operation_id], Self::decode)
    }
}
