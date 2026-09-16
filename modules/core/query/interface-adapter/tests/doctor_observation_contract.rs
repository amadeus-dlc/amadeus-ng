//! 自己診断の観測 DAO (`DoctorObservationDaoImpl`) の契約 — Bun / settings / フック /
//! heartbeat / workspace shell / 配布資産 / 状態版 / カーソル / ストア / 投影の
//! 「不在・不読・不正」を成功に丸めず、観測元を識別して返すこと。
//!
//! 観測先は一時ワークスペースに置く。配布シェルは固定コミット a277af21 の部分集合
//! (`tests/golden/selfhost-stage1/doctor-shell/`)、配布グラフ 3 入力は封印済みコーパス
//! (`tests/golden/upstream-a277af21/data/`) から写す。本リポジトリの実ファイルは変更しない。
// 契約テストは固定の添字参照と panic を検証の合図として使う (既存の契約テストと同じ許容)。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::unnecessary_wraps,
    clippy::missing_const_for_fn
)]

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use core_query_interface_adapter::{
    DoctorEnvironment, DoctorObservationDaoImpl, DoctorPaths, NativeDoctorFacts,
    StateVersionClassifier,
};
use core_query_use_case::orchestration::{
    DoctorObservationDao, DoctorObservationView, HookBindingDeclaration, HookBindingTarget,
    StateFileObservationView, StateVersionKindView, StateVersionView, StoreObservationView,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..")
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

/// U2 の分類器の代役 — `- **State Version**:` の綴りだけを見る (契約テストの固定値)。
struct FixtureClassifier;

impl StateVersionClassifier for FixtureClassifier {
    fn classify(&self, state_content: &str) -> StateVersionView {
        let token = state_content
            .lines()
            .find_map(|line| line.strip_prefix("- **State Version**:"))
            .map(str::trim);
        match token {
            Some("8") => StateVersionView::new(StateVersionKindView::Ok, None, None),
            Some("7") => StateVersionView::new(
                StateVersionKindView::Past,
                Some("7".into()),
                Some("past".into()),
            ),
            Some("9") => StateVersionView::new(
                StateVersionKindView::Future,
                Some("9".into()),
                Some("future".into()),
            ),
            _ => StateVersionView::new(
                StateVersionKindView::Unparseable,
                None,
                Some("unparseable".into()),
            ),
        }
    }
}

struct Workspace {
    temp: tempfile::TempDir,
}

impl Workspace {
    /// 配布シェル (a277af21 の部分集合) + 配布グラフ 3 入力 + settings.json が名指す
    /// フックの空ファイル + default space の memory 層。記録はまだ無い (初回状態)。
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
        Self { temp }
    }

    fn root(&self) -> PathBuf {
        self.temp.path().join("workspace")
    }

    fn intents(&self) -> PathBuf {
        self.root().join("aidlc/spaces/default/intents")
    }

    fn store(&self) -> PathBuf {
        self.intents().join(".aidlc-store.sqlite")
    }

    fn paths(&self, record: Option<&str>) -> DoctorPaths {
        DoctorPaths::new(
            self.root(),
            "default".to_string(),
            record.map(|name| self.intents().join(name)),
            self.store(),
        )
    }

    fn environment(&self) -> DoctorEnvironment {
        DoctorEnvironment::new(
            Some(OsString::from("/nonexistent-bin")),
            Some(self.temp.path().join("home")),
            Some(self.temp.path().join("managed.json")),
        )
    }

    fn facts() -> NativeDoctorFacts {
        NativeDoctorFacts::new(
            Ok(PathBuf::from("/opt/aidlc/target/release/aidlc")),
            Vec::new(),
            vec![
                "record-human-turn".to_string(),
                "write-audit-log".to_string(),
            ],
        )
    }

    fn observe(&self, record: Option<&str>) -> DoctorObservationView {
        self.observe_with(record, self.environment(), Self::facts())
    }

    fn observe_with(
        &self,
        record: Option<&str>,
        environment: DoctorEnvironment,
        facts: NativeDoctorFacts,
    ) -> DoctorObservationView {
        DoctorObservationDaoImpl::new(self.paths(record), environment, facts, FixtureClassifier)
            .find()
            .unwrap()
    }

    /// 記録ディレクトリと監査シャード・状態ファイルを置く (RMU を通さない固定内容)。
    fn add_record(&self, name: &str, state_version: &str, audit: &str) -> PathBuf {
        let record = self.intents().join(name);
        fs::create_dir_all(record.join("audit")).unwrap();
        fs::write(record.join("audit/host-clone.md"), audit).unwrap();
        fs::write(
            record.join("aidlc-state.md"),
            format!(
                "# AI-DLC State Tracking\n\n## Project Information\n- **State Version**: {state_version}\n\n## Stage Progress\n- [x] workspace-scaffold — EXECUTE\n- [x] workspace-detection — EXECUTE\n- [-] state-init — EXECUTE\n- [ ] intent-capture — SKIP: not needed\n"
            ),
        )
        .unwrap();
        record
    }
}

