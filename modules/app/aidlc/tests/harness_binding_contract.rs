//! 配布フック登録 → この build のフック面 (`aidlc engine hook <name>`) の接続契約 (C6 / C8)。
//!
//! 配布 `.claude/settings.json` の `hooks` ブロックが名指す 16 本を、接続定義
//! (`scripts/aidlc-selfhost/hook-binding.json`) が漏れなく分類することと、native 側へ向けた
//! 登録が Claude Code と同じ起動形 (`sh -c "<command>"`、`aidlc` は `PATH` から解決する) で
//! この build を起動し、標準入力・終了コード・標準出力/標準エラーの契約を守ることを確かめる。
//!
//! 検証はすべて一時ワークスペースで行い、本リポジトリの実ファイルは読むだけである。
//! 準備がここで green になっても、それは実地スモーク (FR7) や切替 (FR8) の成功証拠ではない
//! (C8 `not_proof_of_completion`)。
// 契約テストは固定の添字参照と panic を検証の合図として使う (既存の契約テストと同じ許容)。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeSet;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use core_infrastructure::canon_json::{JsonValue, ObjectMembers, SerializationProfile, serialize};

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;

/// この build のバイナリを `target` の名前で置く — **シンボリックリンク**で。
///
/// 共有テスト装置の `tool_link` はハードリンクを張るが、一時ディレクトリの後始末が
/// その実体のリンク数を動かすため、同じ実体を並列に実行している別のテストが macOS の
/// 署名検証で落ちることがある (U3 の `tdd-logs/16-flaky-recheck.log` に同種の記録)。
/// こちらは実体に触れないので、並列でも安定する。
fn place_binary(target: &Path) {
    let source = env!("CARGO_BIN_EXE_aidlc");
    #[cfg(unix)]
    {
        if std::os::unix::fs::symlink(source, target).is_ok() {
            return;
        }
    }
    fs::copy(source, target).map(|_| ()).unwrap();
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

/// U4 が用意するフック接続定義。
fn binding_definition() -> serde_json::Value {
    let path = repo_root().join("scripts/aidlc-selfhost/hook-binding.json");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("接続定義が読めない ({}): {error}", path.display()));
    serde_json::from_str(&raw).expect("接続定義は JSON")
}

fn names(value: &serde_json::Value, key: &str) -> Vec<String> {
    value[key]
        .as_array()
        .unwrap_or_else(|| panic!("{key} は配列"))
        .iter()
        .map(|entry| match entry {
            serde_json::Value::String(name) => name.clone(),
            other => other["name"].as_str().expect("name").to_string(),
        })
        .collect()
}

/// 配布 `.claude/settings.json` の `hooks` ブロックが名指すフック名 (名前順・重複なし)。
///
/// 出所は封印済みの配布シェル (`tests/golden/selfhost-stage1/doctor-shell`、固定本家
/// `a277af21` の `dist/claude` から写したもの) であって、書き換え得る作業ツリーの設定でも、
/// CI の Rust ジョブが取得しない submodule (`vendor/aidlc-workflows`) でもない。
/// 「配布が何本を登録しているか」はここが正本である。
fn distributed_hook_names() -> BTreeSet<String> {
    let raw = fs::read_to_string(
        repo_root().join("tests/golden/selfhost-stage1/doctor-shell/.claude/settings.json"),
    )
    .expect("配布 settings.json");
    let settings: serde_json::Value =
        serde_json::from_str(&raw).expect("配布 settings.json は JSON");
    let mut found = BTreeSet::new();
    for groups in settings["hooks"].as_object().expect("hooks").values() {
        for group in groups.as_array().into_iter().flatten() {
            for hook in group["hooks"].as_array().into_iter().flatten() {
                let command = hook["command"].as_str().unwrap_or_default();
                if let Some(name) = hook_name_of(command) {
                    found.insert(name);
                }
            }
        }
    }
    found
}

/// `bun ".../aidlc-<name>.ts"` からフック名を取り出す。
fn hook_name_of(command: &str) -> Option<String> {
    let start = command.find("aidlc-")? + "aidlc-".len();
    let tail = &command[start..];
    let end = tail.find(".ts")?;
    Some(tail[..end].to_string())
}

