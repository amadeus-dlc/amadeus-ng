//! 自己診断の集計行 (`read_doctor_report`) の読取り。
use super::read_model_store::ReadModelStore;
use core_query_use_case::orchestration::{DoctorReportDao, DoctorSummaryView, ReadModelReadError};
use std::rc::Rc;
/// 一つの読取り接続で診断対象の集計行を引く。
#[derive(Debug)]
pub struct DoctorReportDaoImpl {
    store: Rc<ReadModelStore>,
}
impl DoctorReportDaoImpl {
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> Self {
        Self { store }
    }
}
impl DoctorReportDao for DoctorReportDaoImpl {
    fn find(&self, target: &str) -> Result<Option<DoctorSummaryView>, ReadModelReadError> {
        self.store.find_one(
            "SELECT id, passed, failed, exit_code FROM read_doctor_report WHERE target=?1",
            &[&target],
            |row| {
                // 列は `u64` / `u8` を素通しできないので、SQLite の `INTEGER` から範囲検査つきで
                // 写す (範囲外は行が壊れているということであり、既定値へ丸めない)。
                let count = |index: usize, raw: i64| {
                    u64::try_from(raw)
                        .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, raw))
                };
                let exit_code = row.get::<_, i64>(3)?;
                Ok(DoctorSummaryView::new(
                    row.get(0)?,
                    count(1, row.get::<_, i64>(1)?)?,
                    count(2, row.get::<_, i64>(2)?)?,
                    u8::try_from(exit_code)
                        .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(3, exit_code))?,
                ))
            },
        )
    }
}
