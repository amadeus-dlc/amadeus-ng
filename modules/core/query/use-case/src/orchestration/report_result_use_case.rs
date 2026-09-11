//! 報告結果を指定IDで読む。業務判断・更新は持たない。
use super::{ReadModelReadError, ReportResultDao, ReportResultView};
/// 呼出側が既知の報告識別子で結果を引く。
#[derive(Debug)]
pub struct ReportResultUseCase<D: ReportResultDao> {
    dao: D,
}
impl<D: ReportResultDao> ReportResultUseCase<D> {
    /// DAOを注入する。
    #[must_use]
    pub const fn new(dao: D) -> Self {
        Self { dao }
    }
    /// 投影された結果を返す。
    /// # Errors
    /// リードモデルが読めない場合。
    pub fn execute(&self, report_id: &str) -> Result<Option<ReportResultView>, ReadModelReadError> {
        self.dao.find(report_id)
    }
}