/// 接続定義の起動形を 1 本ぶんに展開する。
///
/// 2.8.2 の登録は `aidlc engine hook <name>` であり、`aidlc` を `PATH` から解決する。
/// 一段形のまま展開すると、この後の起動は「実体が見つからない」(127) で落ち、接続の契約を
/// 見たのか定義の取り違えを見たのか区別できなくなる。だからここで先に止める。
fn render(definition: &serde_json::Value, hook: &str) -> String {
    let rendered = definition["command_template"]
        .as_str()
        .expect("command_template")
        .replace("{hook}", hook);
    assert!(
        rendered.starts_with("aidlc engine hook "),
        "接続定義の起動形が 2.8.2 の二段形ではない — {rendered}"
    );
    rendered
}

/// Claude Code と同じ起動形を再現した一時ワークスペース。
struct Workspace {
    temp: tempfile::TempDir,
}

impl Workspace {
    /// `PATH` 上の `aidlc` としてこの build を置き、bun の無い `PATH` を用意する。
    ///
    /// 2.8.2 の登録は `aidlc engine hook <name>` であり、`aidlc` を `PATH` から解決する
    /// (`.claude/settings.json` はパスを綴らない)。だから接続の実体も `PATH` に置く。
    ///
    /// bun を外すのは、登録が配布 `.ts` へ落ちていれば `bun` が見つからず失敗する —
    /// つまり「この build が起動した」ことの証明になるからである。
    fn new() -> Self {
        Self::with_binary(true)
    }

    /// バイナリを置かない形 — 実体を欠いた登録の失敗経路を見るために使う。
    ///
    /// 置いたリンクを**消す**のではなくはじめから置かないのは、この build のバイナリを
    /// 並列のテストが同じ実体で実行しているからである (`tool_link` はハードリンクを張る)。
    fn without_binary() -> Self {
        Self::with_binary(false)
    }

    fn with_binary(present: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        for relative in [".claude/hooks", "aidlc/spaces/default/memory"] {
            fs::create_dir_all(root.join(relative)).unwrap();
        }
        fs::create_dir_all(temp.path().join("bin")).unwrap();
        if present {
            place_binary(&temp.path().join("bin/aidlc"));
        }
        Self { temp }
    }

    fn root(&self) -> PathBuf {
        self.temp.path().join("workspace")
    }

    /// `aidlc` を解決できる `PATH` (bun は載せない)。
    fn path_env(&self) -> String {
        format!("{}:/usr/bin:/bin", self.temp.path().join("bin").display())
    }

    /// 登録されたコマンド行を、Claude Code と同じく `sh -c` と `CLAUDE_PROJECT_DIR` で起動する。
    fn fire(&self, command: &str, stdin: &str) -> Output {
        let mut child = Command::new("/bin/sh")
            .arg("-c")
            .arg(command)
            .current_dir(self.root())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.temp.path())
            .env("PATH", self.path_env())
            .env("CLAUDE_PROJECT_DIR", self.root())
            .env("LANG", "C.UTF-8")
            .env("LC_ALL", "C.UTF-8")
            .env("TZ", "UTC")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    }

    /// 作業記録の面 (`aidlc/`) に生まれたファイルの相対パス。
    fn workflow_files(&self) -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        collect(&self.root().join("aidlc"), &self.root(), &mut found);
        found
    }
}

fn collect(dir: &Path, base: &Path, found: &mut BTreeSet<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, base, found);
        } else if let Ok(relative) = path.strip_prefix(base) {
            found.insert(relative.display().to_string());
        }
    }
}

fn payload(workspace: &Workspace, session: &str) -> String {
    format!(
        r#"{{"session_id":"{session}","cwd":"{}","hook_event_name":"PreToolUse","tool_name":"Read","tool_input":{{"file_path":"README.md"}}}}"#,
        workspace.root().display()
    )
}

/// 接続定義は、配布が登録する 16 本を native / 配布のどちらかへ漏れなく分類する。
#[test]
fn the_binding_definition_classifies_every_hook_the_distribution_registers() {
    let definition = binding_definition();
    let native: BTreeSet<String> = names(&definition, "native_hooks").into_iter().collect();
    let distributed: BTreeSet<String> = names(&definition, "distributed_hooks")
        .into_iter()
        .collect();
    let registered = distributed_hook_names();
    assert!(
        native.is_disjoint(&distributed),
        "同じフックを native と配布の両方へ置かない: {:?}",
        native.intersection(&distributed).collect::<Vec<_>>()
    );
    let covered: BTreeSet<String> = native.union(&distributed).cloned().collect();
    assert_eq!(
        covered, registered,
        "配布登録の分類漏れ・架空の追加がある (配布={registered:?})"
    );
    // 配布側へ残す分は理由を必ず持つ — 「native に無い」ことを黙って落とさない。
    for entry in definition["distributed_hooks"].as_array().unwrap() {
        assert!(
            entry["reason"]
                .as_str()
                .is_some_and(|text| !text.is_empty()),
            "配布のまま残す理由が無い: {entry}"
        );
    }
}

