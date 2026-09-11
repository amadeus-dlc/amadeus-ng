//! レビュー保護フックの入口 — 書込み凍結 (review-freeze) と reviewer の稼働観測
//! (reviewer-scope)。
//!
//! どちらも upstream と同じく**開いて通す** (fail-open) — 入力が読めない、台帳が無い、
//! 材料が引けないといった事情で人間の作業を止めない。止めるのは、凍結の 3 条件が
//! そろったと**判定できた**ときだけである。
use super::{Completion, Layout};
use crate::wording;
use chrono::Utc;
use core_command_domain::orchestration::ReviewerScopeVerdict;
use core_command_domain::workspace::{Checkboxes, HookHealthTarget, IntentDirName, SpaceName};
use core_command_interface_adapter::orchestration::{
    IntentExecutionRepositoryImpl, IntentRepositoryImpl, SessionAuditRepositoryImpl,
    WorkflowDefinitionRepositoryImpl,
};
use core_command_use_case::orchestration::{
    GuardReviewFreezeUseCase, GuardReviewerScopeUseCase, ReviewerScopeRequest,
};
use std::path::{Path, PathBuf};

/// 稼働記録・drop の名前 (フック名と同じ)。
const FREEZE: &str = "review-freeze";
const REVIEWER_SCOPE: &str = "reviewer-scope";

/// 助言を抑える窓 (upstream `10 * 60 * 1000`)。
const ADVISORY_WINDOW: std::time::Duration = std::time::Duration::from_secs(10 * 60);

/// 差し向け記録の鮮度の窓 (upstream `REVIEWER_DISPATCH_TTL_MS = 6 * 60 * 60 * 1000`)。
const DISPATCH_TTL: std::time::Duration = std::time::Duration::from_secs(6 * 60 * 60);

/// 差し向け記録のファイル名 (`stage-protocol-reviewer.md` §12a step 1)。
const DISPATCH_RECORD: &str = ".aidlc-reviewer-dispatch.json";

/// review-freeze — 終端受領証を無効化する書込みを拒否する。
///
/// `Bash` も他の書込み工具と同じ経路で検査する (upstream「Bash is inspected before execution
/// too」)。シェル経由の書込みは PostToolUse の監査へ現れないので、通してしまうと受領証が
/// 別のバイトの上に立ったままになる。宛先の抽出は封筒
/// (`harness_claude::WriteToolEnvelope`) の側で行う。
pub(super) async fn freeze(layout: &Layout, input: &str) -> Completion {
    if disabled("AIDLC_DISABLE_REVIEW_FREEZE_HOOK") {
        return Completion::silent();
    }
    // 稼働記録は判断より先で、失敗しても判断を変えない (upstream の heartbeat と同じ位置)。
    let _ = super::observe_hook_health(layout, FREEZE).await;
    let envelope = harness_claude::WriteToolEnvelope::parse(input, layout.project_dir());
    // 監査台帳が無ければ守る受領証も無い (台帳の置き場は record から導くので一度に確かめる)。
    let Some(record) = layout
        .record_dir()
        .filter(|_| layout.audit_dir().is_some_and(|dir| has_shard(&dir)))
    else {
        return Completion::silent();
    };
    if envelope.targets().is_empty() {
        return Completion::silent();
    }
    let execution = match crate::execution_cursor::ExecutionCursor::read(record) {
        Ok(Some(cursor)) => cursor.execution_id().clone(),
        Ok(None) => return Completion::silent(),
        Err(error) => return drop_and_allow(layout, FREEZE, &error.to_string()).await,
    };
    let Some(target) = health_target(layout, record) else {
        return Completion::silent();
    };
    let state = layout
        .state_file()
        .and_then(|path| std::fs::read(path).ok())
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();
    let status = super::session_hooks::field(&state, "Status").unwrap_or_default();
    let store = match super::store_path(layout) {
        Ok(store) => store,
        Err(error) => return drop_and_allow(layout, FREEZE, &error).await,
    };
    let (Ok(executions), Ok(intents), Ok(definitions), Ok(audits)) = (
        IntentExecutionRepositoryImpl::open(&store),
        IntentRepositoryImpl::open(&store),
        WorkflowDefinitionRepositoryImpl::open(&store),
        SessionAuditRepositoryImpl::open(&store),
    ) else {
        return drop_and_allow(layout, FREEZE, "cannot open the event store").await;
    };
    let verdict = GuardReviewFreezeUseCase::new(executions, intents, definitions, audits)
        .execute(
            &execution,
            &target,
            &status,
            envelope.tool(),
            envelope.targets(),
            Utc::now(),
        )
        .await;
    let block = match verdict {
        Ok(core_command_domain::orchestration::ReviewFreezeVerdict::Allowed) => {
            return Completion::silent();
        }
        Ok(core_command_domain::orchestration::ReviewFreezeVerdict::Blocked(block)) => block,
        Err(error) => return drop_and_allow(layout, FREEZE, &error.to_string()).await,
    };
    // 拒否は保存済みである。台帳へ描けなくても拒否は変えない (upstream も同じ)。
    if let Err(error) = super::catch_up(layout).await {
        let _ = super::record_hook_drop(layout, FREEZE, &error).await;
    }
    let stage = block.stage().as_str();
    let entry = Checkboxes::parse(&state);
    let guidance = entry.find(stage).map_or_else(
        || {
            wording::review_freeze_recovery(
                stage,
                core_command_domain::workspace::CheckboxState::Pending,
                false,
            )
        },
        |entry| {
            wording::review_freeze_recovery(stage, entry.state(), entry.rest().starts_with("SKIP"))
        },
    );
    Completion::hook_denied(wording::review_freeze_blocked(
        block.target().as_str(),
        stage,
        block
            .unit()
            .map(core_command_domain::orchestration::UnitName::as_str),
        &guidance,
    ))
}

