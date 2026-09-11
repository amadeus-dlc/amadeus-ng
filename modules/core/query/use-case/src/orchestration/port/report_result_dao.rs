//! 報告識別子による結果の読取りポート。
use super::{ReadModelReadError, ReportResultView};
/// 報告時点の結果を読み、現在状態から作り直さない。
pub trait ReportResultDao {
    /// 指定した報告結果。不在はNone。
    /// # Errors
    /// リードモデルが読めない場合。
    fn find(&self, report_id: &str) -> Result<Option<ReportResultView>, ReadModelReadError>;
}