/// `settings.json` が名指す `aidlc-*.ts` (本家 doctor と同じ正規表現の写し)。
fn wired_hook_names(settings: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = settings;
    while let Some(start) = rest.find("aidlc-") {
        let tail = &rest[start..];
        let end = tail
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.'))
            .unwrap_or(tail.len());
        let token = &tail[..end];
        if let Some(stem) = token.strip_suffix(".ts") {
            let name = format!("{stem}.ts");
            if !names.contains(&name) {
                names.push(name);
            }
        }
        rest = &rest[start + 6..];
    }
    names.sort();
    names
}

const AUDIT_WITH_PROGRESS: &str = "## Workflow Started\n**Timestamp**: 2026-09-08T00:27:21Z\n**Event**: WORKFLOW_STARTED\n\n---\n\n## Stage Started\n**Timestamp**: 2026-09-08T00:27:21Z\n**Event**: STAGE_STARTED\n**Stage**: workspace-scaffold\n\n---\n\n## Stage Completed\n**Timestamp**: 2026-09-08T00:27:22Z\n**Event**: STAGE_COMPLETED\n**Stage**: workspace-scaffold\n\n---\n\n## Stage Completed\n**Timestamp**: 2026-09-08T00:27:22Z\n**Event**: STAGE_COMPLETED\n**Stage**: workspace-detection\n\n---\n";

#[test]
fn bun_is_found_on_path_or_under_home_and_absent_otherwise() {
    let workspace = Workspace::shipped();
    let bin = workspace.temp.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    fs::write(bin.join("bun"), "#!/bin/sh\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(bin.join("bun"), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let on_path = DoctorEnvironment::new(
        Some(OsString::from(format!(
            "/nonexistent-bin:{}",
            bin.display()
        ))),
        None,
        None,
    );
    assert!(
        workspace
            .observe_with(None, on_path, Workspace::facts())
            .bun_found()
    );

    let nowhere = DoctorEnvironment::new(
        Some(OsString::from("/nonexistent-bin")),
        Some(workspace.temp.path().join("home")),
        None,
    );
    assert!(
        !workspace
            .observe_with(None, nowhere, Workspace::facts())
            .bun_found()
    );

    let home = workspace.temp.path().join("home");
    fs::create_dir_all(home.join(".bun/bin")).unwrap();
    fs::write(home.join(".bun/bin/bun"), "").unwrap();
    let under_home =
        DoctorEnvironment::new(Some(OsString::from("/nonexistent-bin")), Some(home), None);
    assert!(
        workspace
            .observe_with(None, under_home, Workspace::facts())
            .bun_found()
    );
}

#[test]
fn the_binary_and_missing_entry_points_are_carried_verbatim() {
    let workspace = Workspace::shipped();
    let facts = NativeDoctorFacts::new(
        Err("current executable is not available".to_string()),
        vec!["aidlc-log answer".to_string()],
        Vec::new(),
    );
    let observed = workspace.observe_with(None, workspace.environment(), facts);
    assert_eq!(
        observed
            .entry_points()
            .binary()
            .as_ref()
            .unwrap_err()
            .cause(),
        "current executable is not available"
    );
    assert_eq!(
        observed.entry_points().missing_entry_points(),
        ["aidlc-log answer".to_string()]
    );
    let observed = workspace.observe(None);
    assert_eq!(
        observed.entry_points().binary().as_ref().unwrap(),
        "/opt/aidlc/target/release/aidlc"
    );
    assert!(observed.entry_points().missing_entry_points().is_empty());
}

