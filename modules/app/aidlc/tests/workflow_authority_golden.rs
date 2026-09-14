//! 群 A (`lookup` / `scope-table` / `stage-table`) の CLI ゴールデン — upstream 2.7.1 との
//! **stdout バイト・終了コード**の一致。
//!
//! ゴールデンは `tests/golden/workflow-authority/` の下に、固定ピン `a277af21` の配布ツールを
//! 固定フィクスチャ (`fixture/.claude`) に対して実行して採ったもの
//! (`scripts/goldens/capture-workflow-authority.ts`)。この Rust テストは同じフィクスチャを
//! `--project-dir` の `.claude` として読み、native の出力が upstream と 1 バイトも違わないことを
//! 確かめる。失敗経路 (未知 slug) は stdout が空・exit 1 で一致させる — stderr の
//! エンベロープ形式 (`{"error":..}`) の横断整合は別 Bolt の宿題であり、ここでは byte 比較しない。
#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn fixture_claude() -> PathBuf {
    repo_root().join("tests/golden/workflow-authority/fixture/.claude")
}

fn golden_dir() -> PathBuf {
    repo_root().join("tests/golden/workflow-authority/cli")
}

/// フィクスチャ `.claude` を写した使い捨てワークスペースを建てる。
fn workspace() -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("一時ディレクトリ");
    copy_tree(&fixture_claude(), &temp.path().join(".claude"));
    temp
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("コピー先");
    for entry in fs::read_dir(from).expect("読取") {
        let entry = entry.expect("エントリ");
        let dst = to.join(entry.file_name());
        if entry.file_type().expect("種別").is_dir() {
            copy_tree(&entry.path(), &dst);
        } else {
            fs::copy(entry.path(), &dst).expect("コピー");
        }
    }
}

struct Meta {
    tool: String,
    argv: Vec<String>,
    exit_code: u8,
}

fn read_meta(case: &str) -> Meta {
    let raw = fs::read_to_string(golden_dir().join(case).join("meta.json"))
        .unwrap_or_else(|error| panic!("meta が読めない ({case}): {error}"));
    let value: serde_json::Value = serde_json::from_str(&raw).expect("meta は JSON");
    Meta {
        tool: value["tool"].as_str().expect("tool").to_string(),
        argv: value["argv"]
            .as_array()
            .expect("argv")
            .iter()
            .map(|v| v.as_str().expect("argv 要素").to_string())
            .collect(),
        exit_code: u8::try_from(value["exit_code"].as_u64().expect("exit_code")).expect("u8"),
    }
}

fn golden_stdout(case: &str) -> Vec<u8> {
    fs::read(golden_dir().join(case).join("stdout"))
        .unwrap_or_else(|error| panic!("stdout ゴールデンが読めない ({case}): {error}"))
}

fn argv0_of(tool: &str) -> &'static str {
    match tool {
        "aidlc-state.ts" => "aidlc-state",
        "aidlc-utility.ts" => "aidlc-utility",
        other => panic!("未対応の tool: {other}"),
    }
}

/// 1 ケースを native で走らせ、stdout バイトと終了コードをゴールデンと突き合わせる。
async fn assert_case(case: &str) {
    let workspace = workspace();
    let project_dir = workspace.path();
    let meta = read_meta(case);
    let mut argv: Vec<String> = meta.argv.clone();
    argv.push("--project-dir".to_string());
    argv.push(project_dir.to_string_lossy().into_owned());

    let completion = aidlc::runtime::run(argv0_of(&meta.tool), &argv, project_dir).await;

    // main.rs は `writeln!` で末尾改行を付すので、実効 stdout は line + "\n"。
    let stdout: Vec<u8> = completion
        .line()
        .map(|line| format!("{line}\n").into_bytes())
        .unwrap_or_default();

    assert_eq!(
        stdout,
        golden_stdout(case),
        "{case}: stdout バイトが upstream と違う (native={:?})",
        completion.line()
    );
    assert_eq!(
        completion.code(),
        meta.exit_code,
        "{case}: 終了コードが upstream と違う"
    );
}

macro_rules! golden_case {
    ($name:ident, $case:literal) => {
        #[tokio::test]
        async fn $name() {
            assert_case($case).await;
        }
    };
}

golden_case!(phase_of_by_slug_matches_upstream, "lookup/phase-of-slug");
golden_case!(
    phase_of_by_number_matches_upstream,
    "lookup/phase-of-number"
);
golden_case!(
    phase_of_unknown_is_empty_stdout_exit_one,
    "lookup/phase-of-unknown"
);
golden_case!(agent_for_matches_upstream, "lookup/agent-for-agent");
golden_case!(
    agent_for_orchestrator_matches_upstream,
    "lookup/agent-for-orchestrator"
);
golden_case!(
    agent_for_unknown_is_empty_stdout_exit_one,
    "lookup/agent-for-unknown"
);
golden_case!(
    validate_stage_valid_matches_upstream,
    "lookup/validate-stage-valid"
);
golden_case!(
    validate_stage_by_number_matches_upstream,
    "lookup/validate-stage-number"
);
golden_case!(
    validate_stage_invalid_matches_upstream,
    "lookup/validate-stage-invalid"
);
golden_case!(
    next_stage_execute_matches_upstream,
    "lookup/next-stage-execute"
);
golden_case!(next_stage_beta_matches_upstream, "lookup/next-stage-beta");
golden_case!(
    next_stage_skip_walk_matches_upstream,
    "lookup/next-stage-skip-walk"
);
golden_case!(next_stage_none_matches_upstream, "lookup/next-stage-none");
golden_case!(
    next_stage_unknown_scope_matches_upstream,
    "lookup/next-stage-unknown-scope"
);
golden_case!(scope_table_matches_upstream, "scope-table/scope-table");
golden_case!(stage_table_matches_upstream, "stage-table/stage-table");
