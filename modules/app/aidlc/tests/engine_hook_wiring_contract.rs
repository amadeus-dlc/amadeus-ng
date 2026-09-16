//! 2.8.2 のフック登録形 (`aidlc engine hook <name>`) に対する自己診断と接続定義の契約。
//!
//! 配布物が aidlc 2.8.2 へ上がり、`.claude/settings.json` のフック登録が二段形になった。
//! この形に対して `Hook contract` (D2.a) と `Native hook bindings` (D2.e) が合格することと、
//! 接続定義 (`scripts/aidlc-selfhost/hook-binding.json`) が**登録件数と名前数を別々に**
//! 記録し、bugfix 1 周で発火する登録としない登録を根拠つきで分けることを確かめる。
//!
//! 診断項目の期待値は弱めない。ここが赤いときに直すのは観測側であって、判定側の期待文言でも
//! フィクスチャでもない。
//!
//! 作業ツリーの `.claude/settings.json` は**読むだけ**である。診断は一時ワークスペースへ
//! 写した複製に対して走らせる。
// 契約テストは固定の添字参照と panic を検証の合図として使う (既存の契約テストと同じ許容)。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::{BTreeMap, BTreeSet};
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

// ---------------------------------------------------------------------------
// 作業ツリーの 2.8.2 登録を読む (読むだけ)
// ---------------------------------------------------------------------------

/// `.claude/settings.json` の登録 1 件。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Registration {
    event: String,
    matcher: String,
    hook: String,
}

fn settings_text() -> String {
    fs::read_to_string(repo_root().join(".claude/settings.json")).expect("配布 settings.json")
}

/// `aidlc engine hook <name>` の登録をすべて拾う (登録の重複は畳まない)。
fn registrations() -> Vec<Registration> {
    let settings: serde_json::Value =
        serde_json::from_str(&settings_text()).expect("settings.json は JSON");
    let mut found = Vec::new();
    for (event, groups) in settings["hooks"].as_object().expect("hooks") {
        for group in groups.as_array().into_iter().flatten() {
            let matcher = group["matcher"].as_str().unwrap_or_default().to_string();
            for hook in group["hooks"].as_array().into_iter().flatten() {
                let command = hook["command"].as_str().unwrap_or_default();
                if let Some(name) = two_stage_hook_name(command) {
                    found.push(Registration {
                        event: event.clone(),
                        matcher: matcher.clone(),
                        hook: name,
                    });
                }
            }
        }
    }
    found.sort();
    found
}

/// `… aidlc engine hook <name>` の `<name>`。二段形でなければ `None`。
fn two_stage_hook_name(command: &str) -> Option<String> {
    let words: Vec<&str> = command.split_whitespace().collect();
    let index = words.iter().position(|word| *word == "hook")?;
    let program = words.get(index.checked_sub(1)?)?;
    if *program != "engine" {
        return None;
    }
    words.get(index + 1).map(|name| (*name).to_string())
}

fn registered_hook_names() -> BTreeSet<String> {
    registrations()
        .into_iter()
        .map(|registration| registration.hook)
        .collect()
}

// ---------------------------------------------------------------------------
// 一時ワークスペースでの自己診断
// ---------------------------------------------------------------------------

struct Workspace {
    temp: tempfile::TempDir,
}

impl Workspace {
    /// 配布シェルの部分集合 + 配布グラフ 3 入力に、**作業ツリーの 2.8.2 登録**を重ねた形。
    ///
    /// フック実体は登録が名指す名前ぶんだけ空ファイルで置く (doctor は存在だけを見る)。
    fn with_two_stage_settings() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        fs::create_dir_all(&root).unwrap();
        copy_tree(
            &repo_root().join("tests/golden/selfhost-stage1/doctor-shell"),
            &root,
        );
        let _ = fs::remove_file(root.join("provenance.json"));
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
        // 2.8.2 の登録へ差し替える — ここが本検査の観測対象である。
        fs::write(root.join(".claude/settings.json"), settings_text()).unwrap();
        let hooks = root.join(".claude/hooks");
        fs::create_dir_all(&hooks).unwrap();
        for entry in fs::read_dir(&hooks).unwrap().flatten() {
            fs::remove_file(entry.path()).unwrap();
        }
        for name in registered_hook_names() {
            fs::write(hooks.join(format!("aidlc-{name}.ts")), "").unwrap();
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

    /// 作業ツリーの接続定義を重ねる (宣言と登録の突合を観測するとき)。
    fn with_binding_declaration(self) -> Self {
        let target = self.root().join("scripts/aidlc-selfhost");
        fs::create_dir_all(&target).unwrap();
        fs::copy(
            repo_root().join("scripts/aidlc-selfhost/hook-binding.json"),
            target.join("hook-binding.json"),
        )
        .unwrap();
        self
    }

    fn root(&self) -> PathBuf {
        self.temp.path().join("workspace")
    }

    fn doctor(&self) -> Output {
        let binary = self.temp.path().join("aidlc");
        if !binary.exists() {
            tool_link::link_tool(&binary).unwrap();
        }
        Command::new(binary)
            .arg("--doctor")
            .current_dir(self.root())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.temp.path().join("home"))
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.temp.path().join("bin").display()),
            )
            .env(
                "AIDLC_MANAGED_SETTINGS_PATH",
                self.root().join("aidlc/.absent-managed.json"),
            )
            .env("LANG", "C.UTF-8")
            .env("LC_ALL", "C.UTF-8")
            .env("TZ", "UTC")
            .output()
            .unwrap()
    }
}

