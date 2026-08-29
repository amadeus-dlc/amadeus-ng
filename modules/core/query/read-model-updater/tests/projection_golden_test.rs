//! 投影の**ゴールデン逐語一致** — 1 ドメインイベントから監査行と状態ファイル差分の両面を描き、
//! upstream の実バイト（`audit.md` と `state.diff`）と突き合わせる（FR1.1 / NFR3）。
//!
//! `state.diff` は unified diff なので、ハンクから「前」と「後」の断片を組み立て直し、前の断片へ
//! 投影を当てた結果が後の断片と 1 バイトも違わないことを見る。状態ファイル writer は行単位で
//! 働くので、ハンクの断片だけでも観測は成立する。
//!
//! # ここに無いイベント
//!
//! `GateApproved` / `StageSkipped` / `Jumped` / `StageCompleted` / `Started` / `Recomposed` の
//! 状態面は、ステージ番号・ステージ表題・`lead_agent` といった**ワークフロー定義側の材料**を
//! 要する（ゴールデン `cli/skip/skipped/state.diff` の `Active Agent` / `Next Action`、
//! `cli/recompose/skip-one/state.diff` の `4.5 (incident-response)` を参照）。ドメインイベントは
//! 定義を `definition_id` + `definition_revision` で間接参照するだけなので（ADR-008）、
//! 投影核だけでは描けない。裁定待ちの未実装であり、投影核は誤ったバイトを書く代わりに
//! `ProjectionError::DefinitionLookupRequired` で止まる。

// テストコードでは unwrap / expect / panic を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chrono::{DateTime, SecondsFormat, Utc};
use core_domain::orchestration::IntentId;
use core_domain::orchestration::{
    GateOpened, GateRejected, Parked, StageRevised, WorkflowExecutionEvent,
};
use core_domain::workflow_definition::StageSlug;
use core_query_read_model_updater::orchestration::{GlobalSeqNr, JournalEntry};
use core_query_read_model_updater::workspace::{ReadModel, project};

/// ゴールデンが正規化で潰した実行時値の置き換え先。
const TS_PLACEHOLDER: &str = "<TS>";

/// 投影に渡す発生時刻（正規化で `<TS>` に潰れるので値そのものは観測されない）。
const AT: &str = "2026-08-22T13:43:00Z";

/// 行を運ぶ集約識別子（投影は識別子を描かないので値は任意）。
const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

fn golden(case: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../tests/golden/upstream-3c3146cf/cli")
        .join(case)
}

fn at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(AT)
        .expect("固定の ISO 8601")
        .with_timezone(&Utc)
}

fn slug(value: &str) -> StageSlug {
    StageSlug::parse(value).expect("テストの slug は文法内")
}

fn entry(event: WorkflowExecutionEvent) -> JournalEntry {
    JournalEntry::new(
        GlobalSeqNr::new(1),
        IntentId::parse(INTENT).expect("UUIDv7"),
        1,
        at(),
        event,
    )
}

/// 実測値へゴールデンと同じ正規化（ISO 8601 UTC → `<TS>`）を当てる。
fn normalise(rendered: &str) -> String {
    rendered.replace(
        &at().to_rfc3339_opts(SecondsFormat::Secs, true),
        TS_PLACEHOLDER,
    )
}

/// unified diff のハンクから「前」と「後」の断片を組み立てる。
fn before_and_after(diff: &str) -> (String, String) {
    let mut before = String::new();
    let mut after = String::new();
    let mut in_hunk = false;
    for line in diff.lines() {
        if line.starts_with("@@") {
            in_hunk = true;
            continue;
        }
        if !in_hunk || line.starts_with("---") || line.starts_with("+++") {
            continue;
        }
        match line.split_at_checked(1) {
            Some(("-", rest)) => {
                before.push_str(rest);
                before.push('\n');
            }
            Some(("+", rest)) => {
                after.push_str(rest);
                after.push('\n');
            }
            Some((" ", rest)) => {
                before.push_str(rest);
                before.push('\n');
                after.push_str(rest);
                after.push('\n');
            }
            _ => {}
        }
    }
    (before, after)
}

