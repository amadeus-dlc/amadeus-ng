//! 本家2.7.1の `hooks/aidlc-fold-usage.ts` を、通常のPre/PostToolUse入口として観測する。
//!
//! 正本は固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の実走行採取
//! （`tests/golden/selfhost-stage1/fold-usage.json`、採取器は
//! `scripts/goldens/capture-fold-usage.ts`）である。U1の採取は利用量を無効化していたので、
//! ここでは停止フラグを立てない**有効経路**を検証する。
#![allow(clippy::unwrap_used, clippy::panic)]
use base64::Engine as _;
use std::{
    collections::BTreeMap,
    fs,
    io::Write as _,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// 採取全体。固定コミットのピンをテストの前提として確かめる。
fn golden() -> serde_json::Value {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/fold-usage.json"
    ))
    .unwrap();
    assert_eq!(
        corpus
            .get("source")
            .unwrap()
            .get("commit")
            .unwrap()
            .as_str(),
        Some("a277af218f0df7f325d3b8be7b6d90fce2c5bd40")
    );
    corpus
}

/// 指定した筋書きの観測を、採取した順のまま返す。
fn steps(scenario: &str) -> Vec<serde_json::Value> {
    let prefix = format!("{scenario}/");
    golden()
        .get("observations")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .filter(|value| {
            value
                .get("id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| id.starts_with(&prefix))
        })
        .cloned()
        .collect()
}

/// 採取時のワークスペース根（この文字列を実行時の根へ差し替える）。
fn captured_root(case: &serde_json::Value) -> String {
    case.get("input")
        .unwrap()
        .get("environment")
        .unwrap()
        .get("AIDLC_PROJECT_DIR")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string()
}

/// base64の実バイトを、採取根だけ差し替えて返す。非UTF-8はそのまま通す。
fn localize(encoded: &str, captured: &str, root: &Path) -> Vec<u8> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap();
    match String::from_utf8(bytes) {
        Ok(text) => text.replace(captured, &root.to_string_lossy()).into_bytes(),
        Err(error) => error.into_bytes(),
    }
}

/// 観測の直前の状態を一時ワークスペースへ組み直す。
fn restore(root: &Path, case: &serde_json::Value) {
    let captured = captured_root(case);
    for (relative, encoded) in case.get("initial_files").unwrap().as_object().unwrap() {
        let Some(encoded) = encoded.as_str() else {
            continue;
        };
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, localize(encoded, &captured, root)).unwrap();
    }
}

/// 観測の直後にあるべき内容（`initial_files` を `changed_files` で上書きした形）。
fn expected_files(root: &Path, case: &serde_json::Value) -> BTreeMap<String, Option<Vec<u8>>> {
    let captured = captured_root(case);
    let mut expected = BTreeMap::new();
    for collection in ["initial_files", "changed_files"] {
        for (relative, encoded) in case.get(collection).unwrap().as_object().unwrap() {
            expected.insert(
                relative.clone(),
                encoded
                    .as_str()
                    .map(|encoded| localize(encoded, &captured, root)),
            );
        }
    }
    expected
}