#[test]
fn settings_absence_no_hooks_and_broken_json_are_distinguished() {
    let workspace = Workspace::shipped();
    let settings = workspace.root().join(".claude/settings.json");

    let shipped = workspace.observe(None);
    assert!(shipped.hook_wiring().settings_present());
    let wired = shipped.hook_wiring().wired_hooks().as_ref().unwrap();
    assert_eq!(
        wired.len(),
        17,
        "a277af21 の settings.json は 17 本を名指す"
    );
    assert_eq!(wired.first().unwrap().name(), "aidlc-continue-workflow.ts");
    assert_eq!(wired.last().unwrap().name(), "aidlc-write-audit-log.ts");
    assert!(wired.iter().all(|hook| hook.present()));
    assert!(shipped.hook_wiring().hooks_disabled_by().is_none());
    assert!(!shipped.hook_wiring().managed_hooks_only());
    let bindings = shipped.hook_wiring().bindings().as_ref().unwrap();
    assert!(
        bindings.len() >= 17,
        "hooks ブロックの登録数: {}",
        bindings.len()
    );
    assert!(
        bindings
            .iter()
            .all(|b| matches!(b.target(), HookBindingTarget::Distributed(_)))
    );

    fs::remove_file(
        workspace
            .root()
            .join(".claude/hooks/aidlc-record-human-turn.ts"),
    )
    .unwrap();
    let missing_hook = workspace.observe(None);
    let absent: Vec<_> = missing_hook
        .hook_wiring()
        .wired_hooks()
        .as_ref()
        .unwrap()
        .iter()
        .filter(|hook| !hook.present())
        .map(|hook| hook.name().to_string())
        .collect();
    assert_eq!(absent, ["aidlc-record-human-turn.ts".to_string()]);

    fs::write(&settings, "{}\n").unwrap();
    let no_hooks = workspace.observe(None);
    assert!(no_hooks.hook_wiring().settings_present());
    assert!(
        no_hooks
            .hook_wiring()
            .wired_hooks()
            .as_ref()
            .unwrap()
            .is_empty()
    );
    assert!(
        no_hooks
            .hook_wiring()
            .bindings()
            .as_ref()
            .unwrap()
            .is_empty()
    );

    fs::write(&settings, "{\"hooks\":").unwrap();
    let broken = workspace.observe(None);
    assert!(
        broken
            .hook_wiring()
            .wired_hooks()
            .as_ref()
            .unwrap()
            .is_empty()
    );
    assert!(
        broken.hook_wiring().bindings().is_err(),
        "JSON として読めない"
    );

    fs::remove_file(&settings).unwrap();
    let absent = workspace.observe(None);
    assert!(!absent.hook_wiring().settings_present());
    assert!(absent.hook_wiring().wired_hooks().is_err());
    assert!(absent.hook_wiring().bindings().is_err());
}

