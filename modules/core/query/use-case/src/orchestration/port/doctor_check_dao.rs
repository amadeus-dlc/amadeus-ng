//! 自己診断の行を引くポート — `read_doctor_check` を 1 表 1 引当で読む。

use super::{DoctorCheck, ReadModelReadError};

/// 集計行の主キー (FK `report_id`) で表示順の行を引く。
pub trait DoctorCheckDao {
    /// 報告に属する行を表示順で引く。行が無いのは失敗ではない (空の列が返る)。
    ///
    /// # Errors
    ///
    /// リードモデルを引けなかった場合。
    fn find(&self, report_id: &str) -> Result<Vec<DoctorCheck>, ReadModelReadError>;
}
