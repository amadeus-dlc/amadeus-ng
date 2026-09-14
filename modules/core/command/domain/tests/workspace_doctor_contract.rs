//! 自己診断の判断 (`WorkspaceDoctor` / `DoctorChecks::evaluate`) の契約 — D1.a〜D5.b の各行の
//! 成否・ラベル・原因、初回状態の非適用、advisory を失敗にしないこと、対象外行を出さないこと、
//! 集計と終了コード。
//!
//! 観測は手で組む (値オブジェクト)。本家対応行のラベル・fix は固定コミット a277af21 の
//! `aidlc-utility.ts handleDoctor` の分岐の綴り、独自行は固定ラベル + ` — <原因>` である。
//! 判断はコマンド側の集約が所有する (オーナー裁定 2026-09-12) — 前担当がクエリ側ユースケースに
//! 置いていた同名の契約 (`doctor_report_contract`) をここへ移した。
// 契約テストは固定の添字参照と panic を検証の合図として使う (既存の契約テストと同じ許容)。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::unnecessary_wraps,
    clippy::missing_const_for_fn
)]

use chrono::{DateTime, Utc};
use core_command_domain::workspace::{
    ConsumedArtifact, DefinitionAssets, DoctorCheck, DoctorCheckId, DoctorChecks,
    DoctorObservation, ExecutionCursorObservation, GraphStage, HeartbeatEntry,
    HeartbeatObservation, HookBinding, HookBindingDeclaration, HookBindingTarget, HookHealthTarget,
    HookWiring, NativeEntryPoints, ObservationFailure, ObservedTimestamp, ProjectionObservation,
    RecordLocation, RecordObservation, ScopeGridEntry, SpaceName, StageArtifacts, StageFile,
    StateFileObservation, StateVersionClassification, StateVersionKind, StateVersionObservation,
    StoreObservation, StoreSchema, WiredHook, WorkspaceDoctor, WorkspaceShell,
};
use core_infrastructure::collections::FirstClassCollection as _;

fn at() -> DateTime<Utc> {
    "2026-09-12T00:00:00Z".parse().unwrap()
}

fn target() -> HookHealthTarget {
    HookHealthTarget::new(SpaceName::parse("default").unwrap(), None)
}

/// 集約の誕生 (最初の診断) で評価した行。
fn evaluate(observation: &DoctorObservation) -> DoctorChecks {
    let (doctor, event) = WorkspaceDoctor::start(target(), observation, at()).unwrap();
    assert_eq!(
        doctor.checks(),
        event.checks(),
        "集約とイベントは同じ行を持つ"
    );
    doctor.checks().clone()
}

fn rows(checks: &DoctorChecks) -> Vec<String> {
    checks
        .as_slice()
        .iter()
        .map(|check| match check.fix() {
            Some(fix) if !check.is_passed() => format!("✗ {} — {fix}", check.label()),
            _ => format!(
                "{} {}",
                if check.is_passed() { "✓" } else { "✗" },
                check.label()
            ),
        })
        .collect()
}

fn find(checks: &DoctorChecks, id: DoctorCheckId) -> Vec<DoctorCheck> {
    checks.filter(|check| check.id() == id).as_slice().to_vec()
}

/// runtime と同じ分類器で版を判定する (空文字なら版の行が無い状態ファイル)。
fn classified(token: &str) -> StateVersionKind {
    let content = if token.is_empty() {
        "# AI-DLC State Tracking\n\n## Project Information\n".to_string()
    } else {
        format!("# AI-DLC State Tracking\n\n## Project Information\n- **State Version**: {token}\n")
    };
    StateVersionClassification::classify(&content).kind()
}

const HOOK_EXECUTION_RECOVERY: &str = "1. Run /hooks to check hook approval and policy state. 2. If hooks need approval, approve them and fully restart the CLI; approval does not take effect until a full restart. 3. If /hooks says hooks are restricted by policy, only your Claude Code administrator can lift allowManagedHooksOnly in managed-settings.json. Until then, for an attended session, launch the CLI with AIDLC_SKIP_HUMAN_PRESENCE_GUARD=1 and AIDLC_SKIP_SUMMARY_CONFIRMATION_GUARD=1";

fn failure(cause: &str) -> ObservationFailure {
    ObservationFailure::new(cause.to_string())
}

fn stage(
    slug: &str,
    number: &str,
    requires: &[&str],
    produces: &[&str],
    consumes: &[(&str, bool)],
) -> GraphStage {
    GraphStage::new(
        slug.to_string(),
        "inception".to_string(),
        number.to_string(),
        true,
        requires.iter().map(|s| (*s).to_string()).collect(),
        StageArtifacts::new(
            produces.iter().map(|s| (*s).to_string()).collect(),
            Vec::new(),
            consumes
                .iter()
                .map(|(artifact, required)| {
                    ConsumedArtifact::new((*artifact).to_string(), *required, None)
                })
                .collect(),
        ),
    )
}

/// 3 ステージの小さな配布束: a は成果物 `alpha` を作り、b は `alpha` を要し、c は `alpha` を
/// 任意で読む。bugfix は b と c、feature は全部を EXECUTE。
fn tiny_graph() -> Vec<GraphStage> {
    vec![
        stage("a", "2.1", &[], &["alpha"], &[]),
        stage("b", "2.2", &["a"], &["beta"], &[("alpha", true)]),
        stage("c", "2.3", &["b"], &[], &[("alpha", false)]),
    ]
}

fn grid(bugfix: &[&str], feature: &[&str]) -> Vec<ScopeGridEntry> {
    let entry = |scope: &str, execute: &[&str]| {
        ScopeGridEntry::new(
            scope.to_string(),
            ["a", "b", "c"]
                .iter()
                .map(|slug| {
                    (
                        (*slug).to_string(),
                        if execute.contains(slug) {
                            "EXECUTE"
                        } else {
                            "SKIP"
                        }
                        .to_string(),
                    )
                })
                .collect(),
        )
    };
    vec![entry("bugfix", bugfix), entry("feature", feature)]
}

fn stage_file(slug: &str, mode: &str, lead: &str) -> StageFile {
    StageFile::new(
        "inception".to_string(),
        slug.to_string(),
        Ok(format!(
            "---\nslug: {slug}\nphase: inception\nexecution: ALWAYS\ncondition: Always\nlead_agent: {lead}\nsupport_agents: []\nmode: {mode}\nproduces:\n  - alpha\nconsumes: []\nrequires_stage: []\ninputs: none\noutputs: none\n---\n# Body\n"
        )),
    )
}