/// native へ向けた登録は、bun の無い `PATH` でもこの build を起動して受理する。
#[test]
fn every_native_registration_starts_this_build_without_bun_on_the_path() {
    let definition = binding_definition();
    let workspace = Workspace::new();
    for hook in names(&definition, "native_hooks") {
        let command = render(&definition, &hook);
        let output = workspace.fire(&command, &payload(&workspace, "binding-start"));
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        assert_ne!(
            output.status.code(),
            Some(127),
            "{hook}: 起動できていない (配布 .ts へ落ちている): {stderr}"
        );
        assert!(
            !stderr.contains("not found") && !stderr.contains("Unknown hook"),
            "{hook}: この build のフック面が受理していない: {stderr}"
        );
        assert_eq!(output.status.code(), Some(0), "{hook}: {stderr}");
    }
}

/// 登録形は標準入力をそのままフックへ渡す (`session-start` は受けた識別子を返す)。
#[test]
fn the_registration_hands_the_hook_its_standard_input() {
    let definition = binding_definition();
    let workspace = Workspace::new();
    let output = workspace.fire(
        &render(&definition, "session-start"),
        &payload(&workspace, "handed-through-session"),
    );
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("handed-through-session"),
        "標準入力のセッション識別子が渡っていない: {stdout}"
    );
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());
}

/// 未知のフック名は、同じ登録形でも拒否される (静かに成功へ倒さない)。
#[test]
fn an_unknown_hook_name_is_refused_through_the_same_registration_form() {
    let definition = binding_definition();
    let workspace = Workspace::new();
    let before = workspace.workflow_files();
    let output = workspace.fire(
        &render(&definition, "not-a-hook"),
        &payload(&workspace, "unknown-hook"),
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "Unknown hook: not-a-hook\n"
    );
    assert_eq!(
        workspace.workflow_files(),
        before,
        "拒否で作業記録を作らない"
    );
}

/// 配布のまま残す 2 本は、native のフック名としては受け付けない。
#[test]
fn the_hooks_kept_distributed_are_not_accepted_as_native_names() {
    let definition = binding_definition();
    let workspace = Workspace::new();
    for hook in names(&definition, "distributed_hooks") {
        let output = workspace.fire(&render(&definition, &hook), &payload(&workspace, "kept"));
        assert_eq!(output.status.code(), Some(1), "{hook}");
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            format!("Unknown hook: {hook}\n"),
            "{hook}: native にあるかのように扱わない"
        );
    }
}

