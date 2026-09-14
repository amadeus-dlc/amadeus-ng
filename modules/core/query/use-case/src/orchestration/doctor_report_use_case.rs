//! 自己診断の報告を**読むだけ**のユースケース。
//!
//! 判断 (D1.a〜D5.b の成否・ラベル・修復案) と集計は、コマンド側の集約
//! `WorkspaceDoctor` が決めて RMU が `read_doctor_*` へ焼き込んである。ここが持つのは
//! 「どのキーでどの表を引き、FK をたどって View を組むか」だけで、判断・導出・選択・
//! 文言組立は 1 つも無い (`coding-rules/cqrs-boundaries.md` 規則 6 の 2026-09-02 追記)。

use super::{DoctorCheckDao, DoctorReport, DoctorReportDao, ReadModelReadError};

/// 集計行と検査行の 2 つの DAO を保持し、`execute` で報告を組む読取専用のユースケース。
#[derive(Debug)]
pub struct DoctorReportUseCase<R, C> {
    report_dao: R,
    check_dao: C,
}

impl<R: DoctorReportDao, C: DoctorCheckDao> DoctorReportUseCase<R, C> {
    /// 2 つの読取ポートを注入する。
    #[must_use]
    pub const fn new(report_dao: R, check_dao: C) -> Self {
        Self {
            report_dao,
            check_dao,
        }
    }

    /// 対象の最新の診断を読む。まだ投影されていなければ `None`。
    ///
    /// # Errors
    ///
    /// リードモデルを引けなかった場合。
    pub fn execute(&self, target: &str) -> Result<Option<DoctorReport>, ReadModelReadError> {
        let Some(summary) = self.report_dao.find(target)? else {
            return Ok(None);
        };
        let checks = self.check_dao.find(summary.report_id())?;
        Ok(Some(DoctorReport::new(
            checks,
            summary.passed(),
            summary.failed(),
            summary.exit_code(),
        )))
    }
}