fn tiny_definition() -> DefinitionAssets {
    DefinitionAssets::new(
        Ok(tiny_graph()),
        Ok(grid(&["b", "c"], &["a", "b", "c"])),
        Ok(vec!["bugfix".to_string(), "feature".to_string()]),
        Ok(vec![
            stage_file("a", "inline", "aidlc-product-agent"),
            stage_file("b", "subagent", "aidlc-developer-agent"),
            stage_file("c", "inline", "aidlc-product-agent"),
        ]),
        Ok(vec![
            "aidlc-developer-agent".to_string(),
            "aidlc-product-agent".to_string(),
        ]),
    )
}

fn wired(names: &[&str]) -> Result<Vec<WiredHook>, ObservationFailure> {
    Ok(names
        .iter()
        .map(|name| WiredHook::new((*name).to_string(), true))
        .collect())
}

fn distributed(name: &str) -> HookBinding {
    HookBinding::new(
        "PostToolUse".to_string(),
        "Write|Edit".to_string(),
        format!("bun \"$CLAUDE_PROJECT_DIR/.claude/hooks/aidlc-{name}.ts\""),
        HookBindingTarget::Distributed(name.to_string()),
    )
}

fn healthy_wiring() -> HookWiring {
    HookWiring::new(
        true,
        wired(&["aidlc-record-human-turn.ts", "aidlc-write-audit-log.ts"]),
        None,
        false,
        Ok(vec![
            distributed("record-human-turn"),
            distributed("write-audit-log"),
        ]),
        vec![
            "record-human-turn".to_string(),
            "write-audit-log".to_string(),
        ],
        core_command_domain::workspace::HookBindingDeclaration::Absent,
    )
}

fn not_yet_fired() -> HeartbeatObservation {
    HeartbeatObservation::new(false, false, Vec::new(), 0, false, None)
}

fn cold() -> DoctorObservation {
    DoctorObservation::new(
        true,
        NativeEntryPoints::new(
            Ok("/opt/aidlc/target/release/aidlc".to_string()),
            Vec::new(),
        ),
        healthy_wiring(),
        not_yet_fired(),
        WorkspaceShell::new(true, true),
        tiny_definition(),
        None,
    )
}

fn with_definition(definition: DefinitionAssets) -> DoctorObservation {
    DoctorObservation::new(
        true,
        NativeEntryPoints::new(Ok("/opt/aidlc".to_string()), Vec::new()),
        healthy_wiring(),
        not_yet_fired(),
        WorkspaceShell::new(true, true),
        definition,
        None,
    )
}

fn with_wiring(wiring: HookWiring) -> DoctorObservation {
    DoctorObservation::new(
        true,
        NativeEntryPoints::new(Ok("/opt/aidlc".to_string()), Vec::new()),
        wiring,
        not_yet_fired(),
        WorkspaceShell::new(true, true),
        tiny_definition(),
        None,
    )
}

fn with_heartbeat(heartbeat: HeartbeatObservation) -> DoctorObservation {
    DoctorObservation::new(
        true,
        NativeEntryPoints::new(Ok("/opt/aidlc".to_string()), Vec::new()),
        healthy_wiring(),
        heartbeat,
        WorkspaceShell::new(true, true),
        tiny_definition(),
        None,
    )
}

const EXECUTION: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";
const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

fn location(selected: Option<&str>) -> RecordLocation {
    RecordLocation::new(
        "aidlc/spaces/default/intents".to_string(),
        "aidlc/spaces/default/intents/.aidlc-store.sqlite".to_string(),
        vec!["260908-doctor".to_string()],
        selected.map(str::to_string),
        None,
    )
}

fn opened() -> StoreObservation {
    StoreObservation::Opened(StoreSchema::new(
        vec![
            "amadeus_projection_checkpoint".to_string(),
            "journal".to_string(),
            "read_execution".to_string(),
        ],
        6,
    ))
}

fn consistent() -> ProjectionObservation {
    ProjectionObservation::new(Some(INTENT.to_string()), Some(7), Some(7), 0, 1)
}

fn healthy_record() -> RecordObservation {
    RecordObservation::new(
        location(Some("260908-doctor")),
        StateFileObservation::Classified(StateVersionObservation::new(classified("8"), None)),
        Ok(Some(ExecutionCursorObservation::new(
            EXECUTION.to_string(),
            INTENT.to_string(),
        ))),
        Ok(Some("260908-doctor".to_string())),
        opened(),
        Ok(consistent()),
    )
}

fn with_record(record: RecordObservation) -> DoctorObservation {
    DoctorObservation::new(
        true,
        NativeEntryPoints::new(Ok("/opt/aidlc".to_string()), Vec::new()),
        healthy_wiring(),
        HeartbeatObservation::new(
            true,
            true,
            vec![HeartbeatEntry::new(
                "fixture".to_string(),
                ObservedTimestamp::new(
                    "2026-09-08T00:22:22.000Z".to_string(),
                    Some(1_788_826_942_000),
                ),
            )],
            4,
            true,
            Some(ObservedTimestamp::new(
                "2026-09-08T00:27:22Z".to_string(),
                Some(1_788_827_242_000),
            )),
        ),
        WorkspaceShell::new(true, true),
        tiny_definition(),
        Some(record),
    )
}

#[test]
fn a_cold_workspace_reports_the_table_rows_in_order_and_omits_record_checks() {
    let report = evaluate(&cold());
    assert_eq!(
        rows(&report),
        [
            "✓ bun installed (required for CLI tools and hooks)",
            "✓ Native engine entry points",
            "✓ aidlc-record-human-turn.ts present",
            "✓ aidlc-write-audit-log.ts present",
            "✓ Hooks enabled (resolved disableAllHooks is not true)",
            "✓ settings.json present",
            "✓ Native hook bindings",
            "✓ Hook heartbeats: not yet fired (first workflow stage will populate)",
            "✓ workspace shell ready (.claude/ + aidlc/spaces/default/memory/)",
            "✓ Scope validation: 2 scopes valid (1 advisories)",
            "✓ Cycle detection: 0 cycles",
            "✓ Orphan stage files: 3 graph entries all have files",
            "✓ Schema validation: 3/3 stages validated",
            "✓ Graph references: 2 artifacts + edges resolved",
        ]
    );
    assert_eq!(report.passed(), 14);
    assert_eq!(report.failed(), 0);
    assert_eq!(report.exit_code(), 0);
    for id in [
        DoctorCheckId::D4a,
        DoctorCheckId::D4b,
        DoctorCheckId::D4c,
        DoctorCheckId::D5a,
        DoctorCheckId::D5b,
    ] {
        assert!(
            find(&report, id).is_empty(),
            "初回状態では {id:?} を出さない"
        );
    }
    assert!(
        report
            .as_slice()
            .iter()
            .all(|check| !check.label().starts_with("Uncompiled stage files")),
        "対象外の advisory 行を出さない"
    );
}