/// フックを起動する。停止フラグは採取時の値をそのまま渡す。
fn invoke(root: &Path, input: &str, disable_tracking: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .args(["hook", "fold-usage"])
        .current_dir(root)
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", root)
        .env("PATH", "/usr/bin:/bin")
        .env("AIDLC_DISABLE_USAGE_TRACKING", disable_tracking)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

/// `aidlc/.aidlc-sessions` の実在ファイル一覧。
fn sessions_entries(root: &Path) -> Vec<String> {
    let directory = root.join("aidlc/.aidlc-sessions");
    let Ok(entries) = fs::read_dir(&directory) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// 1 件の観測を再現し、出力と `aidlc/` 配下の全ファイルを突き合わせる。
fn replay(scenario: &str, case: &serde_json::Value) {
    let id = case.get("id").unwrap().as_str().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    restore(root, case);
    let captured = captured_root(case);
    let input = case
        .get("input")
        .unwrap()
        .get("stdin")
        .unwrap()
        .as_str()
        .unwrap()
        .replace(&captured, &root.to_string_lossy());
    let disable = case
        .get("input")
        .unwrap()
        .get("environment")
        .unwrap()
        .get("AIDLC_DISABLE_USAGE_TRACKING")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

    let actual = invoke(root, &input, disable);

    assert_eq!(
        actual.status.code(),
        Some(0),
        "{scenario}/{id}: stderr={}",
        String::from_utf8_lossy(&actual.stderr)
    );
    assert!(
        actual.stdout.is_empty(),
        "{id}: 成功時のstdoutは空である: {:?}",
        String::from_utf8_lossy(&actual.stdout)
    );
    assert!(
        actual.stderr.is_empty(),
        "{id}: 観測だけのフックはstderrも出さない: {:?}",
        String::from_utf8_lossy(&actual.stderr)
    );

    for (relative, expected) in expected_files(root, case) {
        if !relative.starts_with("aidlc/") {
            continue;
        }
        let path = root.join(&relative);
        match expected {
            None => assert!(!path.exists(), "{id}: 消えているはず: {relative}"),
            Some(expected) => {
                let actual = fs::read(&path).unwrap_or_else(|error| {
                    panic!("{id}: {relative} を読めない: {error}");
                });
                assert_eq!(
                    String::from_utf8_lossy(&actual),
                    String::from_utf8_lossy(&expected),
                    "{id}: {relative}"
                );
            }
        }
    }

    let mut expected_names: Vec<String> = expected_files(root, case)
        .into_iter()
        .filter(|(relative, content)| {
            relative.starts_with("aidlc/.aidlc-sessions/") && content.is_some()
        })
        .filter_map(|(relative, _)| {
            Path::new(&relative)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .collect();
    expected_names.sort();
    assert_eq!(
        sessions_entries(root),
        expected_names,
        "{id}: sessionsディレクトリに余分な生成物を作らない"
    );
}

#[test]
fn a_workspace_without_a_record_folds_exactly_as_upstream_does() {
    for case in steps("bare") {
        replay("bare", &case);
    }
}

#[test]
fn a_workspace_with_a_bugfix_record_folds_exactly_as_upstream_does() {
    for case in steps("intent") {
        replay("intent", &case);
    }
}

#[test]
fn truncation_rotation_and_a_missing_transcript_follow_upstream_cursor_handling() {
    for case in steps("rotate") {
        replay("rotate", &case);
    }
}

#[test]
fn the_kill_switch_stops_the_producer_before_it_touches_the_workspace() {
    let cases = steps("disabled");
    assert!(!cases.is_empty());
    for case in cases {
        replay("disabled", &case);
        assert!(
            case.get("changed_files")
                .unwrap()
                .as_object()
                .unwrap()
                .is_empty(),
            "停止時は1バイトも書かない"
        );
    }
}

#[test]
fn the_captured_steps_chain_into_the_same_final_ledger() {
    // 1 本のワークスペースで採取順に撃ち、最後の台帳が本家の最後の台帳と一致することを見る。
    // 各段を独立に再現する上の3件と違い、こちらは**自分が書いた台帳を自分で読み直す**経路である。
    for scenario in ["bare", "intent", "rotate"] {
        let cases = steps(scenario);
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        restore(root, cases.first().unwrap());
        let mut last_ledger = None;
        for case in &cases {
            let captured = captured_root(case);
            // 会話履歴は採取時と同じ内容を毎回置き直す（採取器が撃つ前に書いた形）。
            for (relative, encoded) in case.get("initial_files").unwrap().as_object().unwrap() {
                if !relative.ends_with(".jsonl") && !relative.ends_with(".meta.json") {
                    continue;
                }
                let Some(encoded) = encoded.as_str() else {
                    continue;
                };
                let path = root.join(relative);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, localize(encoded, &captured, root)).unwrap();
            }
            let input = case
                .get("input")
                .unwrap()
                .get("stdin")
                .unwrap()
                .as_str()
                .unwrap()
                .replace(&captured, &root.to_string_lossy());
            let actual = invoke(root, &input, "");
            assert_eq!(actual.status.code(), Some(0));
            last_ledger = case
                .get("changed_files")
                .unwrap()
                .get("aidlc/.aidlc-sessions/usage-ledger.json")
                .and_then(serde_json::Value::as_str)
                .map(|encoded| localize(encoded, &captured, root))
                .or(last_ledger);
        }
        let expected = String::from_utf8(last_ledger.unwrap()).unwrap();
        let produced =
            fs::read_to_string(root.join("aidlc/.aidlc-sessions/usage-ledger.json")).unwrap();
        assert_eq!(produced, expected, "{scenario}: 連続畳み込みの最終台帳");
    }
}

/// 会話履歴を一時ワークスペースへ書き、そのパスを返す。
fn write_transcript(root: &Path, name: &str, lines: &[String]) -> PathBuf {
    let path = root.join(name);
    fs::write(&path, format!("{}\n", lines.join("\n"))).unwrap();
    path
}

/// 1回のassistant応答を表すJSONL行。
fn assistant_line(uuid: &str, message_id: &str, model: &str, input: u64, output: u64) -> String {
    format!(
        r#"{{"uuid":"{uuid}","timestamp":"2026-09-10T00:00:00Z","type":"assistant","message":{{"id":"{message_id}","role":"assistant","model":"{model}","usage":{{"input_tokens":{input},"output_tokens":{output},"cache_read_input_tokens":0,"cache_creation_input_tokens":0}}}}}}"#
    )
}

const SESSION: &str = "11111111-2222-4333-8444-555555555555";

#[test]
fn the_fold_usage_hook_records_the_session_and_transcript_pointers() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    fs::create_dir_all(root.join("aidlc")).unwrap();
    let transcript = write_transcript(
        root,
        "transcript.jsonl",
        &[
            assistant_line("u1", "msg_1", "claude-opus-4-8", 100, 20),
            assistant_line("u2", "msg_2", "claude-opus-4-8", 200, 40),
        ],
    );
    let input = format!(
        r#"{{"session_id":"{SESSION}","hook_event_name":"PostToolUse","tool_name":"Read","tool_input":{{}},"transcript_path":"{}"}}"#,
        transcript.to_string_lossy()
    );

    let output = invoke(root, &input, "");

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty(), "成功時のstdoutは空である");
    let sessions = root.join("aidlc/.aidlc-sessions");
    assert_eq!(
        fs::read_to_string(sessions.join(format!("{SESSION}.transcript"))).unwrap(),
        transcript.to_string_lossy(),
    );
    assert_eq!(
        fs::read_to_string(sessions.join("current.transcript")).unwrap(),
        transcript.to_string_lossy(),
    );
    assert_eq!(
        fs::read_to_string(sessions.join(".current-session")).unwrap(),
        format!("{SESSION}\n"),
    );
}

// ---------------------------------------------------------------------------------------
// Stop フックの flush-all (本家 `hooks/aidlc-continue-workflow.ts:1328-1338`)。
//
// 正本は固定コミットの Stop フックを実走行させた採取 `tests/golden/selfhost-stage1/stop-fold-usage.json`
// (採取器は `scripts/goldens/capture-stop-fold-usage.ts`) である。Stop はエンジンにも問うが、
// ここで固定するのは turn-end の利用量 producer だけなので、比較は `aidlc/.aidlc-sessions/`
// 配下 (台帳と会話履歴ポインタ) と終了コードに限る。エンジン照会の面は Stop の既存契約
// (`upstream_271_contract.rs`) が固定している。
// ---------------------------------------------------------------------------------------

/// Stop の採取全体。固定コミットのピンをテストの前提として確かめる。
fn stop_golden() -> serde_json::Value {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/stop-fold-usage.json"
    ))
    .unwrap();
    assert_eq!(
        corpus
            .get("source")
            .unwrap()
            .get("commit")
            .unwrap()
            .as_str(),
        Some("a277af218f0df7f325d3b8be7b6d90fce2c5bd40")
    );
    corpus
}