#[test]
fn disable_all_hooks_resolves_by_layer_precedence_and_managed_policy_is_read() {
    let workspace = Workspace::shipped();
    let claude = workspace.root().join(".claude");
    let settings = fs::read_to_string(claude.join("settings.json")).unwrap();

    fs::write(
        claude.join("settings.json"),
        settings.replacen("{\n", "{\n  \"disableAllHooks\": true,\n", 1),
    )
    .unwrap();
    assert_eq!(
        workspace.observe(None).hook_wiring().hooks_disabled_by(),
        Some(".claude/settings.json")
    );

    fs::write(
        claude.join("settings.local.json"),
        "{\"disableAllHooks\": false}",
    )
    .unwrap();
    assert_eq!(
        workspace.observe(None).hook_wiring().hooks_disabled_by(),
        None,
        "上位層の明示的な false が下位層の true に勝つ"
    );

    fs::write(
        claude.join("settings.local.json"),
        "{\"disableAllHooks\": true}",
    )
    .unwrap();
    assert_eq!(
        workspace.observe(None).hook_wiring().hooks_disabled_by(),
        Some(".claude/settings.local.json")
    );

    fs::write(
        claude.join("settings.local.json"),
        "{\"disableAllHooks\": \"yes\"}",
    )
    .unwrap();
    assert_eq!(
        workspace.observe(None).hook_wiring().hooks_disabled_by(),
        Some(".claude/settings.json"),
        "boolean でない値は層を決めない"
    );

    let managed = workspace.temp.path().join("managed.json");
    fs::write(
        &managed,
        "{\"disableAllHooks\": true, \"allowManagedHooksOnly\": true}\n",
    )
    .unwrap();
    let observed = workspace.observe(None);
    assert_eq!(
        observed.hook_wiring().hooks_disabled_by(),
        Some("enterprise managed settings")
    );
    assert!(observed.hook_wiring().managed_hooks_only());

    fs::write(&managed, "{\"allowManagedHooksOnly\": false}\n").unwrap();
    let fragments = workspace.temp.path().join("managed-settings.d");
    fs::create_dir_all(&fragments).unwrap();
    fs::write(
        fragments.join("10-policy.json"),
        "{\"allowManagedHooksOnly\": true}",
    )
    .unwrap();
    let observed = workspace.observe(None);
    assert!(
        observed.hook_wiring().managed_hooks_only(),
        "managed-settings.d の断片は後勝ち"
    );
    assert_eq!(
        observed.hook_wiring().hooks_disabled_by(),
        Some(".claude/settings.json"),
        "管理設定が disableAllHooks を語らなければ層の解決へ落ちる (local は boolean でない)"
    );
}

#[test]
fn hook_bindings_are_classified_by_their_target() {
    let workspace = Workspace::shipped();
    fs::write(
        workspace.root().join(".claude/settings.json"),
        r#"{
  "hooks": {
    "PostToolUse": [
      {"matcher": "Write|Edit", "hooks": [
        {"type": "command", "command": "bun \"$CLAUDE_PROJECT_DIR/.claude/hooks/aidlc-write-audit-log.ts\""},
        {"type": "command", "command": "\"$CLAUDE_PROJECT_DIR/target/release/aidlc\" hook record-human-turn"}
      ]}
    ],
    "Stop": [
      {"hooks": [{"type": "command", "command": "sh \"$CLAUDE_PROJECT_DIR/.claude/hooks/custom.sh\""}]}
    ]
  }
}"#,
    )
    .unwrap();
    let observed = workspace.observe(None);
    let bindings = observed.hook_wiring().bindings().as_ref().unwrap();
    assert_eq!(bindings.len(), 3);
    assert_eq!(bindings[0].event(), "PostToolUse");
    assert_eq!(bindings[0].matcher(), "Write|Edit");
    assert_eq!(
        bindings[0].target(),
        &HookBindingTarget::Distributed("write-audit-log".to_string())
    );
    assert_eq!(
        bindings[1].target(),
        &HookBindingTarget::Native("record-human-turn".to_string())
    );
    assert_eq!(bindings[2].event(), "Stop");
    assert_eq!(bindings[2].matcher(), "");
    assert_eq!(bindings[2].target(), &HookBindingTarget::Unknown);
    assert_eq!(
        observed.hook_wiring().native_hook_names(),
        [
            "record-human-turn".to_string(),
            "write-audit-log".to_string()
        ]
    );
}