#[test]
fn d1_fails_when_bun_is_absent_or_the_binary_and_entry_points_cannot_be_confirmed() {
    let observed = DoctorObservation::new(
        false,
        NativeEntryPoints::new(
            Err(failure("current executable unavailable")),
            vec!["aidlc-log answer".to_string(), "aidlc park".to_string()],
        ),
        healthy_wiring(),
        not_yet_fired(),
        WorkspaceShell::new(true, true),
        tiny_definition(),
        None,
    );
    let report = evaluate(&observed);
    let bun = find(&report, DoctorCheckId::D1a);
    assert_eq!(bun.len(), 1);
    assert!(!bun[0].is_passed());
    assert_eq!(
        bun[0].label(),
        "bun installed (required for CLI tools and hooks)"
    );
    assert_eq!(
        bun[0].fix(),
        Some("install via `curl -fsSL https://bun.sh/install | bash`")
    );
    let native = find(&report, DoctorCheckId::D1b);
    assert_eq!(native.len(), 1);
    assert_eq!(native[0].label(), "Native engine entry points");
    assert_eq!(
        native[0].fix(),
        Some(
            "current executable: unreadable (current executable unavailable); aidlc: missing entry points (aidlc-log answer, aidlc park)"
        )
    );
    assert_eq!(report.failed(), 2);
    assert_eq!(report.exit_code(), 1);

    let only_missing = DoctorObservation::new(
        true,
        NativeEntryPoints::new(Ok("/opt/aidlc".to_string()), vec!["aidlc park".to_string()]),
        healthy_wiring(),
        not_yet_fired(),
        WorkspaceShell::new(true, true),
        tiny_definition(),
        None,
    );
    let report = evaluate(&only_missing);
    assert_eq!(
        find(&report, DoctorCheckId::D1b)[0].fix(),
        Some("aidlc: missing entry points (aidlc park)")
    );
}

#[test]
fn hook_contract_rows_follow_the_upstream_branches() {
    let unreadable = with_wiring(HookWiring::new(
        false,
        Err(failure("No such file or directory")),
        None,
        false,
        Err(failure("No such file or directory")),
        Vec::new(),
        core_command_domain::workspace::HookBindingDeclaration::Absent,
    ));
    let report = evaluate(&unreadable);
    assert_eq!(
        rows(&report)[2..7],
        [
            "✗ Hook contract: settings.json unreadable — cannot verify wired hooks — restore .claude/settings.json (copy from `dist/claude/.claude/settings.json`)".to_string(),
            "✓ Hooks enabled (resolved disableAllHooks is not true)".to_string(),
            "✗ settings.json present — copy from `dist/claude/.claude/settings.json`".to_string(),
            "✗ Native hook bindings — .claude/settings.json: missing".to_string(),
            "✓ Hook heartbeats: not yet fired (first workflow stage will populate)".to_string(),
        ]
    );
    assert_eq!(report.failed(), 3);

    let no_hooks = with_wiring(HookWiring::new(
        true,
        Ok(Vec::new()),
        None,
        false,
        Ok(Vec::new()),
        Vec::new(),
        core_command_domain::workspace::HookBindingDeclaration::Absent,
    ));
    let report = evaluate(&no_hooks);
    assert_eq!(
        rows(&report)[2],
        "✗ Hook contract: settings.json wires no aidlc-*.ts hooks — restore the hooks block in .claude/settings.json (copy from `dist/claude/.claude/settings.json`)"
    );
    assert_eq!(
        rows(&report)[5],
        "✗ Native hook bindings — .claude/settings.json: no hook bindings"
    );

    let missing_hook = with_wiring(HookWiring::new(
        true,
        Ok(vec![
            WiredHook::new("aidlc-record-human-turn.ts".to_string(), false),
            WiredHook::new("aidlc-write-audit-log.ts".to_string(), true),
        ]),
        Some(".claude/settings.json".to_string()),
        true,
        Ok(vec![
            distributed("record-human-turn"),
            distributed("write-audit-log"),
        ]),
        Vec::new(),
        core_command_domain::workspace::HookBindingDeclaration::Absent,
    ));
    let report = evaluate(&missing_hook);
    assert_eq!(
        rows(&report)[2..7],
        [
            "✗ aidlc-record-human-turn.ts present — verify file exists in .claude/hooks/".to_string(),
            "✓ aidlc-write-audit-log.ts present".to_string(),
            "✗ Hooks DISABLED via \"disableAllHooks\": true in .claude/settings.json — AI-DLC cannot run (audit, state sync, sensors, and stage-graph rebuild are all silently skipped even though the hook files are present) — remove \"disableAllHooks\": true from .claude/settings.json (or set it to false in a higher-precedence layer such as .claude/settings.local.json) and restart the Claude Code session — AI-DLC's workflow engine is hook-driven and cannot advance while hooks are disabled.".to_string(),
            "✗ Claude managed hook policy: allowManagedHooksOnly=true — hooks from .claude/settings.json are blocked by organization policy (allowManagedHooksOnly); only the Claude Code administrator can lift it in managed-settings.json. Until then, the workflow's human-presence and summary-confirmation receipts cannot be minted; attended sessions can set AIDLC_SKIP_HUMAN_PRESENCE_GUARD=1 and AIDLC_SKIP_SUMMARY_CONFIRMATION_GUARD=1 in the environment that launches the CLI as a temporary bypass".to_string(),
            "✓ settings.json present".to_string(),
        ]
    );

    let managed = with_wiring(HookWiring::new(
        true,
        wired(&["aidlc-write-audit-log.ts"]),
        Some("enterprise managed settings".to_string()),
        false,
        Ok(vec![distributed("write-audit-log")]),
        Vec::new(),
        core_command_domain::workspace::HookBindingDeclaration::Absent,
    ));
    let report = evaluate(&managed);
    let disabled = find(&report, DoctorCheckId::D2b);
    assert_eq!(disabled.len(), 1);
    assert_eq!(
        disabled[0].label(),
        "Hooks DISABLED via \"disableAllHooks\": true in enterprise managed settings — AI-DLC cannot run (audit, state sync, sensors, and stage-graph rebuild are all silently skipped even though the hook files are present)"
    );
    assert_eq!(
        disabled[0].fix(),
        Some(
            "\"disableAllHooks\": true is enforced by enterprise managed settings — the highest-precedence layer, which a project or user setting cannot override. IT policy must remove it (or set it to false) for AI-DLC to run. If policy mandates disabled hooks, AI-DLC v2 is not compatible with this environment — its workflow engine is hook-driven."
        )
    );
    assert!(
        find(&report, DoctorCheckId::D2c).is_empty(),
        "制限が無ければ D2.c の行は無い"
    );
}

