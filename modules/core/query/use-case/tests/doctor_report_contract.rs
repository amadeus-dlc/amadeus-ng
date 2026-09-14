//! 自己診断の報告 (`DoctorReportUseCase`) の契約 — **行を引いて写すだけ**。
//!
//! 判定 (D1.a〜D5.b の成否・ラベル・原因、初回状態の非適用、advisory を失敗にしないこと、
//! 対象外行を出さないこと) はコマンド側の集約が所有し、その契約は
//! `core-command-domain` の `workspace_doctor_contract` が持つ (オーナー裁定 2026-09-12)。
//! ここが固定するのは、クエリ側が**判断を 1 つも持たない**ことである — 集計は行の写しで
//! あり、数え直さない (`coding-rules/cqrs-boundaries.md` 規則 6)。
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::missing_const_for_fn
)]

use std::cell::RefCell;
use std::io::ErrorKind;
use std::rc::Rc;

use core_query_use_case::orchestration::{
    DoctorCheck, DoctorCheckDao, DoctorReportDao, DoctorReportUseCase, DoctorSummaryView,
    ReadModelReadError,
};

const TARGET: &str = "spaces/default/intents";
const REPORT: &str = "workspace-doctor:0f";

/// 集計行のフェイク。引いた鍵を記録する。
struct FakeReportDao {
    row: Option<DoctorSummaryView>,
    fails: bool,
    asked: RefCell<Vec<String>>,
}

impl FakeReportDao {
    fn holding(row: DoctorSummaryView) -> Self {
        Self {
            row: Some(row),
            fails: false,
            asked: RefCell::new(Vec::new()),
        }
    }
    fn empty() -> Self {
        Self {
            row: None,
            fails: false,
            asked: RefCell::new(Vec::new()),
        }
    }
    fn failing() -> Self {
        Self {
            row: None,
            fails: true,
            asked: RefCell::new(Vec::new()),
        }
    }
}

impl FakeReportDao {
    /// 引かれた鍵 (記録順)。
    fn asked(&self) -> Vec<String> {
        self.asked.borrow().clone()
    }
}

/// ユースケースは DAO を所有するので、鍵の記録を外から読むために共有参照ごと注入する。
struct Shared<T>(Rc<T>);

impl DoctorReportDao for Shared<FakeReportDao> {
    fn find(&self, target: &str) -> Result<Option<DoctorSummaryView>, ReadModelReadError> {
        self.0.asked.borrow_mut().push(target.to_string());
        if self.0.fails {
            return Err(ReadModelReadError::new(ErrorKind::WouldBlock, None));
        }
        Ok(self.0.row.clone())
    }
}

/// 検査行のフェイク。引いた FK を記録する。
struct FakeCheckDao {
    rows: Vec<DoctorCheck>,
    fails: bool,
    asked: RefCell<Vec<String>>,
}

impl FakeCheckDao {
    fn holding(rows: Vec<DoctorCheck>) -> Self {
        Self {
            rows,
            fails: false,
            asked: RefCell::new(Vec::new()),
        }
    }
    fn failing() -> Self {
        Self {
            rows: Vec::new(),
            fails: true,
            asked: RefCell::new(Vec::new()),
        }
    }
}

impl FakeCheckDao {
    /// 引かれた FK (記録順)。
    fn asked(&self) -> Vec<String> {
        self.asked.borrow().clone()
    }
}

impl DoctorCheckDao for Shared<FakeCheckDao> {
    fn find(&self, report_id: &str) -> Result<Vec<DoctorCheck>, ReadModelReadError> {
        self.0.asked.borrow_mut().push(report_id.to_string());
        if self.0.fails {
            return Err(ReadModelReadError::new(ErrorKind::InvalidData, None));
        }
        Ok(self.0.rows.clone())
    }
}

/// 組んだユースケースと、鍵の記録を読むための 2 つの共有参照。
type Wired = (
    DoctorReportUseCase<Shared<FakeReportDao>, Shared<FakeCheckDao>>,
    Rc<FakeReportDao>,
    Rc<FakeCheckDao>,
);

