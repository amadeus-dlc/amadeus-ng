//! `aidlc engine statusline` の配線 — 端末の状態表示へ 1 行を描く。
//!
//! **読取専用**である。カーソルも記録も監査も台帳も書かず、常に終了 0 で終わる — 状態行の
//! 失敗が会話を止めてはならない（upstream `hooks/aidlc-statusline.ts` も同じ契約で、読めない
//! 材料はすべて「無い」に倒す）。
//!
//! # なぜクエリ側のユースケースを通さないのか
//!
//! 材料の中心は `aidlc-state.md` — RMU が書く**人が読むファイル面**のリードモデル（系統 (1)）
//! である。`coding-rules/cqrs-boundaries.md` 規則 6 はこの面について「クエリ側ユースケースは
//! 不要」とし、さらに「クエリ側が (1) を逆パースして自分で計算することは禁止」と定める。
//! したがって状態ファイルの読取と段階の数え上げは、両側を知ってよい合成ルートが持つ
//! （同じ理由で `fold_usage` も `Current Stage` をここで読む）。登録簿の join だけは既に
//! クエリ側に owner が居るので、そちらを呼ぶ。
//!
//! # 端末幅を測らない
//!
//! upstream は `process.stdout.columns` が取れるときだけ右寄せの空白を詰め、取れなければ
//! `<左> | <右>` を出す。ホストは状態行をパイプで受け取る（端末ではない）ので upstream も
//! 実際には後者を通る。こちらは幅の出所を持たないので、常に後者を描く。

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::Path;

use core_query_interface_adapter::{IntentListingDaoImpl, display_slug_from_dir_name};
use core_query_use_case::orchestration::ListIntentsUseCase;

use crate::session_navigation::SessionNavigation;
use crate::usage_ledger::{self, UsageLedger};
use crate::wording;

use super::{Completion, Layout};

/// `aidlc engine statusline` — 標準入力のホスト情報を読み、状態行を 1 行返す。
pub(super) fn run(layout: &Layout) -> Completion {
    Completion::emitted(render(layout.project_dir(), &read_host_input()))
}