/// 1 ケースを検収する — 監査行と状態ファイルの**両面**をバイトで突き合わせる。
fn assert_case(case: &str, event: WorkflowExecutionEvent) {
    let dir = golden(case);
    let expected_audit = std::fs::read_to_string(dir.join("audit.md")).expect("audit.md");
    let diff = std::fs::read_to_string(dir.join("state.diff")).expect("state.diff");
    let (before, expected_state) = before_and_after(&diff);

    let mut model = ReadModel::new(before);
    project(&[entry(event)], &mut model).unwrap_or_else(|error| {
        panic!("{case}: 投影が失敗した: {error}");
    });

    assert_eq!(
        normalise(model.appended_audit()),
        expected_audit,
        "{case}: 監査行が upstream と違う"
    );
    assert_eq!(
        normalise(model.state()),
        expected_state,
        "{case}: 状態ファイルが upstream と違う"
    );
}

#[test]
fn opening_a_gate_writes_the_awaiting_approval_row_and_moves_the_checkbox() {
    assert_case(
        "report/awaiting-approval",
        WorkflowExecutionEvent::GateOpened(GateOpened::new(slug("practices-discovery"), vec![])),
    );
}

#[test]
fn rejecting_a_gate_writes_two_rows_and_bumps_the_revision_count() {
    assert_case(
        "report/rejected",
        WorkflowExecutionEvent::GateRejected(GateRejected::new(
            slug("practices-discovery"),
            Some("Sharpen the testing posture.".to_string()),
            1,
        )),
    );
}

#[test]
fn revising_a_stage_re_enters_the_gate_with_the_verbatim_details() {
    assert_case(
        "report/revised",
        WorkflowExecutionEvent::StageRevised(StageRevised::new(slug("practices-discovery"))),
    );
}

#[test]
fn parking_writes_the_marker_at_the_end_of_the_runtime_section() {
    assert_case(
        "park/park",
        WorkflowExecutionEvent::Parked(Parked::new(slug("domain-design"))),
    );
}

#[test]
fn unparking_removes_both_marker_lines() {
    assert_case("unpark/unpark", WorkflowExecutionEvent::Unparked);
}

#[test]
fn replaying_the_same_entry_twice_is_not_the_same_as_projecting_once() {
    // 冪等（NFR3）が保証するのは「同じチェックポイントから流し直せば同一バイト」であって、
    // 「同じイベントを 2 度渡しても 1 度と同じ」ではない — 二度描かないのはチェックポイントの
    // 仕事である。ここはその境界を明示しておくための固定である。
    let event =
        WorkflowExecutionEvent::GateOpened(GateOpened::new(slug("practices-discovery"), vec![]));
    let dir = golden("report/awaiting-approval");
    let diff = std::fs::read_to_string(dir.join("state.diff")).expect("state.diff");
    let (before, _) = before_and_after(&diff);

    let mut once = ReadModel::new(before.clone());
    project(&[entry(event.clone())], &mut once).expect("投影");
    let mut twice = ReadModel::new(before);
    project(&[entry(event.clone()), entry(event)], &mut twice).expect("投影");

    assert_eq!(once.state(), twice.state(), "状態面は同じ位置へ落ち着く");
    assert_eq!(
        twice.appended_audit(),
        format!("{}{}", once.appended_audit(), once.appended_audit()),
        "監査面は台帳なので 2 度分が並ぶ"
    );
}

#[test]
fn projecting_the_same_entries_from_the_same_state_twice_yields_the_same_bytes() {
    // NFR3 の本体 — 同じチェックポイントから何度流しても同一バイト。
    let event =
        WorkflowExecutionEvent::GateOpened(GateOpened::new(slug("practices-discovery"), vec![]));
    let dir = golden("report/awaiting-approval");
    let diff = std::fs::read_to_string(dir.join("state.diff")).expect("state.diff");
    let (before, _) = before_and_after(&diff);

    let run = |source: &str| {
        let mut model = ReadModel::new(source.to_string());
        project(&[entry(event.clone())], &mut model).expect("投影");
        (
            model.state().to_string(),
            model.appended_audit().to_string(),
        )
    };
    assert_eq!(run(&before), run(&before));
}

/// ゴールデンのディレクトリが本当にそこにあることの保険（パスずれで空回りしない）。
#[test]
fn the_golden_corpus_is_where_the_test_thinks_it_is() {
    let root: &Path = &golden("report/approved");
    assert!(root.join("audit.md").exists(), "実際: {}", root.display());
}