fn report(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

/// 失敗行 (`✗` で始まる行) だけを取る。
fn failures(report: &str) -> Vec<String> {
    report
        .lines()
        .filter(|line| line.starts_with('✗'))
        .map(str::to_string)
        .collect()
}

/// D2.a — 2.8.2 の二段形登録に対し、フック実体の確認が全件合格する。
#[test]
fn the_two_stage_hook_registrations_satisfy_the_hook_contract() {
    let workspace = Workspace::with_two_stage_settings();
    let report = report(&workspace.doctor());
    let stale: Vec<String> = failures(&report)
        .into_iter()
        .filter(|line| line.contains("Hook contract"))
        .collect();
    assert!(
        stale.is_empty(),
        "2.8.2 の登録形を Hook contract が読めていない — {stale:?}\n{report}"
    );
    for name in registered_hook_names() {
        let present = report
            .lines()
            .any(|line| line.starts_with('✓') && line.contains(&name) && line.contains("present"));
        assert!(
            present,
            "{name}: 二段形の登録がフック実体の確認に現れていない\n{report}"
        );
    }
}

/// D2.e — 二段形の登録が、この build のフック面へ結ばれていると判定される。
#[test]
fn the_two_stage_hook_registrations_bind_to_this_build() {
    let workspace = Workspace::with_two_stage_settings();
    let report = report(&workspace.doctor());
    let stale: Vec<String> = failures(&report)
        .into_iter()
        .filter(|line| line.contains("Native hook bindings"))
        .collect();
    assert!(
        stale.is_empty(),
        "2.8.2 の登録形が native の接続として分類されていない — {stale:?}\n{report}"
    );
}

/// D2.e — 接続定義がある作業ツリーでも、宣言と二段形の登録が食い違わない。
#[test]
fn the_binding_declaration_agrees_with_the_two_stage_registrations() {
    let workspace = Workspace::with_two_stage_settings().with_binding_declaration();
    let report = report(&workspace.doctor());
    let stale: Vec<String> = failures(&report)
        .into_iter()
        .filter(|line| line.contains("Native hook bindings"))
        .collect();
    assert!(
        stale.is_empty(),
        "接続定義の宣言と 2.8.2 の登録が食い違っている — {stale:?}\n{report}"
    );
}

// ---------------------------------------------------------------------------
// 接続定義 (`hook-binding.json`) の形
// ---------------------------------------------------------------------------

fn binding_definition() -> serde_json::Value {
    let path = repo_root().join("scripts/aidlc-selfhost/hook-binding.json");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("接続定義が読めない ({}): {error}", path.display()));
    serde_json::from_str(&raw).expect("接続定義は JSON")
}

fn text(value: &serde_json::Value, key: &str) -> String {
    value[key]
        .as_str()
        .unwrap_or_else(|| panic!("{key} が無い: {value}"))
        .to_string()
}

/// 接続定義が綴る起動形は、配布が登録しているのと同じ二段形である。
#[test]
fn the_binding_definition_renders_the_two_stage_launch_form() {
    let definition = binding_definition();
    let template = text(&definition, "command_template");
    let rendered = template.replace("{hook}", "write-audit-log");
    let name = two_stage_hook_name(&rendered).unwrap_or_else(|| {
        panic!("接続定義の起動形が二段形 `aidlc engine hook <name>` ではない — {rendered}")
    });
    assert_eq!(name, "write-audit-log", "フック名が展開されていない");
    // 配布の登録と同じ綴りであること (登録の書換えを前提にしない)。
    let registered = registrations();
    let sample = registered
        .iter()
        .find(|registration| registration.hook == "write-audit-log")
        .expect("write-audit-log の登録");
    assert_eq!(sample.hook, name, "フック名の綴りが登録と違う");
}