/// reviewer-scope — per-unit レビュアーが兄弟 Unit へ届く呼出しを拒否する。
///
/// upstream と同じく**開いて通す** — 記録が無い、古い、読めない、形が違う、呼び手が
/// 差し向けの当人でない、のいずれでも通す。止めるのは、差し向け記録から範囲を組めて、
/// その範囲の外へ届くと**判定できた**ときだけである。
///
/// # 本 build に無い分岐
///
/// team unit ownership の claimed-checkout (`unitScope`) は移していない。
pub(super) async fn reviewer_scope(layout: &Layout, input: &str) -> Completion {
    if disabled("AIDLC_DISABLE_REVIEWER_SCOPE_HOOK") {
        return Completion::silent();
    }
    // 稼働記録は判断より先で、失敗しても判断を変えない (upstream の heartbeat と同じ位置)。
    let _ = super::observe_hook_health(layout, REVIEWER_SCOPE).await;
    let envelope = harness_claude::ReviewerScopeEnvelope::parse(input);
    let Some(tool) = envelope.tool() else {
        return Completion::silent();
    };
    let Some(record) = layout.record_dir() else {
        return Completion::silent();
    };
    let dispatch_path = record.join(DISPATCH_RECORD);
    if !dispatch_path.is_file() {
        return advise_missing_record(layout, record, &envelope).await;
    }
    // 差し向けと判定の間でセッションが落ちた記録が、後の無関係な作業を拒み続けないよう
    // 掃除する (upstream の orphaned record janitor と同じ位置・同じ窓)。
    if stale(&dispatch_path) {
        let _ = std::fs::remove_file(&dispatch_path);
        return drop_and_allow(
            layout,
            REVIEWER_SCOPE,
            "ignoring an orphaned reviewer dispatch record (older than the freshness window); cleaned it up",
        )
        .await;
    }
    let raw = match std::fs::read(&dispatch_path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(error) => return drop_and_allow(layout, REVIEWER_SCOPE, &error.to_string()).await,
    };
    let Some(dispatch) = harness_claude::parse_reviewer_dispatch(&raw) else {
        return drop_and_allow(
            layout,
            REVIEWER_SCOPE,
            "reviewer dispatch record is malformed; enforcement skipped",
        )
        .await;
    };
    // 強制する相手は差し向けられたレビュアーだけである。指揮者自身の呼出しや別の
    // サブエージェントは素通しする。
    if !dispatch.is_dispatched(envelope.agent_type(), envelope.scoped_registration()) {
        return Completion::silent();
    }
    let Some(target) = health_target(layout, record) else {
        return Completion::silent();
    };
    let status = super::session_hooks::field(&read_state(layout), "Status").unwrap_or_default();
    let store = match super::store_path(layout) {
        Ok(store) => store,
        Err(error) => return drop_and_allow(layout, REVIEWER_SCOPE, &error).await,
    };
    let Ok(audits) = SessionAuditRepositoryImpl::open(&store) else {
        return drop_and_allow(layout, REVIEWER_SCOPE, "cannot open the event store").await;
    };
    // 封筒に `cwd` が無ければ project dir を基点にする (upstream
    // `cwd: … ? cwdField : projectDir`)。基点を欠くと、綴りに construction が出ない
    // 相対の探索根 (`aidlc` など) が construction/ の上を掃くことを見落とす。
    let cwd = envelope
        .cwd()
        .map(str::to_string)
        .or_else(|| layout.project_dir().to_str().map(str::to_string));
    let request = ReviewerScopeRequest::new(
        dispatch,
        record.to_str().map(str::to_string),
        cwd,
        tool,
        envelope.candidates().clone(),
    );
    let verdict = GuardReviewerScopeUseCase::new(audits)
        .execute(&request, &target, &status, Utc::now())
        .await;
    let block = match verdict {
        Ok(ReviewerScopeVerdict::Allowed) => return Completion::silent(),
        Ok(ReviewerScopeVerdict::Blocked(block)) => block,
        // 記録に失敗しても拒否は変えない (upstream「an audit failure never changes the
        // block decision」)。原因は drop へ残す。
        Err(error) => {
            let _ = super::record_hook_drop(layout, REVIEWER_SCOPE, &error.to_string()).await;
            error.block().clone()
        }
    };
    if let Err(error) = super::catch_up(layout).await {
        let _ = super::record_hook_drop(layout, REVIEWER_SCOPE, &error).await;
    }
    Completion::hook_denied(wording::reviewer_scope_blocked(
        block.target().as_str(),
        block.unit().as_str(),
    ))
}