#[test]
fn native_hook_bindings_fail_on_broken_json_unknown_targets_and_unknown_native_names() {
    let native = |name: &str| {
        HookBinding::new(
            "Stop".to_string(),
            String::new(),
            format!("\"$CLAUDE_PROJECT_DIR/target/release/aidlc\" hook {name}"),
            HookBindingTarget::Native(name.to_string()),
        )
    };
    let unknown = HookBinding::new(
        "Stop".to_string(),
        String::new(),
        "sh custom.sh".to_string(),
        HookBindingTarget::Unknown,
    );
    let wiring = |bindings: Result<Vec<HookBinding>, ObservationFailure>| {
        HookWiring::new(
            true,
            wired(&["aidlc-write-audit-log.ts"]),
            None,
            false,
            bindings,
            vec![
                "continue-workflow".to_string(),
                "write-audit-log".to_string(),
            ],
            core_command_domain::workspace::HookBindingDeclaration::Absent,
        )
    };
    let binding_row = |bindings| {
        let report = evaluate(&with_wiring(wiring(bindings)));
        let row = find(&report, DoctorCheckId::D2e);
        assert_eq!(row.len(), 1);
        (row[0].is_passed(), row[0].fix().map(str::to_string))
    };
    assert_eq!(
        binding_row(Err(failure("invalid JSON: EOF while parsing"))),
        (
            false,
            Some(".claude/settings.json: invalid JSON: EOF while parsing".to_string())
        )
    );
    // 接続定義が無い作業ツリー (配布そのまま) では、native と配布の混在そのものを失敗にしない。
    // どちらの面へ結ぶかを語る定義が無いので、照合の相手がいない。
    assert_eq!(
        binding_row(Ok(vec![
            distributed("write-audit-log"),
            native("continue-workflow")
        ])),
        (true, None)
    );
    assert_eq!(
        binding_row(Ok(vec![distributed("write-audit-log"), unknown])),
        (
            false,
            Some(".claude/settings.json: binding mismatch (Stop: sh custom.sh)".to_string())
        )
    );
    assert_eq!(
        binding_row(Ok(vec![native("continue-workflow"), native("frobnicate")])),
        (
            false,
            Some(
                ".claude/settings.json: binding mismatch (unknown native hook frobnicate)"
                    .to_string()
            )
        )
    );
    assert_eq!(
        binding_row(Ok(vec![
            native("continue-workflow"),
            native("write-audit-log")
        ])),
        (true, None)
    );
    assert_eq!(
        binding_row(Ok(vec![distributed("write-audit-log")])),
        (true, None)
    );
}

/// D2.e は接続定義 (`scripts/aidlc-selfhost/hook-binding.json`) の宣言と登録を照合する。
///
/// 宣言どおりの混在は正常であり、宣言と食い違う面・宣言に無いフック・読めない定義は失敗である
/// (U4 の接続定義への追従、オーナー裁定 2026-09-12)。
#[test]
fn native_hook_bindings_follow_the_repository_binding_declaration() {
    let native = |name: &str| {
        HookBinding::new(
            "Stop".to_string(),
            String::new(),
            format!("\"$CLAUDE_PROJECT_DIR/target/release/aidlc\" hook {name}"),
            HookBindingTarget::Native(name.to_string()),
        )
    };
    let declared = || HookBindingDeclaration::Declared {
        native: vec![
            "continue-workflow".to_string(),
            "write-audit-log".to_string(),
        ],
        distributed: vec!["run-sensors".to_string()],
    };
    let row = |bindings: Vec<HookBinding>, declaration: HookBindingDeclaration| {
        let wiring = HookWiring::new(
            true,
            wired(&["aidlc-run-sensors.ts"]),
            None,
            false,
            Ok(bindings),
            vec![
                "continue-workflow".to_string(),
                "write-audit-log".to_string(),
                "run-sensors".to_string(),
            ],
            declaration,
        );
        let report = evaluate(&with_wiring(wiring));
        let row = find(&report, DoctorCheckId::D2e);
        assert_eq!(row.len(), 1);
        (row[0].is_passed(), row[0].fix().map(str::to_string))
    };
    // 宣言どおりの混在 — native 2 本 + 配布 1 本は正常な接続の姿である。
    assert_eq!(
        row(
            vec![
                native("continue-workflow"),
                native("write-audit-log"),
                distributed("run-sensors"),
            ],
            declared()
        ),
        (true, None)
    );
    // 宣言が native と言った名前が、配布 `.ts` で登録されている。
    assert_eq!(
        row(
            vec![distributed("continue-workflow"), distributed("run-sensors")],
            declared()
        ),
        (
            false,
            Some(
                ".claude/settings.json: binding mismatch (declared native, registered distributed: continue-workflow)"
                    .to_string()
            )
        )
    );
    // 宣言が配布のままと言った名前が、native 面で登録されている (名前自体は既知)。
    assert_eq!(
        row(vec![native("continue-workflow"), native("run-sensors")], declared()),
        (
            false,
            Some(
                ".claude/settings.json: binding mismatch (declared distributed, registered native: run-sensors)"
                    .to_string()
            )
        )
    );
    // どちらにも宣言されていないフックの登録。
    assert_eq!(
        row(
            vec![native("continue-workflow"), distributed("frobnicate")],
            declared()
        ),
        (
            false,
            Some(
                ".claude/settings.json: binding mismatch (undeclared hook frobnicate)".to_string()
            )
        )
    );
    // 定義はあるが読めない。settings.json ではなく定義の側を名指す。
    assert_eq!(
        row(
            vec![native("continue-workflow")],
            HookBindingDeclaration::Unreadable(failure("invalid JSON: EOF while parsing"))
        ),
        (
            false,
            Some(
                "scripts/aidlc-selfhost/hook-binding.json: unreadable (invalid JSON: EOF while parsing)"
                    .to_string()
            )
        )
    );
}

