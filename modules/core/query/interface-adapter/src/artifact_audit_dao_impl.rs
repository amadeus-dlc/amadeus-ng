//! read_artifact_auditをIDで読むDAO。
use super::read_model_store::ReadModelStore;
use core_query_use_case::orchestration::{ArtifactAuditDao, ArtifactAuditView, ReadModelReadError};
use std::rc::Rc;
/// 単一の投影表を読むGateway。
#[derive(Debug)]
pub struct ArtifactAuditDaoImpl {
    store: Rc<ReadModelStore>,
}
impl ArtifactAuditDaoImpl {
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> Self {
        Self { store }
    }
}
impl ArtifactAuditDao for ArtifactAuditDaoImpl {
    fn find(&self, id: &str) -> Result<Option<ArtifactAuditView>, ReadModelReadError> {
        self.store.find_one("SELECT id,target,file,tool,context,created,occurred_at FROM read_artifact_audit WHERE id=?1",&[&id],|row|Ok(ArtifactAuditView::new(row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get::<_,i64>(5)?!=0,row.get(6)?)))
    }
}