/// 記録が無いときの助言 — 差し向け記録を書き忘れた指揮者へ気付きを残す。
///
/// 記録は Unit と許可経路の唯一の出所なので、無ければ強制する材料が無い。拒否はせず、
/// レビュー専用エージェントが `construction/` へ触れたときだけ 10 分に 1 度残す。
async fn advise_missing_record(
    layout: &Layout,
    record: &Path,
    envelope: &harness_claude::ReviewerScopeEnvelope,
) -> Completion {
    if envelope.review_agent() && envelope.touches_construction() {
        advise_once(
            layout,
            record,
            REVIEWER_SCOPE,
            "missing-record",
            &format!(
                "{} touched construction/ paths with no reviewer dispatch record; enforcement skipped (write the stage-protocol-reviewer.md §12a step-1 dispatch record before invoking a per-unit reviewer)",
                envelope.agent_type()
            ),
        )
        .await;
    }
    Completion::silent()
}

/// 状態ファイルの現物 (読めなければ空)。
fn read_state(layout: &Layout) -> String {
    layout
        .state_file()
        .and_then(|path| std::fs::read(path).ok())
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default()
}

/// 差し向け記録が鮮度の窓を過ぎているか (読めない更新時刻は古いとみなさない)。
fn stale(path: &Path) -> bool {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .is_ok_and(|modified| {
            modified
                .elapsed()
                .is_ok_and(|elapsed| elapsed > DISPATCH_TTL)
        })
}

/// 決定的な停止スイッチ (upstream と同じ `=1` 一致)。
fn disabled(variable: &str) -> bool {
    std::env::var(variable).ok().as_deref() == Some("1")
}

/// 監査シャードが 1 枚でもあるか。
fn has_shard(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|entries| {
        entries.filter_map(Result::ok).any(|entry| {
            entry.path().extension().and_then(|ext| ext.to_str()) == Some("md")
                && entry.path().is_file()
        })
    })
}

/// 稼働記録・drop の宛先 (記録名が読めなければ `None`)。
fn health_target(layout: &Layout, record: &Path) -> Option<HookHealthTarget> {
    let space = SpaceName::parse(layout.space()).ok()?;
    let name = record
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| IntentDirName::parse(name).ok());
    Some(HookHealthTarget::new(space, name))
}

/// drop を 1 件残して通す (判断は常に許可)。
async fn drop_and_allow(layout: &Layout, hook: &str, reason: &str) -> Completion {
    let _ = super::record_hook_drop(layout, hook, reason).await;
    Completion::silent()
}

/// 同じ助言を 10 分に 1 度だけ drop へ残す (upstream の marker と同じ抑制)。
async fn advise_once(layout: &Layout, record: &Path, hook: &str, marker: &str, reason: &str) {
    let path = advisory_marker(record, hook, marker);
    if fresh(&path) {
        return;
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::write(&path, format!("{}\n", Utc::now().to_rfc3339())).is_err() {
        return;
    }
    let _ = super::record_hook_drop(layout, hook, reason).await;
}

/// 助言の目印 (稼働記録と同じディレクトリ)。
fn advisory_marker(record: &Path, hook: &str, marker: &str) -> PathBuf {
    record
        .join(".aidlc-hooks-health")
        .join(format!("{hook}.{marker}.last"))
}

/// 目印が抑制窓の内側か。
fn fresh(path: &Path) -> bool {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .is_ok_and(|modified| {
            modified
                .elapsed()
                .is_ok_and(|elapsed| elapsed < ADVISORY_WINDOW)
        })
}
