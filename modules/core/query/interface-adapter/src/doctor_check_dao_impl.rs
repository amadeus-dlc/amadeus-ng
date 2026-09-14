//! 自己診断の行 (`read_doctor_check`) の読取り。
use super::read_model_store::ReadModelStore;
use core_query_use_case::orchestration::{DoctorCheck, DoctorCheckDao, ReadModelReadError};
use std::rc::Rc;
/// 一つの読取り接続で報告に属する行を表示順に引く。
#[derive(Debug)]
pub struct DoctorCheckDaoImpl {
    store: Rc<ReadModelStore>,
}
impl DoctorCheckDaoImpl {
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> Self {
        Self { store }
    }
}
impl DoctorCheckDao for DoctorCheckDaoImpl {
    fn find(&self, report_id: &str) -> Result<Vec<DoctorCheck>, ReadModelReadError> {
        self.store.find_many(
            "SELECT check_id, passed, label, fix FROM read_doctor_check WHERE report_id=?1 ORDER BY position",
            &[&report_id],
            |row| {
                Ok(DoctorCheck::new(
                    row.get(0)?,
                    row.get::<_, i64>(1)? != 0,
                    row.get(2)?,
                    row.get(3)?,
                ))
            },
        )
    }
}
