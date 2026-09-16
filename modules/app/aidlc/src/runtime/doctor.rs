//! `aidlc --doctor` — 自己診断 (契約 C7) の配線。
//!
//! # 流れ (オーナー裁定 2026-09-12)
//!
//! 「doctor は、コマンド側集約が処理してイベントを吐き出し、RMU がイベントからリードモデルを
//! 作り、クエリ側がその結果を表示」である。
//!
//! ```text
//! 観測  DoctorObservationDaoImpl (クエリ側 IA) → DoctorObservationView
//! 写像  observation::to_domain                 → workspace::DoctorObservation
//! 書込  DiagnoseWorkspaceUseCase               → WorkspaceDoctor → WorkspaceDoctorEvent → store
//! 投影  WorkspaceDoctorReadModelUpdater        → read_doctor_report / read_doctor_check
//! 読取  DoctorReportUseCase (DAO 2 本)         → DoctorReport
//! 描画  render                                 → C7 の書式
//! ```
//!
//! 両側を知ってよいのは RMU と合成ルートだけなので、観測 View → ドメインの写像はここに置く
//! (`coding-rules/cqrs-boundaries.md`)。
//!
//! # 診断の事実は一時ストアに置く
//!
//! 診断集約のイベントはプロセス内の SQLite (共有キャッシュのメモリ DB) へ書く。C7 は
//! 診断が正本を修復・再初期化しないこと、初回状態でファイル・イベントを一切作らないこと
//! (DC1) を定めており、DC10 は空間ストアの journal への書込が失敗しても診断出力が残ることを
//! 定める。どちらも「診断は正本のストアへ書かない」ことを要求するので、集約 → イベント →
//! RMU → クエリの流れは**毎回**この一時ストアの上で回す。
//!
//! 永続する事実は従来どおり `HEALTH_CHECKED` 1 件だけである — 起動時に監査シャードがある
//! 記録のときに U2 の更新コマンド (`RecordHealthCheckUseCase`) 経由で記録し、記録後の投影も
//! 通常の RMU 反映だけで `restore_missing_files` は呼ばない。

use std::path::PathBuf;

use chrono::Utc;
use core_command_domain::orchestration::HealthCheckResult;
use core_command_domain::workspace::{
    HookHealthTarget, IntentDirName, SpaceName, StateVersionClassification, StateVersionKind,
    StorePath,
};
use core_command_interface_adapter::orchestration::{
    IntentExecutionRepositoryImpl, WorkspaceDoctorRepositoryImpl,
};
use core_command_use_case::orchestration::{DiagnoseWorkspaceUseCase, RecordHealthCheckUseCase};
use core_query_interface_adapter::{
    DoctorEnvironment, DoctorObservationDaoImpl, DoctorPaths, NativeDoctorFacts, ReadModelDaos,
    StateVersionClassifier,
};
use core_query_use_case::orchestration::{
    DoctorObservationDao as _, DoctorReport, DoctorReportUseCase, StateVersionKindView,
    StateVersionView,
};
use core_read_model_updater::orchestration::WorkspaceDoctorReadModelUpdater;

mod observation;

use super::{Completion, NATIVE_HOOKS, active_execution, catch_up_with, store_path};
use crate::cli::{EngineRoute, Face, Request, parse};
use crate::layout::Layout;
use crate::wording;

/// C7 の区切り線 (`─` × 37)。
const RULE_WIDTH: usize = 37;

/// `aidlc --doctor` の 1 回分。
pub(super) async fn run(layout: &Layout, extra: &[String]) -> Completion {
    if !extra.is_empty() {
        return Completion::refused(wording::doctor_takes_no_arguments(extra));
    }
    let store = match store_path(layout) {
        Ok(store) => store,
        Err(message) => return Completion::refused(message),
    };
    let target = match diagnosed_target(layout) {
        Ok(target) => target,
        Err(message) => return Completion::refused(message),
    };
    let report = match diagnose(layout, &store, &target).await {
        Ok(report) => report,
        Err(message) => return Completion::refused(message),
    };
    let text = render(&report);
    let code = report.exit_code();
    // 本家 4669–4682 / 4737–4746: 起動時に監査シャードがある記録だけが診断実施を記録する。
    if !audit_exists(layout) {
        return Completion::reported(text, code);
    }
    match record_health_check(layout, &store, &report).await {
        Ok(()) => Completion::reported(text, code),
        Err(diagnostic) => Completion::reported_then_refused(text, diagnostic),
    }
}