/// 2 つのフェイクを共有したままユースケースを組む。
fn wire(report: FakeReportDao, checks: FakeCheckDao) -> Wired {
    let report = Rc::new(report);
    let checks = Rc::new(checks);
    (
        DoctorReportUseCase::new(Shared(Rc::clone(&report)), Shared(Rc::clone(&checks))),
        report,
        checks,
    )
}

fn check(check_id: &str, passed: bool, label: &str, fix: Option<&str>) -> DoctorCheck {
    DoctorCheck::new(
        check_id.to_string(),
        passed,
        label.to_string(),
        fix.map(str::to_string),
    )
}

fn rows() -> Vec<DoctorCheck> {
    vec![
        check("D1.a", true, "bun installed", None),
        check(
            "D2.f",
            false,
            "Hooks have never executed",
            Some("1. Run /hooks"),
        ),
        check("D3.a", true, "workspace shell ready", None),
    ]
}

#[test]
fn the_report_is_the_row_of_the_target_with_its_checks_in_stored_order() {
    let (use_case, _, _) = wire(
        FakeReportDao::holding(DoctorSummaryView::new(REPORT.to_string(), 2, 1, 1)),
        FakeCheckDao::holding(rows()),
    );
    let report = use_case.execute(TARGET).unwrap().unwrap();
    assert_eq!(report.checks(), rows().as_slice());
    assert_eq!(report.passed(), 2);
    assert_eq!(report.failed(), 1);
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn the_counts_come_from_the_row_and_are_never_recomputed_from_the_checks() {
    // 行と検査行が食い違っても、クエリ側は**行の値を写す**。数え直した瞬間に判断が生える。
    let (use_case, _, _) = wire(
        FakeReportDao::holding(DoctorSummaryView::new(REPORT.to_string(), 41, 7, 0)),
        FakeCheckDao::holding(rows()),
    );
    let report = use_case.execute(TARGET).unwrap().unwrap();
    assert_eq!(report.checks().len(), 3);
    assert_eq!(report.passed(), 41);
    assert_eq!(report.failed(), 7);
    assert_eq!(report.exit_code(), 0, "終了コードも行の写しである");
}

#[test]
fn the_check_rows_are_pulled_by_the_foreign_key_of_the_summary_row() {
    let (use_case, report_dao, check_dao) = wire(
        FakeReportDao::holding(DoctorSummaryView::new(REPORT.to_string(), 3, 0, 0)),
        FakeCheckDao::holding(rows()),
    );
    use_case.execute(TARGET).unwrap().unwrap();
    assert_eq!(report_dao.asked(), vec![TARGET.to_string()]);
    assert_eq!(check_dao.asked(), vec![REPORT.to_string()]);
}

#[test]
fn a_target_that_has_no_row_is_absent_rather_than_an_empty_report() {
    let (use_case, _, check_dao) = wire(FakeReportDao::empty(), FakeCheckDao::holding(rows()));
    assert!(use_case.execute(TARGET).unwrap().is_none());
    assert!(
        check_dao.asked().is_empty(),
        "集計行が無ければ検査行も引かない"
    );
}

#[test]
fn a_summary_read_failure_is_surfaced_rather_than_reported_as_absent() {
    let (use_case, _, _) = wire(FakeReportDao::failing(), FakeCheckDao::holding(rows()));
    let error = use_case.execute(TARGET).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::WouldBlock);
}

#[test]
fn a_check_read_failure_is_surfaced_rather_than_a_partial_report() {
    let (use_case, _, _) = wire(
        FakeReportDao::holding(DoctorSummaryView::new(REPORT.to_string(), 3, 0, 0)),
        FakeCheckDao::failing(),
    );
    let error = use_case.execute(TARGET).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidData);
}

#[test]
fn the_labels_and_fixes_are_carried_verbatim() {
    let (use_case, _, _) = wire(
        FakeReportDao::holding(DoctorSummaryView::new(REPORT.to_string(), 2, 1, 1)),
        FakeCheckDao::holding(rows()),
    );
    let report = use_case.execute(TARGET).unwrap().unwrap();
    assert_eq!(report.checks()[1].check_id(), "D2.f");
    assert_eq!(report.checks()[1].label(), "Hooks have never executed");
    assert_eq!(report.checks()[1].fix(), Some("1. Run /hooks"));
    assert_eq!(report.checks()[0].fix(), None);
}