/// 起動時に作業ディレクトリを保ち、フック入力をシェルの文字列展開で読み替えない (C6)。
#[test]
fn the_registration_keeps_the_working_directory_and_never_expands_the_hook_input() {
    let definition = binding_definition();
    let workspace = Workspace::new();
    let other = Workspace::new();
    let marker = workspace.root().join("expanded-by-the-shell");
    let hostile = format!(
        r#"{{"session_id":"s-$(touch '{}')-`touch '{}'`-*","cwd":"{}","hook_event_name":"SessionStart"}}"#,
        marker.display(),
        marker.display(),
        workspace.root().display()
    );
    let output = workspace.fire(&render(&definition, "session-start"), &hostile);
    assert_eq!(output.status.code(), Some(0));
    assert!(!marker.exists(), "フック入力がシェルで展開された");
    // 危険な識別子は**データとして拒否**される — 実行された結果ではない。
    assert!(
        output.stdout.is_empty(),
        "安全でない識別子をそのまま束縛した: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    // 安全な識別子は逐語で往復し、記録は起動された作業ツリーの下だけに生まれる。
    let safe = workspace.fire(
        &render(&definition, "session-start"),
        &payload(&workspace, "s-kept-here-1"),
    );
    assert_eq!(safe.status.code(), Some(0));
    assert!(
        String::from_utf8(safe.stdout)
            .unwrap()
            .contains("s-kept-here-1")
    );
    assert!(
        workspace
            .workflow_files()
            .iter()
            .any(|path| path.starts_with("aidlc/.aidlc-sessions/")),
        "起動された作業ツリーにセッション束縛が無い"
    );
    assert!(
        other.workflow_files().is_empty(),
        "起動していない作業ツリーへ書いた: {:?}",
        other.workflow_files()
    );
}

/// 実体を欠いた登録は失敗として返り、別の更新実装へ切り替わらない (C6)。
#[test]
fn a_binding_that_cannot_launch_fails_loudly_and_writes_no_workflow_state() {
    let definition = binding_definition();
    let workspace = Workspace::without_binary();
    let before = workspace.workflow_files();
    let output = workspace.fire(
        &render(&definition, "write-audit-log"),
        &payload(&workspace, "no-binary"),
    );
    assert_ne!(output.status.code(), Some(0), "失敗を成功へ丸めない");
    assert!(output.stdout.is_empty());
    assert!(
        !String::from_utf8_lossy(&output.stderr).is_empty(),
        "失敗の理由を黙らせない"
    );
    assert_eq!(
        workspace.workflow_files(),
        before,
        "接続の失敗を別の更新実装で埋めない"
    );
}

/// 接続の形ごとに、この build の自己診断 (C7 の D2.e) が何を返すかの実測。
///
/// D2.e は接続定義 (`scripts/aidlc-selfhost/hook-binding.json`) を正として照合する。
/// **宣言どおりの混在は正常**であり、宣言と食い違う面・この build のフック面に無い native 名・
/// 解決できない呼出しが失敗である。定義の無い作業ツリー (配布そのまま) では照合する相手が
/// 無いので、混ざり方そのものは判定しない。
///
/// ここでの成功は実地スモーク (FR7) や切替 (FR8) の達成証拠ではない
/// (C8 `not_proof_of_completion`)。
#[test]
fn the_self_diagnosis_verdict_of_each_binding_layout_is_measured_not_assumed() {
    let definition = binding_definition();
    let native: BTreeSet<String> = names(&definition, "native_hooks").into_iter().collect();
    // (接続の形, 接続定義を置くか, 期待する D2.e の行, 期待する終了コード)
    let expectations: [(&str, bool, &str, i32); 7] = [
        // --- 接続定義が無い作業ツリー (配布そのまま) ---
        // 配布 16 本のまま。native 面はどの登録からも起動しない。
        ("distributed", false, "✓  Native hook bindings", 0),
        // native 14 本 + 配布 2 本。照合する宣言が無いので、混在そのものは失敗にしない。
        ("mixed", false, "✓  Native hook bindings", 0),
        // 16 本すべて native 形 — 2 本はこの build のフック面に無い。
        (
            "all-native",
            false,
            "✗  Native hook bindings — .claude/settings.json: binding mismatch (unknown native hook plan-approval-guard)",
            1,
        ),
        // --- 接続定義がある作業ツリー (本リポジトリの姿) ---
        // 配布 16 本のままは、宣言が native と言った名前を配布 `.ts` で登録している状態である。
        (
            "distributed",
            true,
            "✗  Native hook bindings — .claude/settings.json: binding mismatch (declared native, registered distributed: fold-usage)",
            1,
        ),
        // 宣言どおりの接続 — native 14 本 + 配布 2 本。これが切替後の姿である。
        ("mixed", true, "✓  Native hook bindings", 0),
        // 宣言に無い名前を native 形で書いた登録は、宣言の照合より先に落ちる。
        (
            "all-native",
            true,
            "✗  Native hook bindings — .claude/settings.json: binding mismatch (unknown native hook plan-approval-guard)",
            1,
        ),
        // 宣言が配布のままと言った 2 本を登録ごと落とした形。登録が無いことは食い違いではない。
        ("native-only", true, "✓  Native hook bindings", 0),
    ];
    for (layout, declare, expected_binding_row, expected_exit) in expectations {
        let shell = DoctorShell::with_layout(layout, &native, declare);
        let output = shell.doctor();
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(
            stdout.lines().any(|line| line == expected_binding_row),
            "{layout} (declare={declare}): D2.e 行が実測と違う\nSTDOUT={stdout}\nSTDERR={}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            output.status.code(),
            Some(expected_exit),
            "{layout} (declare={declare}): 終了コードが実測と違う\n{stdout}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).is_empty(),
            "{layout} (declare={declare})"
        );
    }
}

/// 配布シェルの `statusLine` を本リポジトリ現物 (`statusline-combined.sh`) に揃えると、
/// native 14 本だけの形は D2.a (`aidlc-*.ts` を 1 本も登録していない) で落ちる。
///
/// 配布 2 本の登録を残すのはこの D2.a のためでもある。D2.e の追従はこの行を変えない。
#[test]
fn a_native_only_layout_without_any_distributed_reference_fails_the_upstream_hook_contract_row() {
    let definition = binding_definition();
    let native: BTreeSet<String> = names(&definition, "native_hooks").into_iter().collect();
    let shell = DoctorShell::with_layout("native-only-repo-statusline", &native, true);
    let output = shell.doctor();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout
            .lines()
            .any(|line| line
                .starts_with("✗  Hook contract: settings.json wires no aidlc-*.ts hooks")),
        "D2.a の失敗行が無い\nSTDOUT={stdout}\nSTDERR={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.lines().any(|line| line == "✓  Native hook bindings"),
        "D2.e は通るはず\n{stdout}"
    );
    assert_eq!(output.status.code(), Some(1));
}

