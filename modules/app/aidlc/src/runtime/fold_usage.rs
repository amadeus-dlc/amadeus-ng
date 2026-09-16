//! 通常のPre/PostToolUseで会話履歴を利用量台帳へ畳み込む producer。
//!
//! 本家 `hooks/aidlc-fold-usage.ts` の入口である。監査・状態・承認権限は更新しない。
//! 失敗は握り潰し、常に終了0・stdout空で返す（観測だけを行う契約）。
//!
//! 発火点ごとの締め方は封筒が決める（[`FoldMode`]）: 通常の PreToolUse は main を締め、
//! 工程を進める PreToolUse は sub-agent まで締め、PostToolUse は保留する。turn-end の
//! Stop は [`flush_on_stop`] で全ファイルを締める（本家 `aidlc-continue-workflow.ts:1328-1338`）。
use super::{Completion, Layout};
use crate::intent_location::IntentLocation;
use crate::session_navigation::SessionNavigation;
use crate::usage_ledger::{
    self, FoldAttribution, LedgerLock, ModelRates, TranscriptSession, UsageLedger,
};
use harness_claude::{FoldMode, FoldUsageEnvelope};
use std::path::Path;

pub(super) fn run(layout: &Layout, input: &str) -> Completion {
    // 停止フラグ。畳み込み・ポインタ・session捕捉のすべてを止める。
    if usage_ledger::tracking_disabled() {
        return Completion::silent();
    }
    let envelope = FoldUsageEnvelope::parse(input);
    let Some(transcript) = envelope.transcript() else {
        return Completion::silent();
    };
    let session = envelope
        .session()
        .filter(|session| Layout::valid_session_id(session))
        .unwrap_or_default();
    // 本家と同じく封筒の session で配置を解き、その状態ファイルの `Current Stage` を読む。
    let selected = Layout::resolve_for_session(
        layout.project_dir(),
        (!session.is_empty()).then_some(session),
    );
    let stage = current_stage(&selected);
    let navigation = SessionNavigation::new(layout.project_dir(), session);
    if let Some(navigation) = &navigation {
        let _ = navigation.write_current();
    }
    SessionNavigation::write_transcript(layout.project_dir(), session, transcript);
    let attribution = attribution(&selected, transcript, session, stage);
    // 台帳の失敗はフックの結果にしない（本家は握り潰して既存の台帳を残す）。
    let _ = fold(&selected, transcript, envelope.fold_mode(), &attribution);
    Completion::silent()
}

/// Stop（turn-end）の締め — 本家 `aidlc-continue-workflow.ts:1328-1338`。
///
/// 会話履歴のパスを session 付きの `<session>.transcript` と `current.transcript` へ書き
/// （`writeCurrentTranscriptPath`）、全ファイルの完了群を締めて台帳を書く
/// （`foldTranscriptIntoLedger(..., "flush-all")`）。Claude 形式の会話履歴だけが対象で、
/// Codex の rollout 形のパス（`/[/\\]rollout-[^/\\]*\.jsonl$/`）は触らない。fold-usage
/// フックと違い `.current-session` は書かない（本家の Stop は `writeCurrentSessionId` を
/// 呼ばない）。停止フラグでは 1 バイトも書かない。失敗は握り潰す — 利用量が Stop の判断を
/// 変えてはならない。
///
/// `layout` は封筒の session で解いた配置（本家の `selection`）、`state` はその状態ファイルの
/// 本文である。呼出側は状態ファイルが在ることを確かめてから呼ぶ（本家も `existsSync(statePath)`
/// の後でしか折り畳みに達しない）。
pub(super) fn flush_on_stop(layout: &Layout, session: Option<&str>, input: &str, state: &str) {
    if usage_ledger::tracking_disabled() {
        return;
    }
    let Some(transcript) = stop_transcript(input) else {
        return;
    };
    let session = session
        .filter(|session| Layout::valid_session_id(session))
        .unwrap_or_default();
    SessionNavigation::write_transcript(layout.project_dir(), session, &transcript);
    let stage =
        super::session_hooks::field(state, "Current Stage").filter(|stage| !stage.is_empty());
    let attribution = attribution(layout, &transcript, session, stage);
    let _ = fold(layout, &transcript, FoldMode::FlushAll, &attribution);
}

/// Stop の封筒が運ぶ Claude 形式の会話履歴のパス。無い・空・Codex 形なら `None`。
fn stop_transcript(input: &str) -> Option<String> {
    let envelope: serde_json::Value = serde_json::from_str(input).ok()?;
    let transcript = envelope
        .get("transcript_path")
        .and_then(serde_json::Value::as_str)
        .filter(|path| !path.is_empty())?;
    (!is_codex_rollout(transcript)).then(|| transcript.to_string())
}

/// 本家の Codex 判定 `/[/\\]rollout-[^/\\]*\.jsonl$/` — 区切りの直後の最後の成分が
/// `rollout-` で始まり `.jsonl` で終わる。
fn is_codex_rollout(path: &str) -> bool {
    path.rfind(['/', '\\']).is_some_and(|separator| {
        let component = &path[separator + 1..];
        component.starts_with("rollout-") && component.ends_with(".jsonl")
    })
}

/// 1 回の畳み込みの帰属 — ステージ・session の鍵・作業の鍵。
fn attribution(
    layout: &Layout,
    transcript: &str,
    session: &str,
    stage: Option<String>,
) -> FoldAttribution {
    FoldAttribution::new(
        stage,
        session_usage_key(transcript, session),
        workflow_usage_key(
            layout,
            SessionNavigation::new(layout.project_dir(), session)
                .as_ref()
                .and_then(SessionNavigation::stamp),
        ),
    )
}