/// ホストが渡す 1 回分の JSON。端末に繋がっているときは読まない（読むと入力待ちで固まる）。
fn read_host_input() -> String {
    use std::io::{IsTerminal as _, Read as _};
    let mut stdin = std::io::stdin();
    if stdin.is_terminal() {
        return String::new();
    }
    let mut bytes = Vec::new();
    if stdin.read_to_end(&mut bytes).is_err() {
        return String::new();
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// 状態行 1 本（末尾改行は `main.rs` の `writeln!` が付す）。
fn render(project: &Path, input: &str) -> String {
    let payload = Payload::parse(input);
    let selection = Layout::resolve_for_session(project, payload.session.as_deref());
    let right = right_side(project, &selection, &payload);
    let Some(state) = read_state(&selection) else {
        return line(wording::STATUSLINE_READY, &right);
    };
    let phase = field(&state, "Lifecycle Phase");
    if phase.is_empty() {
        return line(wording::STATUSLINE_READY, &right);
    }
    let prefix = orientation_prefix(&selection);
    let progress = phase_progress(&state, &phase);
    let status = field(&state, "Status");
    let left = if status == "Completed" || status == "Complete" {
        completion(&prefix, progress)
    } else {
        position(&selection, &state, &prefix, &phase, progress)
    };
    line(&left, &right)
}

/// 進行が終わった記録の左側。
///
/// 段階の見出しが解決できなくても満たしたバーを見せる — 終わったことは進捗の欄からでも
/// 読めなければならない。
fn completion(prefix: &str, progress: (usize, usize)) -> String {
    let (done, total) = progress;
    let bar = match progress_bar(done, total) {
        bar if bar.is_empty() => progress_bar(1, 1),
        bar => bar,
    };
    format!(
        "{} {prefix}{} {bar}",
        wording::STATUSLINE_TAG,
        wording::STATUSLINE_COMPLETE
    )
}

/// 進行中の記録の左側 — 所在・段階・進捗・現在の作業・担当。
///
/// 読めなかった欄はその区画ごと落とす（upstream も空文字を偽として扱い、区画を出さない）。
fn position(
    selection: &Layout,
    state: &str,
    prefix: &str,
    phase: &str,
    progress: (usize, usize),
) -> String {
    let (done, total) = progress;
    let mut left = format!("{} {prefix}{phase}", wording::STATUSLINE_TAG);
    // バーと分数は同じ事実（数えられた項目があるか）で決まる。
    if total > 0 {
        left.push_str(&format!(" {} {done}/{total}", progress_bar(done, total)));
    }
    if let Some(stage) = stage_display(&field(state, "Current Stage")) {
        left.push_str(&format!(" > {stage}"));
    }
    if let Some(agent) = agent_display(selection, &field(state, "Active Agent")) {
        left.push_str(&format!(" -- {agent}"));
    }
    left
}

/// 左右を 1 行に畳む。右が空なら左だけを出す。
fn line(left: &str, right: &str) -> String {
    if right.is_empty() {
        left.to_string()
    } else {
        format!("{left} | {right}")
    }
}

/// 選択された記録の状態ファイル本文。記録が無い・読めないときは `None`。
fn read_state(selection: &Layout) -> Option<String> {
    let path = selection.state_file()?;
    let bytes = std::fs::read(path).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// 状態ファイルの箇条書き欄。欄が無ければ空文字（upstream `extractField`）。
fn field(state: &str, name: &str) -> String {
    super::session_hooks::field(state, name).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// ホストが渡す入力
// ---------------------------------------------------------------------------

/// 状態行がホストから受け取る 4 つの値。壊れた JSON は「何も渡らなかった」と同じに倒す。
#[derive(Debug, Default, PartialEq)]
struct Payload {
    session: Option<String>,
    model: String,
    context: Option<f64>,
    transcript: Option<String>,
}

impl Payload {
    /// 1 回分の JSON から読む。
    fn parse(input: &str) -> Payload {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(input) else {
            return Payload::default();
        };
        let model = value
            .pointer("/model/id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        Payload {
            session: value
                .get("session_id")
                .and_then(serde_json::Value::as_str)
                .filter(|session| Layout::valid_session_id(session))
                .map(str::to_string),
            // 文脈使用率はモデルが名乗れたときだけ読む（upstream 同順）。非有限は
            // 「渡らなかった」に倒す — 外から来る値をそのまま数として描かない。
            context: (!model.is_empty())
                .then(|| {
                    value
                        .pointer("/context_window/used_percentage")
                        .and_then(serde_json::Value::as_f64)
                        .filter(|percentage| percentage.is_finite())
                })
                .flatten(),
            model,
            transcript: value
                .get("transcript_path")
                .and_then(serde_json::Value::as_str)
                .filter(|path| !path.is_empty())
                .map(str::to_string),
        }
    }
}

// ---------------------------------------------------------------------------
// 右側 — モデル・文脈使用率・利用量
// ---------------------------------------------------------------------------

/// ANSI の色指定を解く綴り。
const RESET: &str = "\u{1b}[0m";

/// 右側の 3 区画。どれも出せなければ空文字である。
fn right_side(project: &Path, selection: &Layout, payload: &Payload) -> String {
    let mut parts = Vec::new();
    let model = abbreviate_model(&payload.model);
    if !model.is_empty() {
        parts.push(model);
    }
    if let Some(percentage) = payload.context {
        let percentage = js_round(percentage);
        parts.push(format!(
            "{}ctx:{percentage}%{RESET}",
            context_color(percentage)
        ));
    }
    let cost = cost_segment(project, selection, payload);
    if !cost.is_empty() {
        parts.push(cost);
    }
    parts.join(" ")
}

/// 文脈使用率の色（upstream `contextColor`）。
fn context_color(percentage: f64) -> &'static str {
    if percentage >= 75.0 {
        "\u{1b}[31m"
    } else if percentage >= 50.0 {
        "\u{1b}[33m"
    } else {
        "\u{1b}[32m"
    }
}

/// モデル識別子を状態行の幅に収まる形へ縮める（upstream `abbreviateModel`）。
fn abbreviate_model(model_id: &str) -> String {
    if model_id.is_empty() {
        return String::new();
    }
    let mut prefix = "";
    let mut short = model_id;
    for region in ["us", "eu", "apac", "global"] {
        if let Some(rest) = short.strip_prefix(&format!("{region}.anthropic.")) {
            prefix = "BR:";
            short = rest;
            break;
        }
    }
    let short = short.strip_prefix("claude-").unwrap_or(short);
    let short = drop_version_run(short);
    let short = drop_date_run(&short);
    let short = drop_revision_suffix(&short);
    format!("{prefix}{short}")
}

/// 最初の `-v<10 進数字 1 桁以上>` を落とす（upstream `/-v\d+/` の置換は先頭 1 件だけ）。
fn drop_version_run(text: &str) -> String {
    let mut from = 0;
    while let Some(offset) = text.get(from..).and_then(|tail| tail.find("-v")) {
        let start = from + offset;
        let digits_at = start + 2;
        let digits = digit_run(text, digits_at);
        if digits > 0 {
            return without(text, start, digits_at + digits);
        }
        from = digits_at;
    }
    text.to_string()
}

/// 最初の `-<10 進数字 8 桁>` を落とす（upstream `/-\d{8}/`。9 桁目以降は残る）。
fn drop_date_run(text: &str) -> String {
    let mut from = 0;
    while let Some(offset) = text.get(from..).and_then(|tail| tail.find('-')) {
        let start = from + offset;
        let digits_at = start + 1;
        if digit_run(text, digits_at) >= 8 {
            return without(text, start, digits_at + 8);
        }
        from = digits_at;
    }
    text.to_string()
}

/// 末尾の `:<10 進数字>` を落とす（upstream `/:\d+$/`）。
fn drop_revision_suffix(text: &str) -> String {
    let Some(at) = text.rfind(':') else {
        return text.to_string();
    };
    let digits = text.get(at + 1..).unwrap_or_default();
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return text.to_string();
    }
    text.get(..at).unwrap_or_default().to_string()
}

/// `at` から続く 10 進数字の桁数。
fn digit_run(text: &str, at: usize) -> usize {
    text.get(at..).map_or(0, |tail| {
        tail.bytes().take_while(u8::is_ascii_digit).count()
    })
}

/// `start..end` を抜いた文字列。
fn without(text: &str, start: usize, end: usize) -> String {
    let head = text.get(..start).unwrap_or_default();
    let tail = text.get(end..).unwrap_or_default();
    format!("{head}{tail}")
}

/// 現在の作業 × session の利用量（upstream `costSegment`）。
///
/// 台帳は畳み込みフックが書いたものをそのまま読む — 会話履歴をここで読み直さない。
/// 台帳が無い install（Claude 以外のハーネスや、一度も畳み込んでいない会話）では空文字で
/// あり、状態行はこの機能が入る前と 1 バイトも変わらない。
fn cost_segment(project: &Path, selection: &Layout, payload: &Payload) -> String {
    if usage_ledger::tracking_disabled() {
        return String::new();
    }
    let ledger_path = project.join("aidlc/.aidlc-sessions/usage-ledger.json");
    if !ledger_path.is_file() {
        return String::new();
    }
    let session = payload.session.as_deref().unwrap_or_default();
    let navigation = SessionNavigation::new(project, session);
    let transcript = payload
        .transcript
        .clone()
        .or_else(|| navigation.as_ref().and_then(SessionNavigation::transcript));
    let session_key =
        super::fold_usage::session_usage_key(transcript.as_deref().unwrap_or_default(), session);
    let workflow_key = super::fold_usage::workflow_usage_key(
        selection,
        navigation.as_ref().and_then(SessionNavigation::stamp),
    );
    let Some(totals) = UsageLedger::load(&ledger_path)
        .workflow(&workflow_key)
        .and_then(|workflow| workflow.session(&session_key))
        .map(|session| session.totals())
    else {
        return String::new();
    };
    let input = totals.tokens().input();
    let output = totals.tokens().output();
    let usd = totals.usd();
    // 値付けできたのは、有限で正の費用だけである。値付けできない行しか無いときに `$0` を
    // 描くと、無料で回ったという読み違いを招く。
    let priced = usd.is_finite() && usd > 0.0;
    if input <= 0.0 && output <= 0.0 && !priced {
        return String::new();
    }
    let mut segment = format!(
        "\u{2191}{} \u{2193}{}",
        fmt_tokens(input),
        fmt_tokens(output)
    );
    if priced {
        segment.push_str(&format!(" ${usd:.2}"));
    }
    segment
}

/// 量を `1.2k` / `3.4M` の形へ縮める（upstream `fmtTokens`）。
fn fmt_tokens(value: f64) -> String {
    if !value.is_finite() || value <= 0.0 {
        return "0".to_string();
    }
    if value >= 1e6 {
        return format!("{}M", one_decimal(value / 1e6));
    }
    if value >= 1e3 {
        return format!("{}k", one_decimal(value / 1e3));
    }
    format!("{}", js_round(value))
}

/// 小数 1 桁。`.0` は落とす（`2k` であって `2.0k` ではない）。
fn one_decimal(value: f64) -> String {
    let text = format!("{value:.1}");
    text.strip_suffix(".0").unwrap_or(&text).to_string()
}

/// JavaScript の `Math.round`（半数は正の無限大側へ寄せる）。
fn js_round(value: f64) -> f64 {
    (value + 0.5).floor()
}

// ---------------------------------------------------------------------------
// 左側 — 所在・進行・段階・担当
// ---------------------------------------------------------------------------

/// `<space> · <intent-slug> · ` の所在（upstream `orientationPrefix`）。
///
/// 空間が 1 つだけの利用者には空間名を見せない。記録が無いときは接頭辞そのものが無く、
/// 状態行はこの機能が入る前と同じ綴りになる。
fn orientation_prefix(selection: &Layout) -> String {
    let Some(active) = selection
        .record_dir()
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
    else {
        return String::new();
    };
    let slug = ListIntentsUseCase::new(IntentListingDaoImpl::new(&selection.intents_dir()))
        .execute(Some(active))
        .ok()
        .and_then(|view| {
            view.intents()
                .iter()
                .find(|row| row.directory() == Some(active))
                .map(|row| row.slug().to_string())
        })
        .filter(|slug| !slug.is_empty())
        .unwrap_or_else(|| display_slug_from_dir_name(active));
    let mut segments = Vec::new();
    if space_count(selection.project_dir()) > 1 {
        segments.push(selection.space().to_string());
    }
    segments.push(slug);
    format!("{} \u{b7} ", segments.join(" \u{b7} "))
}

/// 空間の数（upstream `listSpaces` — 既定の空間は必ず数える）。
fn space_count(project: &Path) -> usize {
    let mut names = std::collections::BTreeSet::new();
    names.insert(crate::layout::DEFAULT_SPACE.to_string());
    if let Ok(entries) = std::fs::read_dir(project.join("aidlc/spaces")) {
        for entry in entries.flatten() {
            if entry.path().is_dir()
                && let Some(name) = entry.file_name().to_str()
            {
                names.insert(name.to_string());
            }
        }
    }
    names.len()
}

/// 現在の段階で済んだ項目と全項目（upstream `phaseProgress`）。
///
/// 見出し `### <PHASE> PHASE` から次の見出しまでの箇条書きを数える。`SKIP` と `[S]` の行は
/// 実行しない項目なので分母にも入れない。
fn phase_progress(state: &str, phase: &str) -> (usize, usize) {
    let token = phase
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_uppercase();
    if token.is_empty() {
        return (0, 0);
    }
    let heading = format!("{token} PHASE");
    let mut in_phase = false;
    let mut done = 0;
    let mut total = 0;
    for raw in state.lines() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if line.starts_with("### ") {
            in_phase = line.to_uppercase().contains(&heading);
            continue;
        }
        if !in_phase || !line.starts_with("- [") {
            continue;
        }
        if line.contains("SKIP") || line.contains("[S]") {
            continue;
        }
        total += 1;
        if line.starts_with("- [x]") {
            done += 1;
        }
    }
    (done, total)
}

/// 10 目盛りの進捗バー（upstream `progressBar`）。全項目が 0 なら描かない。
fn progress_bar(done: usize, total: usize) -> String {
    if total == 0 {
        return String::new();
    }
    let filled = (done * 10 / total).min(10);
    format!(
        "[{}{}]",
        "\u{2593}".repeat(filled),
        "\u{2591}".repeat(10 - filled)
    )
}

/// 段階 slug の表示名（upstream `STAGE_DISPLAY`。表に無い slug はそのまま名乗る）。
fn stage_display(slug: &str) -> Option<String> {
    if slug.is_empty() {
        return None;
    }
    Some(
        STAGE_DISPLAY
            .iter()
            .find(|(stage, _)| *stage == slug)
            .map_or_else(|| slug.to_string(), |(_, display)| (*display).to_string()),
    )
}

/// 段階 slug と状態行での表示名（upstream `hooks/aidlc-statusline.ts` の逐語表）。
///
/// コンパイル済みグラフの `name` とは別の値である（グラフは `State Initialization`、状態行は
/// `State Init`）。状態行は幅が限られるので、upstream は専用の短い表を持つ。
const STAGE_DISPLAY: [(&str, &str); 33] = [
    ("workspace-scaffold", "Workspace Scaffold"),
    ("workspace-detection", "Workspace Detection"),
    ("state-init", "State Init"),
    ("intent-capture", "Intent Capture"),
    ("market-research", "Market Research"),
    ("feasibility", "Feasibility"),
    ("scope-definition", "Scope Definition"),
    ("team-formation", "Team Formation"),
    ("rough-mockups", "Rough Mockups"),
    ("approval-handoff", "Approval & Handoff"),
    ("reverse-engineering", "Reverse Engineering"),
    ("practices-discovery", "Practices Discovery"),
    ("requirements-analysis", "Requirements Analysis"),
    ("user-stories", "User Stories"),
    ("refined-mockups", "Refined Mockups"),
    ("domain-design", "Domain Design"),
    ("contract-design", "Contract Design"),
    ("units-generation", "Units Generation"),
    ("delivery-planning", "Delivery Planning"),
    ("functional-design", "Functional Design"),
    ("nfr-requirements", "NFR Requirements"),
    ("nfr-design", "NFR Design"),
    ("infrastructure-design", "Infrastructure Design"),
    ("code-generation", "Code Generation"),
    ("build-and-test", "Build and Test"),
    ("ci-pipeline", "CI Pipeline"),
    ("deployment-pipeline", "Deployment Pipeline"),
    ("environment-provisioning", "Env Provisioning"),
    ("deployment-execution", "Deployment Execution"),
    ("observability-setup", "Observability Setup"),
    ("incident-response", "Incident Response"),
    ("performance-validation", "Performance Validation"),
    ("feedback-optimization", "Feedback & Optimization"),
];

/// 担当エージェントの表示名（upstream `agentDisplayMap`。表に無い名前はそのまま名乗る）。
fn agent_display(selection: &Layout, name: &str) -> Option<String> {
    if name.is_empty() {
        return None;
    }
    let mut map = BTreeMap::new();
    // 状態ファイルは遷移中に `orchestrator` を名乗るが、対応するペルソナ定義は無い。
    map.insert("orchestrator".to_string(), "Orchestrator".to_string());
    map.extend(persona_display_names(&selection.agent_dir()));
    Some(map.get(name).map_or_else(|| name.to_string(), Clone::clone))
}

/// ペルソナ定義の frontmatter が名乗る `name` → `display_name`。
///
/// 同じ `name` を 2 つの定義が名乗ったら表ごと捨てる（upstream 同様） — どちらを採るかを
/// 推測すると、状態行が実行のたびに別の担当を名乗りうる。
fn persona_display_names(directory: &Path) -> BTreeMap<String, String> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return BTreeMap::new();
    };
    let mut map = BTreeMap::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension() != Some(OsStr::new("md")) {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            return BTreeMap::new();
        };
        let body = String::from_utf8_lossy(&bytes);
        let (Some(name), Some(display)) = (
            frontmatter_scalar(&body, "name"),
            frontmatter_scalar(&body, "display_name"),
        ) else {
            continue;
        };
        if map.insert(name, display).is_some() {
            return BTreeMap::new();
        }
    }
    map
}