/// 診断対象 — space と、選ばれている記録。
fn diagnosed_target(layout: &Layout) -> Result<HookHealthTarget, String> {
    let space = SpaceName::parse(layout.space())
        .map_err(|_| wording::invalid_active_space(layout.space()))?;
    let record = layout.record_dir().and_then(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| IntentDirName::parse(name).ok())
    });
    Ok(HookHealthTarget::new(space, record))
}

/// 観測 → 集約 → イベント → 投影 → 行の読取を 1 本の一時ストアの上で回す。
///
/// 一時ストアはリポジトリが開いている間だけ存在するので、`use_case` (リポジトリの所有者) を
/// 行の読取が終わるまで生かしておく必要がある。
async fn diagnose(
    layout: &Layout,
    store: &StorePath,
    target: &HookHealthTarget,
) -> Result<DoctorReport, String> {
    let paths = DoctorPaths::new(
        layout.project_dir().to_path_buf(),
        layout.space().to_string(),
        layout.record_dir().map(std::path::Path::to_path_buf),
        store.as_path().to_path_buf(),
    );
    let observed = DoctorObservationDaoImpl::new(
        paths,
        environment(),
        native_facts(),
        DomainStateVersionClassifier,
    )
    .find()
    .map_err(|error| wording::orchestrate_failure(&error.to_string()))?;

    let repository = WorkspaceDoctorRepositoryImpl::open_ephemeral()
        .map_err(|error| wording::orchestrate_failure(&format!("diagnosis store: {error}")))?;
    let location = repository.location().to_path_buf();
    let mut use_case = DiagnoseWorkspaceUseCase::new(repository);
    use_case
        .execute(target, &observation::to_domain(&observed), Utc::now())
        .await
        .map_err(|error| wording::orchestrate_failure(&format!("diagnosis: {error}")))?;

    WorkspaceDoctorReadModelUpdater::open(&location)
        .and_then(|mut updater| updater.catch_up())
        .map_err(|error| wording::orchestrate_failure(&format!("diagnosis projection: {error}")))?;

    let daos = ReadModelDaos::open(&location)
        .map_err(|error| wording::orchestrate_failure(&error.to_string()))?;
    let report = DoctorReportUseCase::new(daos.doctor_report(), daos.doctor_check())
        .execute(&target.relative_directory())
        .map_err(|error| wording::orchestrate_failure(&error.to_string()))?
        .ok_or_else(|| {
            wording::orchestrate_failure(
                "doctor: the diagnosis was not projected for this workspace",
            )
        })?;
    drop(use_case);
    Ok(report)
}

/// 記録木の根 (`docsRoot`) の `audit/*.md` が 1 つでもあるか (本家 `auditShards(...).length > 0`)。
fn audit_exists(layout: &Layout) -> bool {
    let audit = layout
        .audit_dir()
        .unwrap_or_else(|| layout.intents_dir().join("audit"));
    std::fs::read_dir(audit).is_ok_and(|entries| {
        entries
            .flatten()
            .any(|entry| entry.path().extension().is_some_and(|ext| ext == "md"))
    })
}

/// 診断事実を U2 の更新コマンドへ渡し、通常の RMU 反映で `HEALTH_CHECKED` を投影する。
/// 失敗は C2 の共通エラー経路の文言で返す (成功したふりをしない)。
async fn record_health_check(
    layout: &Layout,
    store: &StorePath,
    report: &DoctorReport,
) -> Result<(), String> {
    let cursor = active_execution(layout)
        .map_err(|error| wording::unreadable_execution_cursor(&error.to_string()))?
        .ok_or_else(|| {
            wording::orchestrate_failure(
                "doctor: the active record has an audit trail but no execution cursor to record the health check",
            )
        })?;
    let repository = IntentExecutionRepositoryImpl::open(store)
        .map_err(|error| wording::orchestrate_failure(&format!("health check record: {error}")))?;
    RecordHealthCheckUseCase::new(repository)
        .execute(
            cursor.execution_id(),
            HealthCheckResult::new(report.passed(), report.failed()),
            Utc::now(),
        )
        .await
        .map_err(|error| wording::orchestrate_failure(&format!("health check record: {error}")))?;
    catch_up_with(layout, false)
        .await
        .map_err(|cause| wording::orchestrate_failure(&cause))
}