/// 台帳のロックを取り、main と sub-agent の新しいバイトを畳み込んで原子的に書く。
/// ロックは本家と同じく 5 秒まで待ち、取れなければ台帳に触らない。
fn fold(
    layout: &Layout,
    transcript: &str,
    mode: FoldMode,
    attribution: &FoldAttribution,
) -> std::io::Result<()> {
    let _guard = LedgerLock::acquire(layout.project_dir(), std::time::Duration::from_secs(5))?;
    let path = layout
        .project_dir()
        .join("aidlc/.aidlc-sessions/usage-ledger.json");
    let rates = ModelRates::load(&layout.definition_data_dir().join("model-rates.json"));
    let mut ledger = UsageLedger::load(&path);
    for source in TranscriptSession::new(transcript).sources(mode) {
        ledger.fold_source(&source, &rates, attribution);
    }
    ledger.write(&path)
}

/// 状態ファイルが名乗る `Current Stage`。record が無い・欄が無いときは `None`
/// （`byStage` を汚さない）。
fn current_stage(layout: &Layout) -> Option<String> {
    let bytes = std::fs::read(layout.state_file()?).ok()?;
    super::session_hooks::field(&String::from_utf8_lossy(&bytes), "Current Stage")
        .filter(|stage| !stage.is_empty())
}

/// `sessions` の鍵。会話履歴のパスが session の安定した識別子である（本家 `sessionUsageKey`）。
///
/// 台帳へ**書く**側と**読む**側は同じ鍵で引かなければならないので、鍵の作り方はここが唯一の
/// owner である（本家も `aidlc-usage.ts` の 1 実装を producer と consumer が共有する）。
pub(super) fn session_usage_key(transcript: &str, session: &str) -> String {
    let trimmed = core_infrastructure::ecmascript::trim(transcript);
    if !trimmed.is_empty() {
        return format!("transcript:{trimmed}");
    }
    if session.is_empty() {
        "session:unknown".to_string()
    } else {
        format!("session:{session}")
    }
}

/// `workflows` の鍵。session が刻んだ intent の UUID、無ければ配置が指す intent の UUID、
/// UUID を持たない legacy/孤児の record は space と record 名（本家 `intentUsageKey`）。
///
/// [`session_usage_key`] と同じ理由で、書く側と読む側が共有する唯一の owner である。
pub(super) fn workflow_usage_key(layout: &Layout, stamp: Option<String>) -> String {
    if let Some(stamp) = stamp {
        return format!("intent:{stamp}");
    }
    if let Some(location) = IntentLocation::current(layout) {
        return format!("intent:{}", location.uuid());
    }
    let record = layout.record_dir().and_then(Path::file_name).map_or_else(
        || "legacy".to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    format!("record:{}/{record}", layout.space())
}

#[cfg(test)]
mod tests {
    use super::{is_codex_rollout, session_usage_key, stop_transcript, workflow_usage_key};

    #[test]
    fn a_codex_rollout_path_is_recognised_only_by_its_last_component() {
        assert!(is_codex_rollout(
            "/s/sessions/2026/rollout-2026-09-10T00-00-00.jsonl"
        ));
        assert!(is_codex_rollout("C:\\s\\rollout-.jsonl"));
        assert!(
            !is_codex_rollout("rollout-x.jsonl"),
            "区切りが無い綴りは本家の正規表現に当たらない"
        );
        assert!(!is_codex_rollout("/s/rollout-x.jsonl/main.jsonl"));
        assert!(!is_codex_rollout("/s/main.jsonl"));
    }

    #[test]
    fn the_stop_envelope_yields_a_claude_transcript_or_nothing() {
        assert_eq!(
            stop_transcript(r#"{"transcript_path":"/t/main.jsonl"}"#).as_deref(),
            Some("/t/main.jsonl")
        );
        for input in [
            "not json",
            "null",
            "{}",
            r#"{"transcript_path":""}"#,
            r#"{"transcript_path":7}"#,
            r#"{"transcript_path":"/t/rollout-1.jsonl"}"#,
        ] {
            assert_eq!(stop_transcript(input), None, "{input}");
        }
    }

    /// session の鍵は会話履歴のパスが第一で、無ければ session 名、それも無ければ `unknown`。
    #[test]
    fn the_session_key_prefers_the_transcript_then_the_session_then_unknown() {
        assert_eq!(
            session_usage_key(" /t/main.jsonl ", "abc"),
            "transcript:/t/main.jsonl"
        );
        assert_eq!(session_usage_key("", "abc"), "session:abc");
        assert_eq!(session_usage_key("  ", ""), "session:unknown");
    }

    /// workflow の鍵は session の印、次に配置の intent、最後に record 名で決まる。
    #[test]
    fn the_workflow_key_falls_back_from_the_stamp_to_the_record_name() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let layout = crate::layout::Layout::resolve(root.path());
        assert_eq!(
            workflow_usage_key(&layout, Some("uuid-1".into())),
            "intent:uuid-1"
        );
        assert_eq!(workflow_usage_key(&layout, None), "record:default/legacy");
        let intents = root.path().join("aidlc/spaces/default/intents");
        std::fs::create_dir_all(intents.join("260101-orphan-aaaaaaaa")).expect("記録");
        std::fs::write(
            intents.join("260101-orphan-aaaaaaaa/aidlc-state.md"),
            "# state\n",
        )
        .expect("状態");
        let layout = crate::layout::Layout::resolve(root.path());
        assert_eq!(
            workflow_usage_key(&layout, None),
            "record:default/260101-orphan-aaaaaaaa",
            "registry の無い孤児の record は名前で鍵にする"
        );
    }
}