/// aidlc 2.8.2 の登録形 `aidlc engine hook <name>` も、この build のフック面として識別する。
///
/// 2.8.2 の配布 `.claude/settings.json` はフックを二段形で登録し、`aidlc` を `PATH` から
/// 解決する。一段形 (`aidlc hook <name>`) と配布 `.ts` の識別はそのまま保つ — 2.8.2 の
/// 識別を**加えて**、どちらの形も取り違えないことをここで固定する。
#[test]
fn the_two_stage_registration_form_is_classified_as_this_builds_hook_face() {
    let workspace = Workspace::shipped();
    fs::write(
        workspace.root().join(".claude/settings.json"),
        r#"{
  "hooks": {
    "PostToolUse": [
      {"matcher": "Write|Edit", "hooks": [
        {"type": "command", "command": "aidlc engine hook write-audit-log"},
        {"type": "command", "command": "bun \"$CLAUDE_PROJECT_DIR/.claude/hooks/aidlc-run-sensors.ts\""}
      ]}
    ],
    "UserPromptSubmit": [
      {"matcher": "", "hooks": [{"type": "command", "command": "aidlc engine hook record-human-turn"}]}
    ]
  }
}"#,
    )
    .unwrap();
    let hooks = workspace.root().join(".claude/hooks");
    for entry in fs::read_dir(&hooks).unwrap().flatten() {
        fs::remove_file(entry.path()).unwrap();
    }
    fs::write(hooks.join("aidlc-write-audit-log.ts"), "").unwrap();
    fs::write(hooks.join("aidlc-run-sensors.ts"), "").unwrap();

    let observed = workspace.observe(None);
    let bindings = observed.hook_wiring().bindings().as_ref().unwrap();
    assert_eq!(bindings.len(), 3);
    assert_eq!(
        bindings[0].target(),
        &HookBindingTarget::Native("write-audit-log".to_string()),
        "二段形の登録がこの build のフック面として識別されていない"
    );
    assert_eq!(
        bindings[1].target(),
        &HookBindingTarget::Distributed("run-sensors".to_string()),
        "配布 .ts の識別が壊れている"
    );
    assert_eq!(
        bindings[2].target(),
        &HookBindingTarget::Native("record-human-turn".to_string()),
        "二段形の登録がこの build のフック面として識別されていない"
    );

    // D2.a の材料 — 二段形で登録したフックも、実体の有無つきで観測される。
    let wired = observed.hook_wiring().wired_hooks().as_ref().unwrap();
    let observed_names: Vec<String> = wired.iter().map(|hook| hook.name().to_string()).collect();
    for (name, present) in [
        ("write-audit-log", true),
        ("run-sensors", true),
        ("record-human-turn", false),
    ] {
        let hook = wired
            .iter()
            .find(|hook| hook.name().contains(name))
            .unwrap_or_else(|| {
                panic!("{name}: 二段形の登録がフックの観測に現れていない — {observed_names:?}")
            });
        assert_eq!(
            hook.present(),
            present,
            "{name}: フック実体の有無が観測と食い違う"
        );
    }
}

#[test]
fn heartbeat_observation_reads_health_dir_audit_and_state_together() {
    let workspace = Workspace::shipped();
    let cold = workspace.observe(None);
    assert!(!cold.heartbeat().health_dir_exists());
    assert!(!cold.heartbeat().has_heartbeat_files());
    assert_eq!(cold.heartbeat().progressed_stage_count(), 0);
    assert!(!cold.heartbeat().stage_started());
    assert!(cold.heartbeat().newest_progress().is_none());

    let record = workspace.add_record("260908-doctor", "8", AUDIT_WITH_PROGRESS);
    let progressed = workspace.observe(Some("260908-doctor"));
    assert_eq!(
        progressed.heartbeat().progressed_stage_count(),
        3,
        "状態の非 pending 3 行 > 監査の 2 slug"
    );
    assert!(progressed.heartbeat().stage_started());
    let newest = progressed.heartbeat().newest_progress().unwrap();
    assert_eq!(newest.raw(), "2026-09-08T00:27:22Z");
    assert_eq!(newest.millis(), Some(1_788_827_242_000));
    assert!(!progressed.heartbeat().health_dir_exists());

    let health = record.join(".aidlc-hooks-health");
    fs::create_dir_all(&health).unwrap();
    let empty = workspace.observe(Some("260908-doctor"));
    assert!(empty.heartbeat().health_dir_exists());
    assert!(!empty.heartbeat().has_heartbeat_files());

    fs::write(health.join("fixture.last"), "2026-09-08T00:22:22.000Z").unwrap();
    fs::write(health.join("fixture.drops"), "ignored\n").unwrap();
    let fired = workspace.observe(Some("260908-doctor"));
    assert!(fired.heartbeat().has_heartbeat_files());
    let entries = fired.heartbeat().entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].hook(), "fixture");
    assert_eq!(entries[0].timestamp().raw(), "2026-09-08T00:22:22.000Z");
    assert_eq!(entries[0].timestamp().millis(), Some(1_788_826_942_000));

    fs::remove_file(health.join("fixture.last")).unwrap();
    fs::create_dir_all(health.join("unreadable.last")).unwrap();
    let unreadable = workspace.observe(Some("260908-doctor"));
    assert!(unreadable.heartbeat().has_heartbeat_files());
    assert!(
        unreadable.heartbeat().entries().is_empty(),
        "読めない heartbeat は項目にならないが、存在は残る"
    );
}