/// 合成ルートが `std::env` から写す環境 (DAO はプロセス環境を直接読まない)。
fn environment() -> DoctorEnvironment {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    DoctorEnvironment::new(
        std::env::var_os("PATH"),
        home,
        std::env::var_os("AIDLC_MANAGED_SETTINGS_PATH")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from),
    )
}

/// この build 自身の事実 — 実行中バイナリと、配線表 (`cli::parse` / [`NATIVE_HOOKS`]) の照合。
fn native_facts() -> NativeDoctorFacts {
    NativeDoctorFacts::new(
        std::env::current_exe().map_err(|error| error.to_string()),
        missing_entry_points(),
        NATIVE_HOOKS
            .iter()
            .map(|name| (*name).to_string())
            .collect(),
    )
}

/// セルフホストで使う入口 (面 × 動詞) のうち、この build の配線表に無いもの。
///
/// 入口を**実行しない** — argv → 要求の写像だけを引き、未知動詞・未配線動詞へ落ちる入口を
/// 名指す。フック面は [`NATIVE_HOOKS`] の名前がそのまま入口である。
fn missing_entry_points() -> Vec<String> {
    let probes: [(Face, &[&str], &str); 14] = [
        (Face::Orchestrate, &["next"], "aidlc next"),
        (Face::Orchestrate, &["continue", "token"], "aidlc continue"),
        (Face::Orchestrate, &["report"], "aidlc report"),
        (Face::Orchestrate, &["park"], "aidlc park"),
        (Face::Orchestrate, &["--doctor"], "aidlc --doctor"),
        (
            Face::Utility,
            &["intent-create"],
            "aidlc-utility intent-create",
        ),
        (Face::Log, &["answer"], "aidlc-log answer"),
        (Face::Log, &["decision"], "aidlc-log decision"),
        (Face::Log, &["review"], "aidlc-log review"),
        (Face::Log, &["link"], "aidlc-log link"),
        (
            Face::State,
            &["practices-promote"],
            "aidlc-state practices-promote",
        ),
        (Face::Bolt, &["set-autonomy"], "aidlc-bolt set-autonomy"),
        (Face::Learnings, &["surface"], "aidlc-learnings surface"),
        (Face::Learnings, &["persist"], "aidlc-learnings persist"),
    ];
    let mut missing: Vec<String> = probes
        .iter()
        .filter(|(face, args, _)| {
            let argv: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
            !is_wired(&parse(*face, &argv))
        })
        .map(|(_, _, display)| (*display).to_string())
        .collect();
    missing.extend(missing_two_stage_entry_points());
    for hook in NATIVE_HOOKS {
        let argv = vec!["hook".to_string(), hook.to_string()];
        if !matches!(parse(Face::Orchestrate, &argv), Request::Hook { name } if name == hook) {
            missing.push(format!("aidlc hook {hook}"));
        }
    }
    missing
}

/// bugfix 1 周が踏む入口の正本（`scripts/aidlc-selfhost/required-surface.json`）。
///
/// **コンパイル時に埋め込む。** doctor が照合するのは、実行時のワークスペースの内容では
/// なく**この build 自身の配線**だからである（D1.b は「この build の入口」の診断であり、
/// 手元のリポジトリの中身を見に行くと、別のワークスペースで走らせたときに意味が変わる）。
const REQUIRED_SURFACE: &str =
    include_str!("../../../../../scripts/aidlc-selfhost/required-surface.json");

/// bugfix 必須と実測した二段形の入口のうち、この build の配線表に無いもの（D13 の `WT-2`）。
///
/// 入口を**実行しない** — 二段形を [`EngineRoute::resolve`] に通し、既存の面へ写せた
/// （`Mapped`）ものだけを `parse` + [`is_wired`] で見る。写せなかったもの（`NotWired` /
/// `Unknown`）はそこで不足である。
fn missing_two_stage_entry_points() -> Vec<String> {
    missing_from(REQUIRED_SURFACE)
}

