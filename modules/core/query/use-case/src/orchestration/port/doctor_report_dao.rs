//! 自己診断の集計行を引くポート — `read_doctor_report` を 1 表 1 引当で読む。

use super::{DoctorSummaryView, ReadModelReadError};

/// 診断対象 (`spaces/<space>/intents[/<record>]`) で集計行を引く。
pub trait DoctorReportDao {
    /// 対象の最新の診断の集計を引く。行が無いのは失敗ではない。
    ///
    /// # Errors
    ///
    /// リードモデルを引けなかった場合。
    fn find(&self, target: &str) -> Result<Option<DoctorSummaryView>, ReadModelReadError>;
}