/// Stop の観測を採取した順のまま返す。
fn stop_steps() -> Vec<serde_json::Value> {
    stop_golden()
        .get("observations")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .filter(|value| {
            value
                .get("id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| id.starts_with("stop/"))
        })
        .cloned()
        .collect()
}

/// Stop フックを起動する。停止フラグは採取時の値をそのまま渡す。
fn invoke_stop(root: &Path, input: &str, disable_tracking: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .args(["hook", "continue-workflow"])
        .current_dir(root)
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", root)
        .env("PATH", "/usr/bin:/bin")
        .env("AIDLC_DISABLE_USAGE_TRACKING", disable_tracking)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

/// Stop の観測 1 件を再現し、`aidlc/.aidlc-sessions/` 配下を突き合わせる。
fn replay_stop(case: &serde_json::Value) {
    let id = case.get("id").unwrap().as_str().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    restore(root, case);
    let captured = captured_root(case);
    let input = case
        .get("input")
        .unwrap()
        .get("stdin")
        .unwrap()
        .as_str()
        .unwrap()
        .replace(&captured, &root.to_string_lossy());
    let disable = case
        .get("input")
        .unwrap()
        .get("environment")
        .unwrap()
        .get("AIDLC_DISABLE_USAGE_TRACKING")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

    let actual = invoke_stop(root, &input, disable);

    assert_eq!(
        actual.status.code(),
        Some(0),
        "{id}: stderr={}",
        String::from_utf8_lossy(&actual.stderr)
    );
    let expected = expected_files(root, case);
    for (relative, expected) in &expected {
        if !relative.starts_with("aidlc/.aidlc-sessions/") {
            continue;
        }
        let path = root.join(relative);
        match expected {
            None => assert!(!path.exists(), "{id}: 消えているはず: {relative}"),
            Some(expected) => {
                let actual = fs::read(&path).unwrap_or_else(|error| {
                    panic!("{id}: {relative} を読めない: {error}");
                });
                assert_eq!(
                    String::from_utf8_lossy(&actual),
                    String::from_utf8_lossy(expected),
                    "{id}: {relative}"
                );
            }
        }
    }
    let mut expected_names: Vec<String> = expected
        .into_iter()
        .filter(|(relative, content)| {
            relative.starts_with("aidlc/.aidlc-sessions/") && content.is_some()
        })
        .filter_map(|(relative, _)| {
            Path::new(&relative)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .collect();
    expected_names.sort();
    assert_eq!(
        sessions_entries(root),
        expected_names,
        "{id}: sessions ディレクトリに余分な生成物を作らない"
    );
}

#[test]
fn the_stop_hook_flushes_every_transcript_group_exactly_as_upstream_does() {
    // 記録がある workspace で Stop を撃つと、main と sub-agent の最後の群まで締めて台帳を書き、
    // 会話履歴ポインタを session 付きと `current.transcript` の両方に書く。
    for case in stop_steps() {
        let id = case.get("id").unwrap().as_str().unwrap();
        if id != "stop/intent" && id != "stop/missing-transcript" {
            continue;
        }
        replay_stop(&case);
        assert!(
            case.get("changed_files")
                .unwrap()
                .as_object()
                .unwrap()
                .contains_key("aidlc/.aidlc-sessions/usage-ledger.json"),
            "{id}: 採取は台帳を書いている"
        );
    }
}

#[test]
fn the_stop_hook_writes_nothing_when_tracking_is_off_or_there_is_nothing_to_fold() {
    // 停止フラグ、記録の無い workspace (Stop は状態ファイルが無ければ折り畳みに達しない)、
    // Codex の rollout 形のパス (Claude 形式でない) のいずれでも `.aidlc-sessions` に 1 バイトも
    // 書かない。
    for case in stop_steps() {
        let id = case.get("id").unwrap().as_str().unwrap();
        if !["stop/disabled", "stop/bare", "stop/codex-rollout"].contains(&id) {
            continue;
        }
        replay_stop(&case);
        assert!(
            !case
                .get("changed_files")
                .unwrap()
                .as_object()
                .unwrap()
                .keys()
                .any(|path| path.starts_with("aidlc/.aidlc-sessions/")),
            "{id}: 採取も sessions には書いていない"
        );
    }
}

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;