#[test]
fn heartbeat_follows_the_five_upstream_states_and_the_300000ms_boundary() {
    let row = |heartbeat| {
        let report = evaluate(&with_heartbeat(heartbeat));
        let row = find(&report, DoctorCheckId::D2f);
        assert_eq!(row.len(), 1);
        (
            row[0].is_passed(),
            row[0].label().to_string(),
            row[0].fix().map(str::to_string),
        )
    };
    let advanced = || {
        Some(ObservedTimestamp::new(
            "2026-09-08T00:27:22Z".to_string(),
            Some(1_788_827_242_000),
        ))
    };
    let fired = |raw: &str, millis: i64| {
        vec![HeartbeatEntry::new(
            "fixture".to_string(),
            ObservedTimestamp::new(raw.to_string(), Some(millis)),
        )]
    };

    assert_eq!(
        row(HeartbeatObservation::new(
            false,
            false,
            Vec::new(),
            0,
            false,
            None
        )),
        (
            true,
            "Hook heartbeats: not yet fired (first workflow stage will populate)".to_string(),
            None
        )
    );
    assert_eq!(
        row(HeartbeatObservation::new(
            false,
            false,
            Vec::new(),
            4,
            true,
            advanced()
        )),
        (
            false,
            "Hooks have never executed although this workflow has progressed 4 stages".to_string(),
            Some(HOOK_EXECUTION_RECOVERY.to_string())
        )
    );
    assert_eq!(
        row(HeartbeatObservation::new(
            false,
            false,
            Vec::new(),
            1,
            true,
            advanced()
        ))
        .1,
        "Hooks have never executed although this workflow has progressed 1 stage"
    );
    assert_eq!(
        row(HeartbeatObservation::new(true, false, Vec::new(), 4, true, advanced())),
        (
            false,
            "Hook heartbeat data".to_string(),
            Some("health dir exists and the ledger shows STAGE_STARTED, but no hook has ever fired — verify hooks are registered in settings.json".to_string())
        )
    );
    assert!(
        row(HeartbeatObservation::new(
            true,
            false,
            Vec::new(),
            0,
            false,
            None
        ))
        .0,
        "空の health ディレクトリは進行前なら初回扱い"
    );
    assert_eq!(
        row(HeartbeatObservation::new(true, true, Vec::new(), 4, true, advanced())),
        (
            false,
            "Hook heartbeat data".to_string(),
            Some("health dir exists but heartbeat files are unreadable — verify permissions and hook registration".to_string())
        )
    );
    assert_eq!(
        row(HeartbeatObservation::new(
            true,
            true,
            fired("2026-09-08T00:22:22.000Z", 1_788_826_942_000),
            4,
            true,
            advanced()
        )),
        (
            true,
            "Hooks last fired: fixture 2026-09-08T00:22:22.000Z".to_string(),
            None
        ),
        "ちょうど 300000ms は遅延にしない"
    );
    assert_eq!(
        row(HeartbeatObservation::new(
            true,
            true,
            fired("2026-09-08T00:22:21.999Z", 1_788_826_941_999),
            4,
            true,
            advanced()
        )),
        (
            false,
            "Hooks last fired 2026-09-08T00:22:21.999Z, but the workflow last advanced 2026-09-08T00:27:22Z".to_string(),
            Some(HOOK_EXECUTION_RECOVERY.to_string())
        )
    );
    let two = vec![
        HeartbeatEntry::new(
            "session-start".to_string(),
            ObservedTimestamp::new("2026-09-08T00:27:20Z".to_string(), Some(1_788_827_240_000)),
        ),
        HeartbeatEntry::new(
            "write-audit-log".to_string(),
            ObservedTimestamp::new("not-a-date".to_string(), None),
        ),
    ];
    assert_eq!(
        row(HeartbeatObservation::new(
            true,
            true,
            two,
            4,
            true,
            advanced()
        ))
        .1,
        "Hooks last fired: session-start 2026-09-08T00:27:20Z, write-audit-log not-a-date"
    );
}

#[test]
fn scope_validation_distinguishes_advisories_errors_unknown_scopes_and_unavailable_assets() {
    let scope_row = |definition| {
        let report = evaluate(&with_definition(definition));
        let row = find(&report, DoctorCheckId::D3b);
        assert_eq!(row.len(), 1);
        (
            row[0].is_passed(),
            row[0].label().to_string(),
            row[0].fix().map(str::to_string),
            report.exit_code(),
        )
    };
    let ok = scope_row(tiny_definition());
    assert_eq!(
        ok,
        (
            true,
            "Scope validation: 2 scopes valid (1 advisories)".to_string(),
            None,
            0
        ),
        "advisory だけなら失敗にしない"
    );

    let mut starved = tiny_graph();
    starved.push(stage("d", "2.4", &[], &[], &[("zzz", true)]));
    let mut grid_with_d = grid(&["b", "c"], &["a", "b", "c"]);
    grid_with_d[1] = ScopeGridEntry::new(
        "feature".to_string(),
        vec![
            ("a".to_string(), "EXECUTE".to_string()),
            ("b".to_string(), "EXECUTE".to_string()),
            ("c".to_string(), "EXECUTE".to_string()),
            ("d".to_string(), "EXECUTE".to_string()),
        ],
    );
    let errors = scope_row(DefinitionAssets::new(
        Ok(starved),
        Ok(grid_with_d),
        Ok(vec!["bugfix".to_string(), "feature".to_string()]),
        Ok(Vec::new()),
        Ok(Vec::new()),
    ));
    assert_eq!(
        errors,
        (
            false,
            "Scope validation: 1 of 2 scopes have errors".to_string(),
            Some("feature: Stage \"d\" requires artifact \"zzz\" but no stage in the graph produces it.".to_string()),
            1
        )
    );

    let unknown = scope_row(DefinitionAssets::new(
        Ok(tiny_graph()),
        Ok(grid(&["b"], &["a"])),
        Ok(vec!["bugfix".to_string(), "classic".to_string()]),
        Ok(Vec::new()),
        Ok(Vec::new()),
    ));
    assert_eq!(
        unknown,
        (
            false,
            "Scope validation: check failed".to_string(),
            Some("Unknown scope: \"feature\". Valid scopes: bugfix, classic".to_string()),
            1
        )
    );

    let unavailable = scope_row(DefinitionAssets::new(
        Err(failure(
            "Stage graph not readable at /w/.claude/tools/data/stage-graph.json: No such file",
        )),
        Ok(grid(&["b"], &["a"])),
        Ok(vec!["bugfix".to_string(), "feature".to_string()]),
        Ok(Vec::new()),
        Ok(Vec::new()),
    ));
    assert_eq!(
        unavailable,
        (
            false,
            "Scope validation: check failed".to_string(),
            Some(
                "Stage graph not readable at /w/.claude/tools/data/stage-graph.json: No such file"
                    .to_string()
            ),
            1
        )
    );
}