#[test]
fn shell_and_definition_assets_keep_each_failure_apart() {
    let workspace = Workspace::shipped();
    let observed = workspace.observe(None);
    assert!(observed.shell().harness_dir_exists());
    assert!(observed.shell().memory_dir_exists());
    let definition = observed.definition();
    let graph = definition.graph().as_ref().unwrap();
    assert_eq!(graph.len(), 33);
    let code_generation = graph
        .iter()
        .find(|stage| stage.slug() == "code-generation")
        .unwrap();
    assert_eq!(code_generation.phase(), "construction");
    assert_eq!(code_generation.number(), "3.5");
    assert!(code_generation.enabled());
    assert!(
        code_generation
            .requires_stage()
            .contains(&"units-generation".to_string())
    );
    assert!(
        code_generation
            .artifacts()
            .consumes()
            .iter()
            .any(|consume| consume.artifact() == "unit-of-work" && consume.required())
    );
    let grid = definition.scope_grid().as_ref().unwrap();
    assert_eq!(grid.len(), 11);
    let bugfix = grid.iter().find(|entry| entry.scope() == "bugfix").unwrap();
    assert!(
        bugfix
            .stages()
            .contains(&("code-generation".to_string(), "EXECUTE".to_string()))
    );
    assert_eq!(
        definition.scope_names().as_ref().unwrap(),
        &["bugfix".to_string(), "feature".to_string()]
    );
    let files = definition.stage_files().as_ref().unwrap();
    assert_eq!(files.len(), 33);
    assert!(files.iter().all(|file| file.content().is_ok()));
    assert_eq!(files.first().unwrap().phase(), "initialization");
    let agents = definition.agents().as_ref().unwrap();
    assert_eq!(agents.len(), 14);
    assert_eq!(agents.first().unwrap(), "aidlc-architect-agent");

    fs::remove_dir_all(workspace.root().join("aidlc/spaces/default/memory")).unwrap();
    fs::write(
        workspace.root().join(".claude/tools/data/stage-graph.json"),
        "[{\"slug\":",
    )
    .unwrap();
    fs::remove_file(workspace.root().join(".claude/tools/data/scope-grid.json")).unwrap();
    let stage = workspace
        .root()
        .join(".claude/aidlc-common/stages/construction/code-generation.md");
    fs::remove_file(&stage).unwrap();
    fs::create_dir_all(&stage).unwrap();
    fs::write(
        workspace
            .root()
            .join(".claude/agents/aidlc-broken-agent.md"),
        "no frontmatter\n",
    )
    .unwrap();
    let broken = workspace.observe(None);
    assert!(!broken.shell().memory_dir_exists());
    assert!(broken.definition().graph().is_err());
    assert!(broken.definition().scope_grid().is_err());
    let unreadable = broken
        .definition()
        .stage_files()
        .as_ref()
        .unwrap()
        .iter()
        .find(|file| file.slug() == "code-generation")
        .unwrap();
    assert!(unreadable.content().is_err());
    assert!(broken.definition().agents().is_err());
}