/// frontmatter の単一行の値。
fn frontmatter_scalar(body: &str, key: &str) -> Option<String> {
    let rest = body
        .strip_prefix("---\n")
        .or_else(|| body.strip_prefix("---\r\n"))?;
    let end = rest.find("\n---")?;
    let prefix = format!("{key}:");
    rest.get(..end)?.lines().find_map(|line| {
        line.strip_prefix(&prefix)
            .map(|value| core_infrastructure::ecmascript::trim(value).to_string())
            .filter(|value| !value.is_empty())
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    fn workspace() -> tempfile::TempDir {
        tempfile::tempdir().expect("一時ディレクトリ")
    }

    /// 記録が無いワークスペースは、所在も進行も名乗らない。
    #[test]
    fn a_workspace_without_a_record_is_ready() {
        let root = workspace();
        assert_eq!(render(root.path(), ""), wording::STATUSLINE_READY);
    }

    /// 壊れた標準入力は「何も渡らなかった」と同じに倒れる（状態行は失敗しない）。
    #[test]
    fn a_malformed_host_payload_is_read_as_nothing() {
        assert_eq!(Payload::parse("{"), Payload::default());
        assert_eq!(Payload::parse(""), Payload::default());
    }

    /// 文脈使用率はモデルが名乗れたときだけ読み、非有限は渡らなかった扱いにする。
    #[test]
    fn the_context_share_is_read_only_beside_a_named_model() {
        let with_model = Payload::parse(
            r#"{"model":{"id":"claude-opus-4-8"},"context_window":{"used_percentage":51.4}}"#,
        );
        assert_eq!(with_model.context, Some(51.4));
        let without_model = Payload::parse(r#"{"context_window":{"used_percentage":51.4}}"#);
        assert_eq!(without_model.context, None);
        let not_a_number =
            Payload::parse(r#"{"model":{"id":"x"},"context_window":{"used_percentage":"51"}}"#);
        assert_eq!(not_a_number.context, None);
    }

    /// session 名は検査を通ったものだけを運ぶ。
    #[test]
    fn only_a_safe_session_name_is_carried() {
        assert_eq!(
            Payload::parse(r#"{"session_id":"abc-123"}"#)
                .session
                .as_deref(),
            Some("abc-123")
        );
        assert_eq!(
            Payload::parse(r#"{"session_id":"../escape"}"#).session,
            None
        );
    }

    /// 推論プロファイルの接頭辞・版・日付・改訂番号を落とす。
    #[test]
    fn a_model_identifier_is_shortened_the_way_upstream_shortens_it() {
        assert_eq!(
            abbreviate_model("global.anthropic.claude-opus-4-8-v1:0"),
            "BR:opus-4-8"
        );
        assert_eq!(
            abbreviate_model("us.anthropic.claude-sonnet-4-5-20250929-v1:0"),
            "BR:sonnet-4-5"
        );
        assert_eq!(abbreviate_model("claude-haiku-4-5"), "haiku-4-5");
        assert_eq!(abbreviate_model(""), "");
        // 9 桁の数字並びは、区切りと 8 桁ぶんだけが落ちて 9 桁目が残る
        // （upstream の `/-\d{8}/` と同じ）。
        assert_eq!(abbreviate_model("claude-x-123456789"), "x9");
        // 数字を伴わない `-v` は版ではない。
        assert_eq!(abbreviate_model("claude-v-next"), "v-next");
    }

    /// 文脈使用率の色は 50% と 75% で変わる。
    #[test]
    fn the_context_colour_changes_at_the_two_thresholds() {
        assert_eq!(context_color(49.0), "\u{1b}[32m");
        assert_eq!(context_color(50.0), "\u{1b}[33m");
        assert_eq!(context_color(74.0), "\u{1b}[33m");
        assert_eq!(context_color(75.0), "\u{1b}[31m");
    }

    /// 量の表記は 1000 と 1000000 で桁を繰り上げ、`.0` を落とす。
    #[test]
    fn a_token_count_is_written_in_the_upstream_compact_form() {
        assert_eq!(fmt_tokens(0.0), "0");
        assert_eq!(fmt_tokens(-1.0), "0");
        assert_eq!(fmt_tokens(f64::NAN), "0");
        assert_eq!(fmt_tokens(999.0), "999");
        assert_eq!(fmt_tokens(1234.0), "1.2k");
        assert_eq!(fmt_tokens(2000.0), "2k");
        assert_eq!(fmt_tokens(3_400_000.0), "3.4M");
    }

    /// 進捗は現在の段階の見出しの中だけを数え、飛ばす項目は分母にも入れない。
    #[test]
    fn the_progress_counts_only_the_current_phase() {
        let state = "\
### INCEPTION PHASE
- [x] one
- [ ] two
- [x] three (SKIP)
- [S] four
### CONSTRUCTION PHASE
- [x] five
";
        assert_eq!(phase_progress(state, "INCEPTION"), (1, 2));
        assert_eq!(phase_progress(state, "CONSTRUCTION (finalizing)"), (1, 1));
        assert_eq!(phase_progress(state, ""), (0, 0));
        assert_eq!(phase_progress(state, "OPERATION"), (0, 0));
    }

    /// バーは 10 目盛りで、全項目が 0 なら描かない。
    #[test]
    fn the_progress_bar_fills_ten_cells() {
        assert_eq!(progress_bar(0, 0), "");
        assert_eq!(progress_bar(0, 4), "[░░░░░░░░░░]");
        assert_eq!(progress_bar(2, 4), "[▓▓▓▓▓░░░░░]");
        assert_eq!(progress_bar(4, 4), "[▓▓▓▓▓▓▓▓▓▓]");
        // 進んだ数が全項目を超えても 10 目盛りに収まる。
        assert_eq!(progress_bar(9, 4), "[▓▓▓▓▓▓▓▓▓▓]");
    }

    /// 表に無い段階 slug はそのまま名乗る。
    #[test]
    fn an_unknown_stage_slug_is_shown_as_it_is() {
        assert_eq!(
            stage_display("code-generation").as_deref(),
            Some("Code Generation")
        );
        assert_eq!(stage_display("not-a-stage").as_deref(), Some("not-a-stage"));
        assert_eq!(stage_display(""), None);
    }

    /// 同じ `name` を 2 つのペルソナが名乗ったら表ごと捨てる。
    #[test]
    fn a_duplicate_persona_name_discards_the_whole_display_map() {
        let root = workspace();
        let agents = root.path().join("agents");
        std::fs::create_dir_all(&agents).expect("ペルソナ置き場");
        std::fs::write(
            agents.join("a.md"),
            "---\nname: aidlc-developer-agent\ndisplay_name: Developer\n---\nbody\n",
        )
        .expect("1 つ目");
        assert_eq!(
            persona_display_names(&agents).get("aidlc-developer-agent"),
            Some(&"Developer".to_string())
        );
        std::fs::write(
            agents.join("b.md"),
            "---\nname: aidlc-developer-agent\ndisplay_name: Coder\n---\nbody\n",
        )
        .expect("2 つ目");
        assert_eq!(persona_display_names(&agents), BTreeMap::new());
    }

    /// frontmatter を持たない定義からは何も読まない。
    #[test]
    fn a_definition_without_frontmatter_names_nothing() {
        assert_eq!(frontmatter_scalar("name: x\n", "name"), None);
        assert_eq!(frontmatter_scalar("---\nname:\n---\n", "name"), None);
        assert_eq!(
            frontmatter_scalar("---\nname: x\n---\nname: y\n", "name"),
            Some("x".to_string())
        );
    }

    /// 記録が 1 つの空間では、所在は依頼名だけを名乗る（空間名は出さない）。
    #[test]
    fn a_single_space_shows_only_the_intent_name() {
        let root = workspace();
        let intents = root.path().join("aidlc/spaces/default/intents");
        let record = intents.join("260907-selfhost-a1b2c3d4");
        std::fs::create_dir_all(&record).expect("記録");
        std::fs::write(
            record.join("aidlc-state.md"),
            "- **Lifecycle Phase**: IDEATION\n",
        )
        .expect("状態ファイル");
        let selection = Layout::resolve(root.path());
        assert_eq!(orientation_prefix(&selection), "selfhost-a1b2c3d4 \u{b7} ");
    }

    /// 空間が 2 つ以上あるときだけ空間名が前に立つ。
    #[test]
    fn more_than_one_space_puts_the_space_name_in_front() {
        let root = workspace();
        let intents = root.path().join("aidlc/spaces/default/intents");
        let record = intents.join("260907-selfhost-a1b2c3d4");
        std::fs::create_dir_all(&record).expect("記録");
        std::fs::write(
            record.join("aidlc-state.md"),
            "- **Lifecycle Phase**: IDEATION\n",
        )
        .expect("状態ファイル");
        std::fs::create_dir_all(root.path().join("aidlc/spaces/other")).expect("2 つ目の空間");
        let selection = Layout::resolve(root.path());
        assert_eq!(
            orientation_prefix(&selection),
            "default \u{b7} selfhost-a1b2c3d4 \u{b7} "
        );
    }

    /// 登録簿が名乗る slug は、ディレクトリ名からの導出より優先する。
    #[test]
    fn the_registry_slug_wins_over_the_derived_one() {
        let root = workspace();
        let intents = root.path().join("aidlc/spaces/default/intents");
        let record = intents.join("260907-selfhost-a1b2c3d4");
        std::fs::create_dir_all(&record).expect("記録");
        std::fs::write(
            record.join("aidlc-state.md"),
            "- **Lifecycle Phase**: IDEATION\n",
        )
        .expect("状態ファイル");
        std::fs::write(
            intents.join("intents.json"),
            r#"[{"uuid":"u","slug":"renamed","status":"active","dirName":"260907-selfhost-a1b2c3d4"}]"#,
        )
        .expect("登録簿");
        let selection = Layout::resolve(root.path());
        assert_eq!(orientation_prefix(&selection), "renamed \u{b7} ");
    }

    /// 進行中の記録は、所在・段階・進捗・担当を 1 行に畳む。
    #[test]
    fn an_active_record_shows_where_it_is() {
        let root = workspace();
        let intents = root.path().join("aidlc/spaces/default/intents");
        let record = intents.join("260907-selfhost-a1b2c3d4");
        std::fs::create_dir_all(&record).expect("記録");
        std::fs::write(
            record.join("aidlc-state.md"),
            "\
- **Lifecycle Phase**: CONSTRUCTION
- **Current Stage**: code-generation
- **Active Agent**: orchestrator
- **Status**: In Progress

### CONSTRUCTION PHASE
- [x] one
- [ ] two
",
        )
        .expect("状態ファイル");
        assert_eq!(
            render(root.path(), ""),
            "[AIDLC] selfhost-a1b2c3d4 \u{b7} CONSTRUCTION [▓▓▓▓▓░░░░░] 1/2 > Code Generation -- Orchestrator"
        );
    }

    /// 進行が終わった記録は満たしたバーだけを見せる。
    #[test]
    fn a_completed_record_shows_a_full_bar() {
        let root = workspace();
        let intents = root.path().join("aidlc/spaces/default/intents");
        let record = intents.join("260907-selfhost-a1b2c3d4");
        std::fs::create_dir_all(&record).expect("記録");
        std::fs::write(
            record.join("aidlc-state.md"),
            "- **Lifecycle Phase**: OPERATION\n- **Status**: Completed\n",
        )
        .expect("状態ファイル");
        assert_eq!(
            render(root.path(), ""),
            "[AIDLC] selfhost-a1b2c3d4 \u{b7} COMPLETE [▓▓▓▓▓▓▓▓▓▓]"
        );
    }

    /// 段階を名乗れない状態ファイルは、記録があっても待機として描く。
    #[test]
    fn a_state_file_without_a_phase_is_ready() {
        let root = workspace();
        let intents = root.path().join("aidlc/spaces/default/intents");
        let record = intents.join("260907-selfhost-a1b2c3d4");
        std::fs::create_dir_all(&record).expect("記録");
        std::fs::write(record.join("aidlc-state.md"), "# no fields\n").expect("状態ファイル");
        assert_eq!(render(root.path(), ""), wording::STATUSLINE_READY);
    }

    /// ホストが名乗ったモデルと文脈使用率は右側に付く。
    #[test]
    fn the_host_model_and_context_share_ride_on_the_right() {
        let root = workspace();
        assert_eq!(
            render(
                root.path(),
                r#"{"model":{"id":"claude-opus-4-8"},"context_window":{"used_percentage":80.4}}"#
            ),
            "[AIDLC] ready | opus-4-8 \u{1b}[31mctx:80%\u{1b}[0m"
        );
    }
}