/// 配布シェルの部分集合から組む doctor 用の一時ワークスペース。
///
/// `doctor_contract.rs` と同じ材料 (封印済み `doctor-shell` + 配布グラフ 3 入力 + bun の代役) を使う。
struct DoctorShell {
    temp: tempfile::TempDir,
}

impl DoctorShell {
    fn with_layout(layout: &str, native: &BTreeSet<String>, declare: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        fs::create_dir_all(&root).unwrap();
        copy_tree(
            &repo_root().join("tests/golden/selfhost-stage1/doctor-shell"),
            &root,
        );
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
        let settings_path = root.join(".claude/settings.json");
        let raw = fs::read_to_string(&settings_path).unwrap();
        let settings =
            core_infrastructure::canon_json::parse(&raw).expect("配布 settings.json は JSON");
        let settings = rewrite_settings(&settings, layout, native);
        fs::write(
            &settings_path,
            serialize(&settings, SerializationProfile::ContractPretty),
        )
        .unwrap();
        let hooks = root.join(".claude/hooks");
        fs::create_dir_all(&hooks).unwrap();
        let rendered = fs::read_to_string(&settings_path).unwrap();
        for name in referenced_tool_files(&rendered) {
            fs::write(hooks.join(name), "").unwrap();
        }
        fs::create_dir_all(root.join("aidlc/spaces/default/memory")).unwrap();
        if declare {
            // このリポジトリ固有の接続定義を、実物のまま作業ツリーへ置く。
            let declaration = root.join("scripts/aidlc-selfhost");
            fs::create_dir_all(&declaration).unwrap();
            fs::copy(
                repo_root().join("scripts/aidlc-selfhost/hook-binding.json"),
                declaration.join("hook-binding.json"),
            )
            .unwrap();
        }
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

    fn doctor(&self) -> Output {
        let root = self.temp.path().join("workspace");
        let binary = self.temp.path().join("aidlc");
        if !binary.exists() {
            place_binary(&binary);
        }
        Command::new(binary)
            .arg("--doctor")
            .current_dir(&root)
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.temp.path().join("home"))
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.temp.path().join("bin").display()),
            )
            .env(
                "AIDLC_MANAGED_SETTINGS_PATH",
                root.join("aidlc/.capture-managed.json"),
            )
            .env("LANG", "C.UTF-8")
            .env("LC_ALL", "C.UTF-8")
            .env("TZ", "UTC")
            .output()
            .unwrap()
    }
}

/// 設定全体を、指定した接続の形へ書き換える (`hooks` と、必要なら `statusLine`)。
fn rewrite_settings(settings: &JsonValue, layout: &str, native: &BTreeSet<String>) -> JsonValue {
    let JsonValue::Object(members) = settings else {
        panic!("settings.json はオブジェクト");
    };
    let mut rewritten = ObjectMembers::new();
    for (key, value) in members.iter() {
        let value = match key {
            "hooks" => rewrite_hooks(value, layout, native),
            "statusLine" if layout == "native-only-repo-statusline" => repo_statusline(),
            _ => value.clone(),
        };
        rewritten.insert(key, value);
    }
    JsonValue::Object(rewritten)
}