#[test]
fn graph_rows_report_cycles_missing_files_schema_failures_and_broken_references() {
    let labels = |definition| {
        let report = evaluate(&with_definition(definition));
        let mut out: Vec<(bool, String, Option<String>)> = Vec::new();
        for id in [DoctorCheckId::D3c, DoctorCheckId::D3d] {
            for check in find(&report, id) {
                out.push((
                    check.is_passed(),
                    check.label().to_string(),
                    check.fix().map(str::to_string),
                ));
            }
        }
        out
    };

    let mut cyclic = vec![
        stage("a", "1.1", &["b"], &[], &[]),
        stage("b", "1.2", &["a"], &[], &[]),
        stage("c", "1.3", &["c"], &[], &[]),
        stage("d", "1.4", &["e"], &[], &[]),
        stage("e", "1.5", &["f"], &[], &[]),
        stage("f", "1.6", &["d"], &[], &[]),
    ];
    cyclic.push(stage("g", "1.7", &["not-a-stage"], &[], &[("ghost", true)]));
    let files: Vec<StageFile> = ["a", "b", "c", "d", "e", "f"]
        .iter()
        .map(|slug| stage_file(slug, "inline", "aidlc-product-agent"))
        .chain(std::iter::once(stage_file(
            "g",
            "invalid-mode",
            "aidlc-product-agent",
        )))
        .collect();
    let out = labels(DefinitionAssets::new(
        Ok(cyclic),
        Ok(Vec::new()),
        Ok(vec!["bugfix".to_string(), "feature".to_string()]),
        Ok(files),
        Ok(vec!["aidlc-product-agent".to_string()]),
    ));
    assert_eq!(
        out,
        [
            (
                false,
                "Cycle detection: 3 cycle(s) found".to_string(),
                Some("cycles: b → a; c; f → e → d".to_string())
            ),
            (
                true,
                "Orphan stage files: 7 graph entries all have files".to_string(),
                None
            ),
            (
                false,
                "Schema validation: 1 of 7 stage(s) failed".to_string(),
                Some("g: mode must be one of inline | subagent | pipeline | mob | agent-team, got \"invalid-mode\"".to_string())
            ),
            (
                false,
                "Graph references: 2 broken reference(s)".to_string(),
                Some("g: consumes unknown artifact \"ghost\"; g: requires_stage unknown slug \"not-a-stage\"".to_string())
            ),
        ]
    );

    let mut missing_file = tiny_definition_files_without("b");
    missing_file.push(stage_file("z", "inline", "aidlc-product-agent"));
    let out = labels(DefinitionAssets::new(
        Ok(tiny_graph()),
        Ok(Vec::new()),
        Ok(vec!["bugfix".to_string(), "feature".to_string()]),
        Ok(missing_file),
        Ok(vec![
            "aidlc-product-agent".to_string(),
            "aidlc-developer-agent".to_string(),
        ]),
    ));
    assert_eq!(
        out[1],
        (
            false,
            "Orphan stage files: 1 graph entries have no file on disk".to_string(),
            Some("missing files: b".to_string())
        )
    );
    assert_eq!(
        out[2],
        (
            true,
            "Schema validation: 2/2 stages validated".to_string(),
            None
        ),
        "無いファイルは schema の対象に数えない・グラフ外のファイルは見ない"
    );

    let out = labels(DefinitionAssets::new(
        Ok(tiny_graph()),
        Ok(Vec::new()),
        Ok(vec!["bugfix".to_string(), "feature".to_string()]),
        Ok(vec![
            stage_file("a", "inline", "aidlc-product-agent"),
            stage_file("b", "subagent", "nobody-agent"),
            StageFile::new(
                "inception".to_string(),
                "c".to_string(),
                Ok("no frontmatter here\n".to_string()),
            ),
        ]),
        Ok(vec!["aidlc-product-agent".to_string()]),
    ));
    assert_eq!(
        out[2],
        (
            false,
            "Schema validation: 2 of 3 stage(s) failed".to_string(),
            Some("b: lead_agent \"nobody-agent\" has no matching .claude/agents/*.md; c: Stage file missing YAML frontmatter (---...---)".to_string())
        )
    );

    let out = labels(DefinitionAssets::new(
        Ok(tiny_graph()),
        Ok(Vec::new()),
        Ok(vec!["bugfix".to_string(), "feature".to_string()]),
        Ok(vec![
            stage_file("a", "inline", "aidlc-product-agent"),
            StageFile::new(
                "inception".to_string(),
                "b".to_string(),
                Err(failure("Permission denied")),
            ),
            stage_file("c", "inline", "aidlc-product-agent"),
        ]),
        Ok(vec!["aidlc-product-agent".to_string()]),
    ));
    assert_eq!(
        out[2],
        (
            false,
            "Schema validation: check failed".to_string(),
            Some("Permission denied".to_string())
        ),
        "存在するのに読めないファイルは本家どおり検査全体の失敗"
    );

    let out = labels(DefinitionAssets::new(
        Err(failure("graph broken")),
        Ok(Vec::new()),
        Ok(Vec::new()),
        Ok(Vec::new()),
        Err(failure(
            "Agent file /w/.claude/agents/x.md missing required frontmatter: name",
        )),
    ));
    assert_eq!(
        out,
        [
            (
                false,
                "Cycle detection: graph load failed".to_string(),
                Some("graph broken".to_string())
            ),
            (
                false,
                "Orphan stage files: check failed".to_string(),
                Some("graph broken".to_string())
            ),
            (
                false,
                "Schema validation: check failed".to_string(),
                Some("graph broken".to_string())
            ),
            (
                false,
                "Graph references: check failed".to_string(),
                Some("graph broken".to_string())
            ),
        ]
    );
}

fn tiny_definition_files_without(slug: &str) -> Vec<StageFile> {
    [
        ("a", "inline", "aidlc-product-agent"),
        ("b", "subagent", "aidlc-developer-agent"),
        ("c", "inline", "aidlc-product-agent"),
    ]
    .iter()
    .filter(|(name, _, _)| *name != slug)
    .map(|(name, mode, lead)| stage_file(name, mode, lead))
    .collect()
}

