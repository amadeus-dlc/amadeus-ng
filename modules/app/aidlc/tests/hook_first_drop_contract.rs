//! 最初の通知が失敗でも、未観測のheartbeatを作らずdropだけを記録する。
#![allow(clippy::unwrap_used)]
use base64::Engine as _;
use std::{
    fs,
    io::Write as _,
    process::{Command, Stdio},
};

#[test]
fn an_unknown_session_stamp_records_the_first_drop_without_a_heartbeat() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/session-end-unknown.json"
    ))
    .unwrap();
    assert_eq!(
        corpus
            .pointer("/source/commit")
            .and_then(serde_json::Value::as_str),
        Some("a277af218f0df7f325d3b8be7b6d90fce2c5bd40")
    );
    let case = corpus
        .get("observations")
        .unwrap()
        .as_array()
        .unwrap()
        .first()
        .unwrap();
    let root = tempfile::tempdir().unwrap();
    let initial = case.get("initial_files").unwrap().as_object().unwrap();
    for (relative, encoded) in initial {
        let path = root.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            path,
            base64::engine::general_purpose::STANDARD
                .decode(encoded.as_str().unwrap())
                .unwrap(),
        )
        .unwrap();
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .args(["hook", "session-end"])
        .current_dir(root.path())
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", root.path())
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            case.pointer("/input/stdin")
                .unwrap()
                .as_str()
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
    let (relative, encoded) = case
        .get("changed_files")
        .unwrap()
        .as_object()
        .unwrap()
        .iter()
        .next()
        .unwrap();
    let expected = String::from_utf8(
        base64::engine::general_purpose::STANDARD
            .decode(encoded.as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    let drop = root.path().join(relative);
    assert!(drop.is_file(), "初回のdropも公開する");
    let actual = fs::read_to_string(&drop).unwrap();
    assert_eq!(
        actual.split_once('\t').unwrap().1,
        expected.split_once('\t').unwrap().1,
        "時刻以外の本文は本家と同じ"
    );
    assert!(!drop.with_extension("last").exists(), "heartbeatは未観測");
    for (relative, encoded) in initial {
        if relative.ends_with("aidlc-state.md") || relative.contains("/audit/") {
            assert_eq!(
                fs::read(root.path().join(relative)).unwrap(),
                base64::engine::general_purpose::STANDARD
                    .decode(encoded.as_str().unwrap())
                    .unwrap(),
                "{relative}"
            );
        }
    }
}

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;