/// 本リポジトリ現物の `statusLine` (配布の `aidlc-statusline.ts` ではない)。
fn repo_statusline() -> JsonValue {
    let mut members = ObjectMembers::new();
    members.insert("type", JsonValue::String("command".to_string()));
    members.insert(
        "command",
        JsonValue::String(
            "sh \"$CLAUDE_PROJECT_DIR/.claude/hooks/statusline-combined.sh\"".to_string(),
        ),
    );
    JsonValue::Object(members)
}

/// `hooks` ブロックの登録を、指定した接続の形へ書き換える。
fn rewrite_hooks(hooks: &JsonValue, layout: &str, native: &BTreeSet<String>) -> JsonValue {
    let JsonValue::Object(events) = hooks else {
        panic!("hooks はオブジェクト");
    };
    let mut rewritten = ObjectMembers::new();
    for (event, groups) in events.iter() {
        let JsonValue::Array(groups) = groups else {
            panic!("{event} の登録は配列");
        };
        let kept_groups: Vec<JsonValue> = groups
            .iter()
            .filter_map(|group| rewrite_group(group, layout, native))
            .collect();
        if !kept_groups.is_empty() {
            rewritten.insert(event, JsonValue::Array(kept_groups));
        }
    }
    JsonValue::Object(rewritten)
}

/// 1 つの matcher グループ。登録が全部落ちたらグループごと消す。
fn rewrite_group(group: &JsonValue, layout: &str, native: &BTreeSet<String>) -> Option<JsonValue> {
    let JsonValue::Object(members) = group else {
        panic!("グループはオブジェクト");
    };
    let JsonValue::Array(entries) = members.get("hooks")? else {
        panic!("グループの hooks は配列");
    };
    let kept: Vec<JsonValue> = entries
        .iter()
        .filter_map(|entry| rewrite_entry(entry, layout, native))
        .collect();
    if kept.is_empty() {
        return None;
    }
    let mut rewritten = ObjectMembers::new();
    for (key, value) in members.iter() {
        let value = if key == "hooks" {
            JsonValue::Array(kept.clone())
        } else {
            value.clone()
        };
        rewritten.insert(key, value);
    }
    Some(JsonValue::Object(rewritten))
}

/// 登録 1 件。この build のフック面に無いものは、形によっては落とす。
fn rewrite_entry(entry: &JsonValue, layout: &str, native: &BTreeSet<String>) -> Option<JsonValue> {
    let JsonValue::Object(members) = entry else {
        panic!("登録はオブジェクト");
    };
    let Some(JsonValue::String(command)) = members.get("command") else {
        return Some(entry.clone());
    };
    let Some(name) = hook_name_of(command) else {
        return Some(entry.clone());
    };
    let command = match placement(layout, native.contains(&name)) {
        Placement::Distributed => command.clone(),
        Placement::Native => native_command(&name),
        // native-only の 2 形 — この build のフック面に無い登録は落とす。
        Placement::Drop => return None,
    };
    let mut rewritten = ObjectMembers::new();
    for (key, value) in members.iter() {
        let value = if key == "command" {
            JsonValue::String(command.clone())
        } else {
            value.clone()
        };
        rewritten.insert(key, value);
    }
    Some(JsonValue::Object(rewritten))
}

/// 1 本の登録を、その形でどこへ置くか。
enum Placement {
    Distributed,
    Native,
    Drop,
}

fn placement(layout: &str, is_native: bool) -> Placement {
    match (layout, is_native) {
        ("distributed", _) => Placement::Distributed,
        ("all-native", _) | (_, true) => Placement::Native,
        ("mixed", false) => Placement::Distributed,
        (_, false) => Placement::Drop,
    }
}

fn native_command(hook: &str) -> String {
    format!("\"$CLAUDE_PROJECT_DIR/target/release/aidlc\" hook {hook}")
}

/// 設定本文が名指す `aidlc-*.ts` (doctor の D2.a と同じ拾い方)。
fn referenced_tool_files(raw: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = raw;
    while let Some(start) = rest.find("aidlc-") {
        let tail = &rest[start..];
        let end = tail[6..]
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .map_or(tail.len(), |offset| 6 + offset);
        if tail[end..].starts_with(".ts") {
            found.insert(format!("{}.ts", &tail[..end]));
        }
        rest = &tail[1..];
    }
    found
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