#[test]
fn record_observation_distinguishes_cold_partial_and_initialized_workspaces() {
    let workspace = Workspace::shipped();
    assert!(
        workspace.observe(None).record().is_none(),
        "記録もストアも無い初回状態"
    );

    let record = workspace.add_record("260908-doctor", "7", AUDIT_WITH_PROGRESS);
    let unselected = workspace.observe(None);
    let observed = unselected.record().unwrap();
    assert_eq!(observed.location().records(), ["260908-doctor".to_string()]);
    assert_eq!(observed.location().selected(), None);
    assert_eq!(
        observed.location().intents_relative(),
        "aidlc/spaces/default/intents"
    );
    assert_eq!(
        observed.location().store_relative(),
        "aidlc/spaces/default/intents/.aidlc-store.sqlite"
    );
    assert_eq!(observed.store(), &StoreObservationView::Absent);
    assert!(observed.projection().is_err());

    let selected = workspace.observe(Some("260908-doctor"));
    let observed = selected.record().unwrap();
    assert_eq!(observed.location().selected(), Some("260908-doctor"));
    match observed.state() {
        StateFileObservationView::Classified(version) => {
            assert_eq!(version.kind(), StateVersionKindView::Past);
            assert_eq!(version.version(), Some("7"));
        }
        other => panic!("分類済みのはず: {other:?}"),
    }
    assert_eq!(observed.cursor().as_ref().unwrap(), &None);
    assert_eq!(
        observed.registry_directory().as_ref().unwrap(),
        &None,
        "カーソルが無ければ登録簿を引く鍵が無い"
    );

    fs::write(record.join(".aidlc-execution"), "not-a-cursor\n").unwrap();
    let malformed = workspace.observe(Some("260908-doctor"));
    assert!(malformed.record().unwrap().cursor().is_err());

    fs::write(
        record.join(".aidlc-execution"),
        "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000\n01a02785-1bd8-76eb-aeea-5aa303ebd5b6\n",
    )
    .unwrap();
    let unregistered = workspace.observe(Some("260908-doctor"));
    assert!(
        unregistered.record().unwrap().registry_directory().is_err(),
        "intents.json が無い"
    );
    fs::write(
        workspace.intents().join("intents.json"),
        "[{\"uuid\":\"01a02785-1bd8-76eb-aeea-5aa303ebd5b6\",\"dirName\":\"260908-doctor\"}]",
    )
    .unwrap();
    fs::remove_file(record.join("aidlc-state.md")).unwrap();
    fs::create_dir_all(workspace.store()).unwrap();
    let partial = workspace.observe(Some("260908-doctor"));
    let observed = partial.record().unwrap();
    assert_eq!(observed.state(), &StateFileObservationView::Absent);
    let cursor = observed.cursor().as_ref().unwrap().as_ref().unwrap();
    assert_eq!(
        cursor.execution_id(),
        "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000"
    );
    assert_eq!(cursor.intent_id(), "01a02785-1bd8-76eb-aeea-5aa303ebd5b6");
    assert_eq!(
        observed.registry_directory().as_ref().unwrap().as_deref(),
        Some("260908-doctor")
    );
    assert!(
        matches!(observed.store(), StoreObservationView::Unreadable(_)),
        "ディレクトリはストアとして開けない: {:?}",
        observed.store()
    );

    fs::remove_dir_all(workspace.store()).unwrap();
    fs::create_dir_all(record.join("aidlc-state.md")).unwrap();
    let db = rusqlite::Connection::open(workspace.store()).unwrap();
    db.execute_batch(
        "CREATE TABLE journal (pkey TEXT NOT NULL, skey TEXT NOT NULL, aid TEXT NOT NULL, seq_nr INTEGER NOT NULL, payload BLOB NOT NULL, occurred_at INTEGER NOT NULL, manifest TEXT NOT NULL DEFAULT '', PRIMARY KEY (pkey, skey));
         CREATE TABLE snapshot (pkey TEXT NOT NULL, skey TEXT NOT NULL, aid TEXT NOT NULL, seq_nr INTEGER NOT NULL, payload BLOB NOT NULL, PRIMARY KEY (pkey, skey));
         CREATE TABLE amadeus_projection_checkpoint (projection TEXT PRIMARY KEY, last_global_seq INTEGER NOT NULL, anchor_aid TEXT, anchor_seq_nr INTEGER);
         CREATE TABLE read_execution (id TEXT PRIMARY KEY, intent_id TEXT NOT NULL);
         CREATE TABLE amadeus_publication (projection TEXT PRIMARY KEY, committed INTEGER NOT NULL);
         PRAGMA user_version = 6;
         INSERT INTO journal VALUES ('p1','s1','0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000',1,x'00',0,'intent-execution-event/1');
         INSERT INTO journal VALUES ('p2','s2','other-aggregate',1,x'00',0,'hook-health-event/1');
         INSERT INTO journal VALUES ('p3','s3','0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000',2,x'00',0,'intent-execution-event/1');
         INSERT INTO amadeus_projection_checkpoint VALUES ('orchestration-0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000', 2, NULL, NULL);
         INSERT INTO read_execution VALUES ('0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000','01a02785-1bd8-76eb-aeea-5aa303ebd5b6');
         INSERT INTO amadeus_publication VALUES ('orchestration-0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000', 0);",
    )
    .unwrap();
    drop(db);
    let initialized = workspace.observe(Some("260908-doctor"));
    let observed = initialized.record().unwrap();
    assert!(
        matches!(observed.state(), StateFileObservationView::Unreadable(_)),
        "ディレクトリになった状態ファイルは不読: {:?}",
        observed.state()
    );
    let StoreObservationView::Opened(schema) = observed.store() else {
        panic!("開けるはず: {:?}", observed.store());
    };
    assert_eq!(schema.schema_version(), 6);
    assert!(schema.tables().contains(&"journal".to_string()));
    assert!(schema.tables().contains(&"read_execution".to_string()));
    let projection = observed.projection().as_ref().unwrap();
    assert_eq!(
        projection.execution_intent_id(),
        Some("01a02785-1bd8-76eb-aeea-5aa303ebd5b6")
    );
    assert_eq!(projection.checkpoint(), Some(2));
    assert_eq!(projection.latest_execution_event(), Some(3));
    assert_eq!(projection.pending_publications(), 1);
    assert_eq!(projection.audit_shard_count(), 1);
    assert!(
        fs::read_dir(workspace.intents())
            .unwrap()
            .flatten()
            .all(|entry| entry.file_name() != "store.sqlite3"),
        "観測は別名のストアを作らない"
    );
}

