//! `aidlc --doctor` (契約 C7) の公開面 — 入口・書式・終了コード・副作用 (DC1 / DC2 / DC10) と、
//! 固定本家 2.7.1 (`a277af21`) の doctor 採取 (`tests/golden/upstream-a277af21/doctor/cases.json`)
//! に対する採用行のバイト一致 (DC3〜DC9)。
//!
//! 作業ツリーは配布シェルの部分集合 (`tests/golden/selfhost-stage1/doctor-shell/`、封印済み
//! `source-manifest.sha256` と一致することを先に確かめる) と配布グラフ 3 入力から組み、
//! settings.json が名指すフックは空ファイルで置く (doctor は存在だけを見る)。異常は
//! 一時ワークスペースへ注入し、本リポジトリの実ファイルは変更しない。
// 契約テストは固定の添字参照と panic を検証の合図として使う (既存の契約テストと同じ許容)。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
#[path = "../../../../tests/support/tool_link.rs"]
mod tool_link;
use coverage_profile_env::coverage_profile_env;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn shell_fixture() -> PathBuf {
    repo_root().join("tests/golden/selfhost-stage1/doctor-shell")
}

fn copy_tree(from: &Path, to: &Path) {
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            fs::create_dir_all(&target).unwrap();
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// `settings.json` が名指す `aidlc-*.ts` (本家 doctor と同じ正規表現の写し)。
fn wired_hook_names(settings: &str) -> Vec<String> {
    let mut names = BTreeSet::new();
    let mut rest = settings;
    while let Some(start) = rest.find("aidlc-") {
        let tail = &rest[start..];
        let end = tail[6..]
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .map_or(tail.len(), |offset| 6 + offset);
        if tail[end..].starts_with(".ts") {
            names.insert(format!("{}.ts", &tail[..end]));
        }
        rest = &tail[1..];
    }
    names.into_iter().collect()
}

struct Workspace {
    temp: tempfile::TempDir,
}

impl Workspace {
    /// 配布シェルの部分集合 + 配布グラフ 3 入力 + フック空ファイル + default space の memory 層。
    /// Bun の代役を `bin/bun` に置き、`PATH` はそれと `/usr/bin:/bin` だけにする。
    fn shipped() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        fs::create_dir_all(&root).unwrap();
        copy_tree(&shell_fixture(), &root);
        fs::remove_file(root.join("provenance.json")).unwrap();
        let data = root.join(".claude/tools/data");
        fs::create_dir_all(&data).unwrap();
        for name in ["stage-graph.json", "scope-grid.json", "harness.json"] {
            fs::copy(
                repo_root()
                    .join("tests/golden/upstream-a277af21/data")
                    .join(name),
                data.join(name),
            )
            .unwrap();
        }
        fs::create_dir_all(root.join(".claude/hooks")).unwrap();
        let settings = fs::read_to_string(root.join(".claude/settings.json")).unwrap();
        for name in wired_hook_names(&settings) {
            fs::write(root.join(".claude/hooks").join(name), "").unwrap();
        }
        fs::create_dir_all(root.join("aidlc/spaces/default/memory")).unwrap();
        let bin = temp.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("bun"), "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(bin.join("bun"), fs::Permissions::from_mode(0o755)).unwrap();
        }
        fs::create_dir_all(temp.path().join("home")).unwrap();
        Self { temp }
    }

    fn root(&self) -> PathBuf {
        self.temp.path().join("workspace")
    }

    fn path_env(&self, with_bun: bool) -> String {
        if with_bun {
            format!("{}:/usr/bin:/bin", self.temp.path().join("bin").display())
        } else {
            "/usr/bin:/bin".to_string()
        }
    }

    fn cli_with(&self, argv0: &str, args: &[&str], with_bun: bool) -> Output {
        let binary = self.temp.path().join(argv0);
        if !binary.exists() {
            tool_link::link_tool(&binary).unwrap();
        }
        Command::new(binary)
            .args(args)
            .current_dir(self.root())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.temp.path().join("home"))
            .env("PATH", self.path_env(with_bun))
            .env(
                "AIDLC_MANAGED_SETTINGS_PATH",
                self.root().join("aidlc/.capture-managed.json"),
            )
            .env("LANG", "C.UTF-8")
            .env("LC_ALL", "C.UTF-8")
            .env("TZ", "UTC")
            .output()
            .unwrap()
    }

    fn cli(&self, argv0: &str, args: &[&str]) -> Output {
        self.cli_with(argv0, args, true)
    }

    fn doctor(&self) -> Output {
        self.cli("aidlc", &["--doctor"])
    }

    /// ワークスペース配下の全ファイル (相対パス、名前順)。
    fn files(&self) -> Vec<String> {
        fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
            for entry in fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(root, &path, out);
                } else {
                    out.push(
                        path.strip_prefix(root)
                            .unwrap()
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.root(), &self.root(), &mut out);
        out.sort();
        out
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

const RULE: &str = "─────────────────────────────────────";

/// 配布シェルの部分集合が封印済み manifest と一致することを、比較の前に確かめる。
#[test]
fn the_doctor_shell_fixture_matches_the_sealed_source_manifest() {
    let manifest = fs::read_to_string(
        repo_root().join("tests/golden/upstream-a277af21/source-manifest.sha256"),
    )
    .unwrap();
    let sealed: std::collections::BTreeMap<&str, &str> = manifest
        .lines()
        .filter_map(|line| line.split_once("  "))
        .map(|(hash, path)| (path, hash))
        .collect();
    let provenance: serde_json::Value =
        serde_json::from_slice(&fs::read(shell_fixture().join("provenance.json")).unwrap())
            .unwrap();
    let files = provenance["files"].as_array().unwrap();
    assert_eq!(files.len(), 50);
    for file in files {
        let path = file["path"].as_str().unwrap();
        let bytes = fs::read(shell_fixture().join(path)).unwrap();
        let digest = core_infrastructure::hash::sha256_hex(&bytes);
        assert_eq!(file["sha256"].as_str().unwrap(), digest, "{path}");
        assert_eq!(
            sealed.get(path).copied(),
            Some(digest.as_str()),
            "{path} は封印済み manifest に無い"
        );
    }
}

#[test]
fn doctor_is_an_orchestrate_entry_that_takes_no_extra_arguments() {
    let workspace = Workspace::shipped();
    let cold = workspace.doctor();
    assert_eq!(cold.status.code(), Some(0), "{cold:?}");
    assert!(
        stdout(&cold).starts_with("AI-DLC Health Check\n"),
        "{}",
        stdout(&cold)
    );
    assert_eq!(stderr(&cold), "");

    let via_orchestrate = workspace.cli("aidlc-orchestrate", &["--doctor"]);
    assert_eq!(
        via_orchestrate.status.code(),
        Some(0),
        "{via_orchestrate:?}"
    );
    assert_eq!(stdout(&via_orchestrate), stdout(&cold));

    let extra = workspace.cli("aidlc", &["--doctor", "--export"]);
    assert_eq!(extra.status.code(), Some(1), "{extra:?}");
    assert_eq!(stdout(&extra), "", "追加引数は診断を始めない");
    assert!(stderr(&extra).contains("--doctor"), "{}", stderr(&extra));
    assert!(stderr(&extra).contains("--export"), "{}", stderr(&extra));

    let next_doctor = workspace.cli("aidlc", &["next", "--doctor"]);
    assert_eq!(next_doctor.status.code(), Some(0), "{next_doctor:?}");
    let directive: serde_json::Value = serde_json::from_str(stdout(&next_doctor).trim()).unwrap();
    assert_eq!(
        directive["kind"], "print",
        "`next --doctor` は本家どおり TS 委譲の print のまま"
    );
    // 2.8.2 は読み取り専用のうち doctor / version だけ engine を挟まず `aidlc <sub>` を綴る
    // (`.claude/tools/aidlc-orchestrate.ts:4191` の `${aidlcInvocation()} ${sub}`)。
    assert!(
        stdout(&next_doctor).contains("Run `aidlc doctor`"),
        "{}",
        stdout(&next_doctor)
    );
    assert!(
        !stdout(&next_doctor).contains("bun .claude/tools/"),
        "{}",
        stdout(&next_doctor)
    );
}

#[test]
fn a_cold_shipped_workspace_renders_the_c7_report_and_creates_nothing() {
    let workspace = Workspace::shipped();
    let before = workspace.files();
    let output = workspace.doctor();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(stderr(&output), "");
    let settings = fs::read_to_string(workspace.root().join(".claude/settings.json")).unwrap();
    let mut expected = format!("AI-DLC Health Check\n{RULE}\n");
    expected.push_str("✓  bun installed (required for CLI tools and hooks)\n");
    expected.push_str("✓  Native engine entry points\n");
    for hook in wired_hook_names(&settings) {
        expected.push_str(&format!("✓  {hook} present\n"));
    }
    expected.push_str("✓  Hooks enabled (resolved disableAllHooks is not true)\n");
    expected.push_str("✓  settings.json present\n");
    expected.push_str("✓  Native hook bindings\n");
    expected.push_str("✓  Hook heartbeats: not yet fired (first workflow stage will populate)\n");
    expected.push_str("✓  workspace shell ready (.claude/ + aidlc/spaces/default/memory/)\n");
    expected.push_str("✓  Scope validation: 2 scopes valid (6 advisories)\n");
    expected.push_str("✓  Cycle detection: 0 cycles\n");
    expected.push_str("✓  Orphan stage files: 33 graph entries all have files\n");
    expected.push_str("✓  Schema validation: 33/33 stages validated\n");
    expected.push_str("✓  Graph references: 122 artifacts + edges resolved\n");
    expected.push_str(&format!("{RULE}\n29 passed, 0 failed\n"));
    assert_eq!(stdout(&output), expected);
    assert_eq!(workspace.files(), before, "DC1: 初回状態では何も作らない");
    assert!(
        !workspace
            .root()
            .join("aidlc/spaces/default/intents")
            .exists()
    );
}

#[test]
fn a_failed_required_check_exits_one_with_the_fix_on_its_row_and_keeps_stderr_empty() {
    let workspace = Workspace::shipped();
    let output = workspace.cli_with("aidlc", &["--doctor"], false);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(stderr(&output), "");
    let text = stdout(&output);
    assert!(
        text.contains("✗  bun installed (required for CLI tools and hooks) — install via `curl -fsSL https://bun.sh/install | bash`\n"),
        "{text}"
    );
    assert!(
        text.ends_with(&format!("{RULE}\n28 passed, 1 failed\n")),
        "{text}"
    );
    assert!(
        text.contains("✓  Native engine entry points\n"),
        "他の行も評価する: {text}"
    );
}

// ---------------------------------------------------------------------------
// Step 5 — 実行記録の副作用 (DC1 / DC2 / DC10)
// ---------------------------------------------------------------------------

impl Workspace {
    /// `intent-create` で記録を鋳造する (Rust の入口 — 状態・監査・ストアが揃う)。
    fn initialize(&self) -> PathBuf {
        let created = self.cli(
            "aidlc-utility",
            &[
                "intent-create",
                "--scope",
                "bugfix",
                "--label",
                "doctor",
                "--arguments",
                "Diagnose this fixture",
            ],
        );
        assert!(created.status.success(), "{created:?}");
        let intents = self.root().join("aidlc/spaces/default/intents");
        intents.join(
            fs::read_to_string(intents.join("active-intent"))
                .unwrap()
                .trim(),
        )
    }

    fn audit_shard(record: &Path) -> PathBuf {
        fs::read_dir(record.join("audit"))
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .find(|path| path.extension().is_some_and(|ext| ext == "md"))
            .unwrap()
    }

    /// 監査の最新 STAGE_/GATE_ 行の時刻を読む (heartbeat の遅延判定の基準)。
    fn newest_progress(record: &Path) -> chrono::DateTime<chrono::Utc> {
        let audit = fs::read_to_string(Self::audit_shard(record)).unwrap();
        audit
            .split("\n---\n")
            .filter(|block| {
                block.lines().any(|line| {
                    line.starts_with("**Event**: STAGE_") || line.starts_with("**Event**: GATE_")
                })
            })
            .filter_map(|block| {
                block
                    .lines()
                    .find_map(|line| line.strip_prefix("**Timestamp**: "))
            })
            .filter_map(|raw| raw.trim().parse::<chrono::DateTime<chrono::Utc>>().ok())
            .max()
            .unwrap()
    }

    /// heartbeat を最終進行と同時刻で置く (フックが発火した記録を装う)。
    fn fire_heartbeat(record: &Path, hook: &str, at: chrono::DateTime<chrono::Utc>) {
        let health = record.join(".aidlc-hooks-health");
        fs::create_dir_all(&health).unwrap();
        fs::write(
            health.join(format!("{hook}.last")),
            at.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        )
        .unwrap();
    }

    fn store(&self) -> PathBuf {
        self.root()
            .join("aidlc/spaces/default/intents/.aidlc-store.sqlite")
    }
}

#[test]
fn an_initialized_record_with_live_hooks_passes_and_records_one_health_check() {
    let workspace = Workspace::shipped();
    let record = workspace.initialize();
    let advanced = Workspace::newest_progress(&record);
    Workspace::fire_heartbeat(&record, "fixture", advanced);
    let before_audit = fs::read_to_string(Workspace::audit_shard(&record)).unwrap();
    let before_state = fs::read(record.join("aidlc-state.md")).unwrap();

    let output = workspace.doctor();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(stderr(&output), "");
    let text = stdout(&output);
    assert!(
        text.contains(&format!(
            "✓  Hooks last fired: fixture {}\n",
            advanced.format("%Y-%m-%dT%H:%M:%S%.3fZ")
        )),
        "{text}"
    );
    assert!(text.contains("✓  State Version: 8\n"), "{text}");
    assert!(
        text.contains("✓  Native workflow state readable\n"),
        "{text}"
    );
    assert!(text.contains("✓  Native workflow identity\n"), "{text}");
    assert!(text.contains("✓  Native event store readable\n"), "{text}");
    assert!(
        text.contains("✓  Native projection consistency\n"),
        "{text}"
    );
    assert!(
        text.ends_with(&format!("{RULE}\n34 passed, 0 failed\n")),
        "{text}"
    );
    assert!(
        !text.contains("Workflow diagnosis"),
        "対象外の欄を出さない: {text}"
    );

    let after_audit = fs::read_to_string(Workspace::audit_shard(&record)).unwrap();
    let appended = after_audit.strip_prefix(&before_audit).unwrap();
    assert_eq!(
        appended.matches("**Event**: HEALTH_CHECKED").count(),
        1,
        "{appended}"
    );
    assert!(
        appended.contains("**Request**: /aidlc --doctor\n"),
        "{appended}"
    );
    assert!(
        appended.contains("**Details**: 34 passed, 0 failed\n"),
        "{appended}"
    );
    assert_eq!(
        fs::read(record.join("aidlc-state.md")).unwrap(),
        before_state,
        "記録は状態ファイルを変えない"
    );

    let again = workspace.doctor();
    assert_eq!(again.status.code(), Some(0), "{again:?}");
    assert!(
        stdout(&again).contains("✓  Native projection consistency\n"),
        "自分の記録を投影し終えている: {}",
        stdout(&again)
    );
    let twice = fs::read_to_string(Workspace::audit_shard(&record)).unwrap();
    assert_eq!(twice.matches("**Event**: HEALTH_CHECKED").count(), 2);
}

#[test]
fn a_failed_report_is_still_recorded_with_its_actual_counts() {
    let workspace = Workspace::shipped();
    let record = workspace.initialize();
    let output = workspace.doctor();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(stderr(&output), "");
    let text = stdout(&output);
    assert!(
        text.contains("✗  Hooks have never executed although this workflow has progressed 4 stages — 1. Run /hooks"),
        "{text}"
    );
    assert!(
        text.ends_with(&format!("{RULE}\n33 passed, 1 failed\n")),
        "{text}"
    );
    let audit = fs::read_to_string(Workspace::audit_shard(&record)).unwrap();
    assert!(audit.contains("**Event**: HEALTH_CHECKED\n**Request**: /aidlc --doctor\n**Details**: 33 passed, 1 failed\n"), "{audit}");
}

#[test]
fn a_recording_failure_keeps_the_report_and_exits_one_without_a_forged_fact() {
    let workspace = Workspace::shipped();
    let record = workspace.initialize();
    let advanced = Workspace::newest_progress(&record);
    Workspace::fire_heartbeat(&record, "fixture", advanced);
    let before_audit = fs::read(Workspace::audit_shard(&record)).unwrap();
    let db = rusqlite::Connection::open(workspace.store()).unwrap();
    let before_rows: i64 = db
        .query_row("SELECT count(*) FROM journal", [], |row| row.get(0))
        .unwrap();
    db.execute_batch("CREATE TRIGGER fail_doctor_record BEFORE INSERT ON journal BEGIN SELECT RAISE(ABORT, 'diagnostic persistence unavailable'); END").unwrap();
    drop(db);

    let output = workspace.doctor();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = stdout(&output);
    assert!(
        text.starts_with("AI-DLC Health Check\n"),
        "診断出力は残る: {text}"
    );
    assert!(
        text.ends_with(&format!("{RULE}\n34 passed, 0 failed\n")),
        "{text}"
    );
    assert!(
        !stderr(&output).is_empty(),
        "記録失敗は stderr へ: {output:?}"
    );
    assert_eq!(
        fs::read(Workspace::audit_shard(&record)).unwrap(),
        before_audit
    );
    let db = rusqlite::Connection::open(workspace.store()).unwrap();
    let after_rows: i64 = db
        .query_row("SELECT count(*) FROM journal", [], |row| row.get(0))
        .unwrap();
    assert_eq!(after_rows, before_rows, "失敗した記録は事実を残さない");
}

#[test]
fn a_record_that_lost_its_state_file_is_reported_not_repaired() {
    let workspace = Workspace::shipped();
    let record = workspace.initialize();
    let advanced = Workspace::newest_progress(&record);
    Workspace::fire_heartbeat(&record, "fixture", advanced);
    fs::remove_file(record.join("aidlc-state.md")).unwrap();
    let output = workspace.doctor();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = stdout(&output);
    let relative = format!(
        "aidlc/spaces/default/intents/{}/aidlc-state.md",
        record.file_name().unwrap().to_string_lossy()
    );
    assert!(
        text.contains(&format!(
            "✗  Native workflow state readable — {relative}: missing\n"
        )),
        "{text}"
    );
    assert!(
        !text.contains("State Version:"),
        "本家 D4.a は状態ファイルが無ければ出ない: {text}"
    );
    assert!(
        !record.join("aidlc-state.md").exists(),
        "診断は失われた状態ファイルを復元しない"
    );
}

// ---------------------------------------------------------------------------
// Step 6 — 本家採取 (DC3〜DC9) との突き合わせ
// ---------------------------------------------------------------------------

/// 本家 doctor 全 50 行のうち C7 が採用する行 (ラベルの先頭で識別)。
fn is_adopted_upstream_row(label: &str) -> bool {
    const ADOPTED: [&str; 17] = [
        "bun installed",
        "Hook contract:",
        "Hooks enabled",
        "Hooks DISABLED",
        "Claude managed hook policy",
        "settings.json present",
        "Hook heartbeats:",
        "Hooks last fired",
        "Hooks have never executed",
        "Hook heartbeat data",
        "workspace shell ready",
        "Scope validation:",
        "Cycle detection:",
        "Orphan stage files:",
        "Schema validation:",
        "Graph references:",
        "State Version:",
    ];
    ADOPTED.iter().any(|prefix| label.starts_with(prefix))
        || label.starts_with("state version ")
        || (label.starts_with("aidlc-") && label.contains(".ts present"))
}

/// この build 固有の必須診断 (本家に同名の行は無い)。
fn is_native_row(label: &str) -> bool {
    label.starts_with("Native ")
}

/// 報告書の行 (先頭の区切りと末尾の集計を除く)。
fn report_rows(text: &str) -> Vec<String> {
    let body = text
        .strip_prefix(&format!("AI-DLC Health Check\n{RULE}\n"))
        .unwrap_or_else(|| panic!("ヘッダが違う: {text}"));
    body.split(&format!("\n{RULE}\n"))
        .next()
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

fn label_of(row: &str) -> &str {
    let row = row
        .strip_prefix("✓  ")
        .or_else(|| row.strip_prefix("✗  "))
        .unwrap_or(row);
    row.split(" — ").next().unwrap_or(row)
}

/// ISO 8601 UTC を `<TS>` に潰す (`normalization.json` の規則)。
fn normalize_timestamps(text: &str) -> String {
    let pattern = regex::Regex::new(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z").unwrap();
    pattern.replace_all(text, "<TS>").into_owned()
}

#[test]
fn upstream_doctor_observations_agree_on_adopted_rows_and_exit_codes() {
    let corpus: serde_json::Value = serde_json::from_slice(
        &fs::read(repo_root().join("tests/golden/upstream-a277af21/doctor/cases.json")).unwrap(),
    )
    .unwrap();
    let observations = corpus["observations"].as_array().unwrap();
    let mut compared = 0;
    for observation in observations {
        let id = observation["id"].as_str().unwrap();
        if id.starts_with("doctor/setup-") {
            continue;
        }
        if id == "doctor/audit-locked" {
            // 本家の mkdir ロックは TS 固有の機構であり、この build には対応する競合面が無い
            // (記録失敗は DC10 の SQLite 経路で固定する)。比較対象から外す。
            continue;
        }
        let scenario = id.strip_prefix("doctor/").unwrap();
        let workspace = Workspace::shipped();
        let record = if scenario == "initialized"
            || scenario.starts_with("version-")
            || scenario.starts_with("heartbeat-")
        {
            Some(workspace.initialize())
        } else {
            None
        };
        let record_name = record
            .as_ref()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned());
        let map_path = |path: &str| -> PathBuf {
            let mapped = match &record_name {
                Some(name) => path.replace("260908-doctor", name),
                None => path.to_string(),
            };
            workspace.root().join(mapped)
        };
        for (path, contents) in observation["fixture_changes"].as_object().unwrap() {
            let target = map_path(path);
            if path.ends_with(".aidlc-hooks-health/fixture.last") {
                // 採取と同じ相対計算: 最終進行 − 300000ms (boundary) / − 300001ms (stale)。
                let advanced = Workspace::newest_progress(record.as_ref().unwrap());
                let lag = if scenario == "heartbeat-boundary" {
                    300_000
                } else {
                    300_001
                };
                let at = advanced - chrono::Duration::milliseconds(lag);
                Workspace::fire_heartbeat(record.as_ref().unwrap(), "fixture", at);
                continue;
            }
            if path.ends_with("/aidlc-state.md") {
                // 採取と同じ置換を、この build が書いた状態ファイルへ当てる。
                let content = fs::read_to_string(&target).unwrap();
                let replacement = match scenario {
                    "version-missing" => String::new(),
                    "version-past" => "- **State Version**: 7".to_string(),
                    "version-future" => "- **State Version**: 9".to_string(),
                    other => panic!("状態ファイルを書き換える採取ケースではない: {other}"),
                };
                let rewritten: Vec<String> = content
                    .lines()
                    .map(|line| {
                        if line.starts_with("- **State Version**:") {
                            replacement.clone()
                        } else {
                            line.to_string()
                        }
                    })
                    .collect();
                fs::write(&target, format!("{}\n", rewritten.join("\n"))).unwrap();
                continue;
            }
            match contents.as_str() {
                None => fs::remove_file(&target).unwrap(),
                Some(encoded) => {
                    use base64::Engine as _;
                    let bytes = base64::engine::general_purpose::STANDARD
                        .decode(encoded)
                        .unwrap();
                    fs::create_dir_all(target.parent().unwrap()).unwrap();
                    fs::write(&target, bytes).unwrap();
                }
            }
        }
        for directory in observation["fixture_directories"].as_array().unwrap() {
            fs::create_dir_all(map_path(directory.as_str().unwrap())).unwrap();
        }

        let output = workspace.cli_with("aidlc", &["--doctor"], scenario != "missing-bun");
        let expected_exit = observation["output"]["exit_code"].as_i64().unwrap();
        assert_eq!(
            output.status.code(),
            Some(i32::try_from(expected_exit).unwrap()),
            "{scenario}: {output:?}"
        );
        assert_eq!(stderr(&output), "", "{scenario}");
        let ours = report_rows(&stdout(&output));
        let theirs = report_rows(observation["output"]["stdout"].as_str().unwrap());

        // 表示順は C7 の表順 (D2.f が D3.a より先、D4.a は D3 の後) で本家と異なるので、
        // 行の集合として突き合わせる。順序は下の C7 表順の検査が固定する。
        let mut expected: Vec<String> = theirs
            .iter()
            .filter(|row| is_adopted_upstream_row(label_of(row)))
            .map(|row| normalize_timestamps(row))
            .collect();
        expected.sort();
        let mut actual: Vec<String> = ours
            .iter()
            .filter(|row| !is_native_row(label_of(row)))
            .map(|row| normalize_timestamps(row))
            .collect();
        actual.sort();
        for (mine, upstream) in actual.iter().zip(&expected) {
            if upstream.starts_with("✓  Scope validation: ") {
                // C7 D3.b は対象を bugfix / feature の 2 スコープに限定するので集計だけが違う。
                assert_eq!(
                    mine, "✓  Scope validation: 2 scopes valid (6 advisories)",
                    "{scenario}"
                );
                continue;
            }
            assert_eq!(mine, upstream, "{scenario}: 採用行はバイト一致する");
        }
        assert_eq!(
            actual.len(),
            expected.len(),
            "{scenario}: 採用行の数\nours={actual:#?}\ntheirs={expected:#?}"
        );

        let order: Vec<usize> = [
            "bun installed",
            "Native engine entry points",
            "Hook",
            "settings.json present",
            "Native hook bindings",
            "workspace shell ready",
            "Scope validation:",
            "Cycle detection:",
            "Orphan stage files:",
            "Schema validation:",
            "Graph references:",
        ]
        .iter()
        .filter_map(|prefix| {
            ours.iter()
                .position(|row| label_of(row).starts_with(prefix))
        })
        .collect();
        assert!(
            order.windows(2).all(|pair| pair[0] < pair[1]),
            "{scenario}: C7 の表順 {order:?}"
        );
        assert!(
            ours.iter()
                .any(|row| row == "✓  Native engine entry points"),
            "{scenario}"
        );
        assert!(
            ours.iter()
                .any(|row| label_of(row) == "Native hook bindings"),
            "{scenario}"
        );
        let record_rows = ours.iter().filter(|row| {
            matches!(
                label_of(row),
                "Native workflow state readable"
                    | "Native workflow identity"
                    | "Native event store readable"
                    | "Native projection consistency"
            )
        });
        assert_eq!(
            record_rows.count(),
            if record.is_some() { 4 } else { 0 },
            "{scenario}"
        );
        assert!(
            !stdout(&output).contains("Workflow diagnosis"),
            "{scenario}: 対象外の欄を出さない"
        );
        for row in &ours {
            let label = label_of(row);
            assert!(
                is_adopted_upstream_row(label) || is_native_row(label),
                "{scenario}: 対象外の本家行を出している: {row}"
            );
        }
        compared += 1;
    }
    assert_eq!(
        compared, 19,
        "採取 20 観測のうち audit-locked を除く 19 を比較した"
    );
}

/// 独自診断 (D2.e / D4.b / D4.c / D5.a / D5.b) を異常注入で失敗させる。D1.b (入口欠落) は
/// バイナリ自身の配線表なので子プロセスからは注入できず、集約の契約テストが持つ。
#[test]
fn native_checks_fail_on_injected_binding_state_identity_store_and_projection_faults() {
    let row = |text: &str, label: &str| -> String {
        text.lines()
            .find(|line| label_of(line) == label)
            .unwrap_or_else(|| panic!("{label} の行が無い: {text}"))
            .to_string()
    };

    // D2.e — この build のフック面に無い名前を native 形で書いた登録。
    //
    // native と配布の混在そのものは失敗ではない (混ぜ方を語るのは U4 の接続定義であり、
    // 定義どおりの混在は正常な接続の姿である)。定義と登録の食い違いは U4 の
    // `harness_binding_contract` が接続定義を置いた作業ツリーで固定する。
    let workspace = Workspace::shipped();
    let settings_path = workspace.root().join(".claude/settings.json");
    let settings = fs::read_to_string(&settings_path).unwrap();
    fs::write(
        &settings_path,
        settings.replacen(
            "bun \\\"$CLAUDE_PROJECT_DIR/.claude/hooks/aidlc-record-human-turn.ts\\\"",
            "\\\"$CLAUDE_PROJECT_DIR/target/release/aidlc\\\" hook frobnicate",
            1,
        ),
    )
    .unwrap();
    let output = workspace.doctor();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(
        row(&stdout(&output), "Native hook bindings"),
        "✗  Native hook bindings — .claude/settings.json: binding mismatch (unknown native hook frobnicate)"
    );

    // D4.b — 状態ファイルが読めない (ディレクトリになっている)。
    let workspace = Workspace::shipped();
    let record = workspace.initialize();
    let name = record.file_name().unwrap().to_string_lossy().into_owned();
    Workspace::fire_heartbeat(&record, "fixture", Workspace::newest_progress(&record));
    fs::remove_file(record.join("aidlc-state.md")).unwrap();
    fs::create_dir_all(record.join("aidlc-state.md")).unwrap();
    let output = workspace.doctor();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = stdout(&output);
    assert!(
        row(&text, "Native workflow state readable").starts_with(&format!(
            "✗  Native workflow state readable — aidlc/spaces/default/intents/{name}/aidlc-state.md: unreadable ("
        )),
        "{text}"
    );
    assert!(!text.contains("State Version:"), "{text}");

    // D4.c — 登録簿が別の記録を名指す。
    let workspace = Workspace::shipped();
    let record = workspace.initialize();
    let name = record.file_name().unwrap().to_string_lossy().into_owned();
    Workspace::fire_heartbeat(&record, "fixture", Workspace::newest_progress(&record));
    let registry = workspace
        .root()
        .join("aidlc/spaces/default/intents/intents.json");
    let text = fs::read_to_string(&registry).unwrap();
    fs::write(&registry, text.replace(&name, "260101-elsewhere")).unwrap();
    let output = workspace.doctor();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        row(&stdout(&output), "Native workflow identity")
            .ends_with("is registered to 260101-elsewhere)"),
        "{}",
        stdout(&output)
    );

    // D5.a / D5.b — ストアが読めない (ディレクトリになっている)。記録も失敗するので stderr が出る。
    let workspace = Workspace::shipped();
    let record = workspace.initialize();
    Workspace::fire_heartbeat(&record, "fixture", Workspace::newest_progress(&record));
    fs::remove_file(workspace.store()).unwrap();
    fs::create_dir_all(workspace.store()).unwrap();
    let output = workspace.doctor();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = stdout(&output);
    assert_eq!(
        row(&text, "Native event store readable"),
        "✗  Native event store readable — aidlc/spaces/default/intents/.aidlc-store.sqlite: unreadable (is a directory)"
    );
    assert!(
        row(&text, "Native projection consistency").contains("projection unavailable"),
        "{text}"
    );
    assert!(
        row(&text, "Native workflow identity").contains("unreadable"),
        "{text}"
    );

    // D5.b — 実行行の投影が欠けている (読み面だけを消す。ジャーナルは触らない)。
    let workspace = Workspace::shipped();
    let record = workspace.initialize();
    Workspace::fire_heartbeat(&record, "fixture", Workspace::newest_progress(&record));
    let db = rusqlite::Connection::open(workspace.store()).unwrap();
    db.execute("DELETE FROM read_execution", []).unwrap();
    drop(db);
    let output = workspace.doctor();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = stdout(&output);
    assert!(
        row(&text, "Native projection consistency").contains("not projected"),
        "{text}"
    );
    assert!(
        row(&text, "Native event store readable").starts_with("✓  "),
        "{text}"
    );
}
