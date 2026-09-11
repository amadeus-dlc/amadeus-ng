//! セッション監査結果行のSQLite読取り。
use super::read_model_store::ReadModelStore;
use core_query_use_case::orchestration::{ReadModelReadError, SessionAuditDao, SessionAuditView};
use std::rc::Rc;
#[derive(Debug)]
/// SQLの既存行をViewへ写すDAO。
pub struct SessionAuditDaoImpl {
    store: Rc<ReadModelStore>,
}
impl SessionAuditDaoImpl {
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> Self {
        Self { store }
    }
}
impl SessionAuditDao for SessionAuditDaoImpl {
    fn find(&self, id: &str) -> Result<Option<SessionAuditView>, ReadModelReadError> {
        self.store.find_one(
            "SELECT id,target,kind FROM read_session_audit WHERE id=?1",
            &[&id],
            |row| Ok(SessionAuditView::new(row.get(0)?, row.get(1)?, row.get(2)?)),
        )
    }
}