#[test]
fn record_rows_cover_state_version_readability_identity_store_and_projection() {
    let report = evaluate(&with_record(healthy_record()));
    assert_eq!(
        rows(&report)[14..],
        [
            "✓ State Version: 8".to_string(),
            "✓ Native workflow state readable".to_string(),
            "✓ Native workflow identity".to_string(),
            "✓ Native event store readable".to_string(),
            "✓ Native projection consistency".to_string(),
        ]
    );
    assert_eq!(report.exit_code(), 0);

    let record_rows = |record| {
        let report = evaluate(&with_record(record));
        rows(&report)[14..].to_vec()
    };
    let with_state = |state| {
        RecordObservation::new(
            location(Some("260908-doctor")),
            state,
            Ok(Some(ExecutionCursorObservation::new(
                EXECUTION.to_string(),
                INTENT.to_string(),
            ))),
            Ok(Some("260908-doctor".to_string())),
            opened(),
            Ok(consistent()),
        )
    };
    assert_eq!(
        record_rows(with_state(StateFileObservation::Classified(
            StateVersionObservation::new(
                classified("7"),
                Some("Incompatible workflow state: State Version 7 predates ...".to_string()),
            )
        )))[..2],
        [
            "✗ state version current — Incompatible workflow state: State Version 7 predates ..."
                .to_string(),
            "✓ Native workflow state readable".to_string(),
        ]
    );
    assert_eq!(
        record_rows(with_state(StateFileObservation::Classified(
            StateVersionObservation::new(classified("9"), Some("newer".to_string()),)
        )))[0],
        "✗ state version compatible — newer"
    );
    assert_eq!(
        record_rows(with_state(StateFileObservation::Classified(
            StateVersionObservation::new(classified(""), Some("missing".to_string()),)
        )))[0],
        "✗ state version readable — missing"
    );
    assert_eq!(
        record_rows(with_state(StateFileObservation::Unreadable(failure("Is a directory"))))[..2],
        [
            "✗ Native workflow state readable — aidlc/spaces/default/intents/260908-doctor/aidlc-state.md: unreadable (Is a directory)".to_string(),
            "✓ Native workflow identity".to_string(),
        ],
        "不読では本家 D4.a の行は出ず、独自 D4.b が明示する"
    );
    assert_eq!(
        record_rows(with_state(StateFileObservation::Absent))[0],
        "✗ Native workflow state readable — aidlc/spaces/default/intents/260908-doctor/aidlc-state.md: missing"
    );

    let identity = |selected: Option<&str>, cursor, registry, projection| {
        RecordObservation::new(
            location(selected),
            StateFileObservation::Classified(StateVersionObservation::new(classified("8"), None)),
            cursor,
            registry,
            opened(),
            projection,
        )
    };
    let cursor_ok = || {
        Ok(Some(ExecutionCursorObservation::new(
            EXECUTION.to_string(),
            INTENT.to_string(),
        )))
    };
    assert_eq!(
        record_rows(identity(None, Ok(None), Ok(None), Ok(consistent())))[2],
        "✗ Native workflow identity — aidlc/spaces/default/intents: ambiguous (1 records, no active record selected)"
    );
    assert_eq!(
        record_rows(identity(
            Some("260908-doctor"),
            Ok(None),
            Ok(None),
            Ok(consistent())
        ))[2],
        "✗ Native workflow identity — aidlc/spaces/default/intents/260908-doctor/.aidlc-execution: missing"
    );
    assert_eq!(
        record_rows(identity(
            Some("260908-doctor"),
            Err(failure("malformed execution cursor")),
            Ok(None),
            Ok(consistent())
        ))[2],
        "✗ Native workflow identity — aidlc/spaces/default/intents/260908-doctor/.aidlc-execution: unreadable (malformed execution cursor)"
    );
    assert_eq!(
        record_rows(identity(
            Some("260908-doctor"),
            cursor_ok(),
            Ok(None),
            Ok(consistent())
        ))[2],
        format!(
            "✗ Native workflow identity — aidlc/spaces/default/intents/intents.json: missing entry for intent {INTENT}"
        )
    );
    assert_eq!(
        record_rows(identity(
            Some("260908-doctor"),
            cursor_ok(),
            Ok(Some("260901-other".to_string())),
            Ok(consistent())
        ))[2],
        format!(
            "✗ Native workflow identity — aidlc/spaces/default/intents/intents.json: mismatch (intent {INTENT} is registered to 260901-other)"
        )
    );
    assert_eq!(
        record_rows(identity(
            Some("260908-doctor"),
            cursor_ok(),
            Err(failure("invalid JSON")),
            Ok(consistent())
        ))[2],
        "✗ Native workflow identity — aidlc/spaces/default/intents/intents.json: unreadable (invalid JSON)"
    );
    assert_eq!(
        record_rows(identity(
            Some("260908-doctor"),
            cursor_ok(),
            Ok(Some("260908-doctor".to_string())),
            Ok(ProjectionObservation::new(
                Some("other-intent".to_string()),
                Some(7),
                Some(7),
                0,
                1
            ))
        ))[2],
        format!(
            "✗ Native workflow identity — aidlc/spaces/default/intents/.aidlc-store.sqlite: mismatch (execution {EXECUTION} belongs to intent other-intent)"
        )
    );
    assert_eq!(
        record_rows(identity(
            Some("260908-doctor"),
            cursor_ok(),
            Ok(Some("260908-doctor".to_string())),
            Err(failure("store missing"))
        ))[2],
        "✗ Native workflow identity — aidlc/spaces/default/intents/.aidlc-store.sqlite: unreadable (store missing)"
    );

    let with_store = |store, projection| {
        RecordObservation::new(
            location(Some("260908-doctor")),
            StateFileObservation::Classified(StateVersionObservation::new(classified("8"), None)),
            cursor_ok(),
            Ok(Some("260908-doctor".to_string())),
            store,
            projection,
        )
    };
    assert_eq!(
        record_rows(with_store(StoreObservation::Absent, Err(failure("store missing"))))[3..],
        [
            "✗ Native event store readable — aidlc/spaces/default/intents/.aidlc-store.sqlite: missing".to_string(),
            "✗ Native projection consistency — aidlc/spaces/default/intents/.aidlc-store.sqlite: projection unavailable (store missing)".to_string(),
        ]
    );
    assert_eq!(
        record_rows(with_store(
            StoreObservation::Unreadable(failure("is a directory")),
            Err(failure("store unreadable: is a directory"))
        ))[3],
        "✗ Native event store readable — aidlc/spaces/default/intents/.aidlc-store.sqlite: unreadable (is a directory)"
    );
    assert_eq!(
        record_rows(with_store(
            StoreObservation::Opened(StoreSchema::new(vec!["journal".to_string()], 0)),
            Ok(consistent())
        ))[3],
        "✗ Native event store readable — aidlc/spaces/default/intents/.aidlc-store.sqlite: incompatible schema (missing tables: amadeus_projection_checkpoint, read_execution; read schema version 0)"
    );
    let projection_row = |projection| record_rows(with_store(opened(), Ok(projection)))[4].clone();
    assert_eq!(
        projection_row(ProjectionObservation::new(None, Some(7), Some(7), 0, 1)),
        format!(
            "✗ Native projection consistency — aidlc/spaces/default/intents/.aidlc-store.sqlite: projection unavailable (execution {EXECUTION} not projected)"
        )
    );
    assert_eq!(
        projection_row(ProjectionObservation::new(
            Some(INTENT.to_string()),
            None,
            Some(7),
            0,
            1
        )),
        format!(
            "✗ Native projection consistency — aidlc/spaces/default/intents/.aidlc-store.sqlite: projection unavailable (no checkpoint for orchestration-{EXECUTION})"
        )
    );
    assert_eq!(
        projection_row(ProjectionObservation::new(
            Some(INTENT.to_string()),
            Some(5),
            Some(7),
            0,
            1
        )),
        "✗ Native projection consistency — aidlc/spaces/default/intents/.aidlc-store.sqlite: projection unavailable (journal position 7 is ahead of checkpoint 5)"
    );
    assert_eq!(
        projection_row(ProjectionObservation::new(
            Some(INTENT.to_string()),
            Some(7),
            Some(7),
            2,
            1
        )),
        "✗ Native projection consistency — aidlc/spaces/default/intents/.aidlc-store.sqlite: projection unavailable (2 pending publications)"
    );
    assert_eq!(
        projection_row(ProjectionObservation::new(
            Some(INTENT.to_string()),
            Some(7),
            Some(7),
            0,
            0
        )),
        "✗ Native projection consistency — aidlc/spaces/default/intents/260908-doctor/audit: projection unavailable (no audit shard)"
    );
    assert_eq!(
        projection_row(ProjectionObservation::new(
            Some(INTENT.to_string()),
            Some(9),
            Some(7),
            0,
            1
        )),
        "✓ Native projection consistency",
        "チェックポイントがジャーナルより先でも未反映ではない"
    );
}

