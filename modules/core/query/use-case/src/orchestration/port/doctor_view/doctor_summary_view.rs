//! 自己診断の集計の DTO — `read_doctor_report` の行の写し。
//!
//! 集計 (`passed` / `failed` / 終了コード) は RMU が集約のクエリの答えを焼き込んだ列で
//! あり、クエリ側は数えない (`coding-rules/cqrs-boundaries.md` 規則 6)。

/// `read_doctor_report` の 1 行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorSummaryView {
    report_id: String,
    passed: u64,
    failed: u64,
    exit_code: u8,
}

impl DoctorSummaryView {
    /// 行を束ねる (**唯一の構築経路**)。
    #[must_use]
    pub const fn new(report_id: String, passed: u64, failed: u64, exit_code: u8) -> Self {
        Self {
            report_id,
            passed,
            failed,
            exit_code,
        }
    }

    /// 行の主キー — 検査行の FK (`read_doctor_check.report_id`) が指す先。
    #[must_use]
    pub fn report_id(&self) -> &str {
        &self.report_id
    }

    /// 成功行の数。
    #[must_use]
    pub const fn passed(&self) -> u64 {
        self.passed
    }

    /// 必須失敗行の数。
    #[must_use]
    pub const fn failed(&self) -> u64 {
        self.failed
    }

    /// `failed = 0` なら 0、それ以外は 1。
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        self.exit_code
    }
}
