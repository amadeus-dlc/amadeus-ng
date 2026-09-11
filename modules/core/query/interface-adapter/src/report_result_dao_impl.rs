//! 報告結果表の読取り。
use super::read_model_store::ReadModelStore;
use core_query_use_case::orchestration::{ReadModelReadError, ReportResultDao, ReportResultView};
use std::rc::Rc;
/// 一つの読取り接続で報告識別子を引く。
#[derive(Debug)]
pub struct ReportResultDaoImpl {
    store: Rc<ReadModelStore>,
}
impl ReportResultDaoImpl {
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> Self {
        Self { store }
    }
}
impl ReportResultDao for ReportResultDaoImpl {
    fn find(&self, report_id: &str) -> Result<Option<ReportResultView>, ReadModelReadError> {
        self.store.find_one("SELECT execution_id, stage, scope, result_kind, steps, no_op_reason, current_stage FROM read_report_result WHERE report_id=?1", &[&report_id], |row| Ok(ReportResultView::new(row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?)))
    }
}
