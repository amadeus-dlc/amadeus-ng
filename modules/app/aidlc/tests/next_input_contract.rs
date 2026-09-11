//! 空workspaceのnext入力は、ワークフローを初期化せず本家の終端指示を返す。
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::process::Command;

fn field<'a>(value: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    value.get(name).expect("保存コーパスの必須項目")
}

#[test]
fn leading_workspace_commands_keep_the_upstream_argv_and_terminal_output() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/next-input.json"
    ))
    .unwrap();
    let observations = field(&corpus, "observations").as_array().unwrap();
    assert!(!observations.is_empty());
    let mut ids = std::collections::BTreeSet::new();
    for observation in observations {
        let root = tempfile::tempdir().unwrap();
        let args: Vec<&str> = field(observation, "args")
            .as_array()
            .unwrap()
            .iter()
            .map(|arg| arg.as_str().unwrap())
            .collect();
        let result = Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .arg("next")
            .args(args)
            .current_dir(root.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", root.path())
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap();
        let id = field(observation, "id").as_str().unwrap();
        assert!(ids.insert(id), "コーパスIDの重複: {id}");
        assert_eq!(
            result.status.code().map(i64::from),
            field(observation, "exit_code").as_i64(),
            "{id}"
        );
        assert_eq!(
            result.stderr,
            field(observation, "stderr").as_str().unwrap().as_bytes(),
            "{id}"
        );
        assert_eq!(
            result.stdout,
            field(observation, "stdout").as_str().unwrap().as_bytes(),
            "{id}"
        );
        assert!(
            !root.path().join("aidlc").exists(),
            "{id}: 終端指示で初期化しない"
        );
    }
}

/// 本家parseNextFlagsの先頭位置規則。文中や--後の語は名詞コマンドにならない。
#[test]
fn workspace_nouns_inside_freeform_remain_task_text() {
    use aidlc::cli::{Face, Request, parse};
    use core_query_use_case::orchestration::NextTurnInput;
    for words in [
        vec!["fix", "intent", "routing"],
        vec!["describe", "space", "selection"],
        vec!["--", "intent", "list"],
    ] {
        let argv: Vec<String> = std::iter::once("next")
            .chain(words.iter().copied())
            .map(str::to_string)
            .collect();
        let expected = words
            .iter()
            .filter(|word| **word != "--")
            .copied()
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(
            parse(Face::Orchestrate, &argv),
            Request::Next(Box::new(NextTurnInput::new().with_freeform(expected)))
        );
    }
}

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;