/// 接続定義は作業ツリーから読む — 無い / 読めた / 壊れている の 3 態を取り違えない。
///
/// D2.e はこの宣言を正として登録と照合する (U4 の接続定義)。定義の無い作業ツリー
/// (配布そのまま) を「壊れている」と読み替えないことが、本家対応行の終了コードを守る。
#[test]
fn the_repository_binding_declaration_is_read_from_the_workspace() {
    let workspace = Workspace::shipped();
    assert_eq!(
        workspace.observe(None).hook_wiring().declaration(),
        &HookBindingDeclaration::Absent,
        "定義の無い作業ツリーでは Absent"
    );

    let declaration = workspace.root().join("scripts/aidlc-selfhost");
    fs::create_dir_all(&declaration).unwrap();
    fs::write(
        declaration.join("hook-binding.json"),
        r#"{"native_hooks":["write-audit-log"],"distributed_hooks":[{"name":"run-sensors"}]}"#,
    )
    .unwrap();
    assert_eq!(
        workspace.observe(None).hook_wiring().declaration(),
        &HookBindingDeclaration::Declared {
            native: vec!["write-audit-log".to_string()],
            distributed: vec!["run-sensors".to_string()],
        },
        "文字列とオブジェクトの両方の綴りから名前を取る"
    );

    fs::write(declaration.join("hook-binding.json"), "{").unwrap();
    let observed = workspace.observe(None);
    let HookBindingDeclaration::Unreadable(cause) = observed.hook_wiring().declaration() else {
        panic!(
            "壊れた定義は Unreadable: {:?}",
            observed.hook_wiring().declaration()
        );
    };
    assert!(cause.cause().starts_with("invalid JSON: "), "{cause}");

    fs::write(
        declaration.join("hook-binding.json"),
        r#"{"native_hooks":"nope","distributed_hooks":[]}"#,
    )
    .unwrap();
    assert!(
        matches!(
            workspace.observe(None).hook_wiring().declaration(),
            HookBindingDeclaration::Unreadable(_)
        ),
        "名前の並びでない宣言を空の宣言へ丸めない"
    );
}