#[test]
fn the_aggregate_records_each_diagnosis_as_one_event_and_replays_the_latest_rows() {
    use core_command_domain::workspace::{WorkspaceDoctorError, WorkspaceDoctorId};

    let (mut doctor, genesis) = WorkspaceDoctor::start(target(), &cold(), at()).unwrap();
    assert_eq!(doctor.id(), &WorkspaceDoctor::id_for(&target()));
    assert_eq!(doctor.id(), genesis.aggregate_id());
    assert_eq!(doctor.seq_nr(), 1);
    assert_eq!(doctor.version(), 0);
    assert_eq!(doctor.diagnosed_at(), at());
    assert_eq!(doctor.checks().failed(), 0);

    // 2 回目の診断は同じ集約に 1 イベントを足し、行は最新の観測を映す。
    let later: DateTime<Utc> = "2026-09-12T00:01:00Z".parse().unwrap();
    let broken = with_wiring(HookWiring::new(
        false,
        Err(failure("ENOENT")),
        None,
        false,
        Err(failure("ENOENT")),
        Vec::new(),
        core_command_domain::workspace::HookBindingDeclaration::Absent,
    ));
    let second = doctor.diagnose(&broken, later).unwrap();
    assert_eq!(doctor.seq_nr(), 2);
    assert_eq!(doctor.diagnosed_at(), later);
    assert_eq!(doctor.checks(), second.checks());
    assert!(doctor.checks().failed() > 0);
    assert_ne!(genesis.id(), second.id(), "イベントは自前の識別子を持つ");

    // 誕生の snapshot に差分イベントを畳むと同じ最新状態になる。
    let snapshot = WorkspaceDoctor::new(
        genesis.aggregate_id().clone(),
        target(),
        genesis.checks().clone(),
        1,
        0,
        at(),
    )
    .unwrap();
    let replayed = WorkspaceDoctor::replay(snapshot, [(second, 2, later)]).with_version(2);
    assert_eq!(replayed, doctor.clone().with_version(2));

    // 完全コンストラクタは識別子と対象の不一致・通番 0 を拒む。
    let other = HookHealthTarget::new(SpaceName::parse("other").unwrap(), None);
    assert_eq!(
        WorkspaceDoctor::new(
            WorkspaceDoctorId::for_target(&other),
            target(),
            genesis.checks().clone(),
            1,
            0,
            at()
        )
        .unwrap_err(),
        WorkspaceDoctorError::TargetMismatch
    );
    assert_eq!(
        WorkspaceDoctor::new(
            genesis.aggregate_id().clone(),
            target(),
            genesis.checks().clone(),
            0,
            0,
            at()
        )
        .unwrap_err(),
        WorkspaceDoctorError::InvalidHistory
    );

    // 識別子と検査 ID は保存された綴りから戻せる (不正な綴りは拒む)。
    assert_eq!(
        WorkspaceDoctorId::parse(doctor.id().as_str()).unwrap(),
        doctor.id().clone()
    );
    assert_eq!(
        WorkspaceDoctorId::parse("workspace-doctor:zz").unwrap_err(),
        WorkspaceDoctorError::InvalidIdentity
    );
    assert_eq!(DoctorCheckId::parse("D2.f").unwrap(), DoctorCheckId::D2f);
    assert_eq!(
        DoctorCheckId::parse("D9.z").unwrap_err(),
        WorkspaceDoctorError::InvalidCheckId
    );
    for check in doctor.checks().as_slice() {
        assert_eq!(
            DoctorCheckId::parse(check.id().as_str()).unwrap(),
            check.id()
        );
    }
}

#[test]
#[should_panic(expected = "invalid WorkspaceDoctor history")]
fn a_foreign_event_cannot_be_applied_as_this_doctors_history() {
    let (mut doctor, _) = WorkspaceDoctor::start(target(), &cold(), at()).unwrap();
    let other = HookHealthTarget::new(SpaceName::parse("other").unwrap(), None);
    let (_, foreign) = WorkspaceDoctor::start(other, &cold(), at()).unwrap();
    doctor.apply_event(&foreign, 2, at());
}