/// 与えた必要集合に対して同じ照合を行う。
///
/// probe の出所を引数に開いておくのは、**この照合そのものを観測できるようにする**ためである
/// （`missing_two_stage_entry_points` は不足分しか返さないので、出所が正しいかを結果からは
/// 読み取れない）。
fn missing_from(surface: &str) -> Vec<String> {
    let Ok(surface) = serde_json::from_str::<serde_json::Value>(surface) else {
        return vec![wording::REQUIRED_SURFACE_UNREADABLE.to_string()];
    };
    let Some(entries) = surface.get("entries").and_then(serde_json::Value::as_array) else {
        return vec![wording::REQUIRED_SURFACE_UNREADABLE.to_string()];
    };
    let mut missing = Vec::new();
    for entry in entries {
        if entry.get("bugfix_required") != Some(&serde_json::Value::Bool(true)) {
            continue;
        }
        let Some(noun) = entry.get("noun").and_then(serde_json::Value::as_str) else {
            missing.push(wording::REQUIRED_SURFACE_UNREADABLE.to_string());
            continue;
        };
        let verb = entry.get("verb").and_then(serde_json::Value::as_str);
        let mut argv = vec!["engine".to_string(), noun.to_string()];
        argv.extend(verb.map(str::to_string));
        let resolved = matches!(
            EngineRoute::resolve(&argv),
            Some(EngineRoute::Mapped { face, argv: target }) if is_wired(&parse(face, &target))
        );
        if !resolved {
            missing.push(wording::engine_entry_point(noun, verb));
        }
    }
    missing
}

/// 未知動詞・未配線動詞へ落ちない要求か。
const fn is_wired(request: &Request) -> bool {
    !matches!(
        request,
        Request::UnknownOrchestrateVerb { .. }
            | Request::UnknownUtilityVerb { .. }
            | Request::UnknownLogVerb { .. }
            | Request::StateNotWired { .. }
            | Request::UnknownStateVerb { .. }
            | Request::BoltNotWired { .. }
            | Request::UnknownBoltVerb { .. }
            | Request::UnknownLearningsVerb { .. }
    )
}

/// U2 の分類器 (`StateVersionClassification`) を診断へ注入する — 分類規則は 1 箇所のまま、
/// 本家分類の message (`wording`) を添えて Query 側の写しへ変換する。
struct DomainStateVersionClassifier;

impl StateVersionClassifier for DomainStateVersionClassifier {
    fn classify(&self, state_content: &str) -> StateVersionView {
        let classified = StateVersionClassification::classify(state_content);
        let version = classified.version().map(str::to_string);
        let raw = classified.version().unwrap_or_default();
        let (kind, message) = match classified.kind() {
            StateVersionKind::Ok => (StateVersionKindView::Ok, None),
            StateVersionKind::Unparseable => (
                StateVersionKindView::Unparseable,
                Some(wording::INCOMPATIBLE_STATE_UNPARSEABLE.to_string()),
            ),
            StateVersionKind::Past => (
                StateVersionKindView::Past,
                Some(wording::incompatible_state_past(raw)),
            ),
            StateVersionKind::Future => (
                StateVersionKindView::Future,
                Some(wording::incompatible_state_future(raw)),
            ),
        };
        StateVersionView::new(kind, version, message)
    }
}