/// 登録件数と名前数は別々に記録され、どちらも配布の現物と一致する。
#[test]
fn the_binding_definition_counts_registrations_apart_from_hook_names() {
    let definition = binding_definition();
    let registered = registrations();
    let names = registered_hook_names();
    assert!(
        registered.len() > names.len(),
        "配布の登録に重複が無い — この検査の前提が変わっている ({} 登録 / {} 名前)",
        registered.len(),
        names.len()
    );
    assert_eq!(
        definition["registration_count"].as_u64(),
        Some(registered.len() as u64),
        "登録件数が配布の現物と一致しない"
    );
    assert_eq!(
        definition["hook_name_count"].as_u64(),
        Some(names.len() as u64),
        "名前数が配布の現物と一致しない"
    );
    let listed: Vec<Registration> = {
        let mut listed: Vec<Registration> = definition["registrations"]
            .as_array()
            .expect("registrations が無い — 登録件数を名前数と区別して記録していない")
            .iter()
            .map(|entry| Registration {
                event: text(entry, "event"),
                matcher: entry["matcher"].as_str().unwrap_or_default().to_string(),
                hook: text(entry, "hook"),
            })
            .collect();
        listed.sort();
        listed
    };
    assert_eq!(
        listed, registered,
        "記録した登録 (イベント・matcher・フック名) が配布の現物とずれている"
    );
    // 名前の分類は 16 名前をちょうど覆う。
    let classified: BTreeSet<String> = definition["native_hooks"]
        .as_array()
        .expect("native_hooks")
        .iter()
        .map(|value| value.as_str().expect("native_hooks は文字列").to_string())
        .chain(
            definition["distributed_hooks"]
                .as_array()
                .expect("distributed_hooks")
                .iter()
                .map(|entry| text(entry, "name")),
        )
        .collect();
    assert_eq!(
        classified, names,
        "名前の分類が配布の登録と覆いきれていない"
    );
}

/// 各登録は、bugfix 1 周で発火するかどうかと、その判定の根拠を持つ。
#[test]
fn each_registration_records_whether_the_bugfix_loop_fires_it_and_on_what_basis() {
    let definition = binding_definition();
    let mut fires: BTreeMap<String, bool> = BTreeMap::new();
    for entry in definition["registrations"]
        .as_array()
        .expect("registrations")
    {
        let label = format!(
            "{} [{}] {}",
            text(entry, "event"),
            entry["matcher"].as_str().unwrap_or_default(),
            text(entry, "hook")
        );
        let basis = text(entry, "basis");
        assert!(
            ["matcher", "scope-stages", "undecided"].contains(&basis.as_str()),
            "{label}: 発火判定の根拠が未知の値である ({basis})"
        );
        assert!(
            !text(entry, "reason").trim().is_empty(),
            "{label}: 発火判定の根拠の記述が無い"
        );
        let entry_fires = entry["bugfix_fires"]
            .as_bool()
            .unwrap_or_else(|| panic!("{label}: bugfix_fires が真偽値でない"));
        // 発火しない側へ倒せるのは、根拠が実際に絞り込むときだけである。matcher が空の
        // 登録はそのイベントの全発生に一致するので、matcher を根拠に「発火しない」とは
        // 言えない。必要集合を狭める方向の誤りだけを止める (order.md §5.1 の 6)。
        if !entry_fires && basis == "matcher" {
            assert!(
                !entry["matcher"].as_str().unwrap_or_default().is_empty(),
                "{label}: matcher が空なのに matcher を根拠に発火しないと判定している"
            );
        }
        fires
            .entry(text(entry, "hook"))
            .and_modify(|value| *value = *value || entry_fires)
            .or_insert(entry_fires);
    }
    assert!(
        fires.values().any(|value| *value),
        "bugfix 1 周で発火する登録が 1 件も無い"
    );
}

/// マッチャ条件でもステージ構成でも決まらない登録は、発火する側へ倒す。
#[test]
fn an_undecided_registration_is_counted_as_firing() {
    let definition = binding_definition();
    for entry in definition["registrations"]
        .as_array()
        .expect("registrations")
    {
        if text(entry, "basis") != "undecided" {
            continue;
        }
        assert_eq!(
            entry["bugfix_fires"].as_bool(),
            Some(true),
            "{}: 判定不能な登録を発火しない側へ倒している",
            text(entry, "hook")
        );
    }
}
