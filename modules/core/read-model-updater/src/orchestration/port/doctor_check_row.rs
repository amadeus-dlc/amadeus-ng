//! `read_doctor_check` の 1 行 — 表示順の診断行。

use crate::read_tables::doctor_check;

/// `read_doctor_check` の 1 行。
///
/// 主キーは自然キー (`report_id` × `position`) から導いた代理キーで、FK 列 `report_id` が
/// `read_doctor_report` の行を指す。`check_id` / `passed` / `label` / `fix` は集約が決めた
/// 行の写しである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorCheckRow {
    id: String,
    report_id: String,
    position: usize,
    check_id: String,
    passed: bool,
    label: String,
    fix: Option<String>,
}

impl DoctorCheckRow {
    /// 行の値を束ねる。主キーは `report_id` と `position` から導く
    /// (`read_tables` の代理キーの作り方を 1 箇所から借りる)。
    #[must_use]
    pub fn new(
        report_id: String,
        position: usize,
        check_id: String,
        passed: bool,
        label: String,
        fix: Option<String>,
    ) -> Self {
        Self {
            id: doctor_check(&report_id, position),
            report_id,
            position,
            check_id,
            passed,
            label,
            fix,
        }
    }

    /// 主キー。DAO が列へ書く境界のための読取。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// FK — 属する報告の id。DAO が列へ書く境界のための読取。
    #[must_use]
    pub fn report_id(&self) -> &str {
        &self.report_id
    }

    /// 表示順の位置 (0 起点)。DAO が列へ書く境界のための読取。
    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }

    /// 検査の識別子 (`D1.a` など)。DAO が列へ書く境界のための読取。
    #[must_use]
    pub fn check_id(&self) -> &str {
        &self.check_id
    }

    /// 合格したか。DAO が列へ書く境界のための読取。
    #[must_use]
    pub const fn is_passed(&self) -> bool {
        self.passed
    }

    /// 表示ラベル。DAO が列へ書く境界のための読取。
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// 修復案 (失敗行だけが持つ)。DAO が列へ書く境界のための読取。
    #[must_use]
    pub fn fix(&self) -> Option<&str> {
        self.fix.as_deref()
    }
}