/// C7「出力と終了コード」の描画 (本家 4692–4736 の写し。末尾の LF は `main` が付ける)。
fn render(report: &DoctorReport) -> String {
    let rule = "\u{2500}".repeat(RULE_WIDTH);
    let mut out = format!("AI-DLC Health Check\n{rule}\n");
    for check in report.checks() {
        if check.is_passed() {
            out.push_str("\u{2713}  ");
            out.push_str(check.label());
        } else {
            out.push_str("\u{2717}  ");
            out.push_str(check.label());
            if let Some(fix) = check.fix() {
                out.push_str(" \u{2014} ");
                out.push_str(fix);
            }
        }
        out.push('\n');
    }
    out.push_str(&rule);
    out.push('\n');
    out.push_str(&format!(
        "{} passed, {} failed",
        report.passed(),
        report.failed()
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_query_use_case::orchestration::DoctorCheck;

    #[test]
    fn every_selfhost_entry_point_is_wired_in_this_build() {
        assert_eq!(missing_entry_points(), Vec::<String>::new());
    }

    /// 二段形の確認対象は `required-surface.json` の bugfix 必須集合そのものである。
    ///
    /// 出所を固定するだけでは「加え忘れ」を検出できないので、集合の件数と、今回配線した
    /// 入口が確認対象に入っていることを、埋め込んだ正本から直接数えて突き合わせる。
    #[test]
    fn the_two_stage_probes_are_the_bugfix_required_set_of_the_frozen_surface() {
        let surface: serde_json::Value =
            serde_json::from_str(REQUIRED_SURFACE).expect("埋め込んだ必要集合は JSON である");
        let required: Vec<(String, Option<String>)> = surface
            .get("entries")
            .and_then(serde_json::Value::as_array)
            .expect("entries")
            .iter()
            .filter(|entry| entry.get("bugfix_required") == Some(&serde_json::Value::Bool(true)))
            .map(|entry| {
                (
                    entry
                        .get("noun")
                        .and_then(serde_json::Value::as_str)
                        .expect("noun")
                        .to_string(),
                    entry
                        .get("verb")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string),
                )
            })
            .collect();
        assert_eq!(required.len(), 26, "bugfix 必須集合の件数が変わった");
        for expected in [
            ("review-brief", Some("context")),
            ("review-brief", Some("review")),
            ("review-brief", Some("summary")),
            ("testing-posture", Some("brief")),
            ("statusline", None),
        ] {
            assert!(
                required
                    .iter()
                    .any(|(noun, verb)| noun == expected.0 && verb.as_deref() == expected.1),
                "{expected:?} が確認対象に入っていない"
            );
        }
        assert_eq!(missing_two_stage_entry_points(), Vec::<String>::new());
    }

    /// 配線の無い二段形の入口は、その綴りのまま不足として名指される。
    ///
    /// 照合が本当に働いていることの証拠である — 常に空を返す実装ならこの行は出ない。
    #[test]
    fn a_two_stage_entry_this_build_does_not_wire_is_reported_as_missing() {
        let surface = r#"{"entries":[
            {"noun":"state","verb":"practices-event","bugfix_required":true},
            {"noun":"frobnicate","verb":"run","bugfix_required":true},
            {"noun":"recompose","verb":null,"bugfix_required":true},
            {"noun":"orchestrate","verb":"next","bugfix_required":true},
            {"noun":"state","verb":"fork","bugfix_required":false}
        ]}"#;
        assert_eq!(
            missing_from(surface),
            vec![
                "aidlc engine state practices-event".to_string(),
                "aidlc engine frobnicate run".to_string(),
                "aidlc engine recompose".to_string(),
            ]
        );
    }

    /// 読めない必要集合は合格へ倒さない。
    #[test]
    fn an_unreadable_required_surface_is_reported_rather_than_passed() {
        for broken in ["{", r#"{"no-entries": true}"#] {
            assert_eq!(
                missing_from(broken),
                vec![wording::REQUIRED_SURFACE_UNREADABLE.to_string()]
            );
        }
    }

    #[test]
    fn the_report_is_drawn_in_the_c7_shape_without_the_trailing_newline() {
        let report = DoctorReport::new(
            vec![
                DoctorCheck::new("D1.a".to_string(), true, "ok row".to_string(), None),
                DoctorCheck::new(
                    "D1.b".to_string(),
                    false,
                    "bad row".to_string(),
                    Some("do this".to_string()),
                ),
            ],
            1,
            1,
            1,
        );
        assert_eq!(
            render(&report),
            format!(
                "AI-DLC Health Check\n{rule}\n\u{2713}  ok row\n\u{2717}  bad row \u{2014} do this\n{rule}\n1 passed, 1 failed",
                rule = "\u{2500}".repeat(37)
            )
        );
    }

    #[test]
    fn the_domain_classifier_carries_the_upstream_message_for_incompatible_versions() {
        let past = DomainStateVersionClassifier
            .classify("# AI-DLC State Tracking\n\n- **State Version**: 7\n");
        assert_eq!(past.kind(), StateVersionKindView::Past);
        assert_eq!(past.version(), Some("7"));
        assert_eq!(
            past.message(),
            Some(wording::incompatible_state_past("7").as_str())
        );
        let ok = DomainStateVersionClassifier
            .classify("# AI-DLC State Tracking\n\n- **State Version**: 8\n");
        assert_eq!(ok.kind(), StateVersionKindView::Ok);
        assert_eq!(ok.message(), None);
        let unparseable = DomainStateVersionClassifier.classify("# no row\n");
        assert_eq!(unparseable.kind(), StateVersionKindView::Unparseable);
        assert_eq!(
            unparseable.message(),
            Some(wording::INCOMPATIBLE_STATE_UNPARSEABLE)
        );
    }
}
