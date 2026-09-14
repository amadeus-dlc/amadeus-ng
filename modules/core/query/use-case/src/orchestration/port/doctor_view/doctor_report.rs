//! 自己診断の報告 DTO — 表示順の行と、焼き込まれた集計。
//!
//! ユースケースが 2 表を FK でたどって組む View である (`coding-rules/cqrs-boundaries.md`
//! 規則 6「ユースケースは FK をたどって表ごとに引き、View を組む」)。行の値も集計も
//! 集約の答えの写しであり、この型は判断を 1 つも持たない。

use super::DoctorCheck;

/// 1 回の診断の報告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    checks: Vec<DoctorCheck>,
    passed: u64,
    failed: u64,
    exit_code: u8,
}

impl DoctorReport {
    /// 行と集計を束ねる (**唯一の構築経路**)。
    #[must_use]
    pub const fn new(checks: Vec<DoctorCheck>, passed: u64, failed: u64, exit_code: u8) -> Self {
        Self {
            checks,
            passed,
            failed,
            exit_code,
        }
    }

    /// 表示順の行。
    #[must_use]
    pub fn checks(&self) -> &[DoctorCheck] {
        &self.checks
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
