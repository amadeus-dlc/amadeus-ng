//! 状態ファイルの**骨格**を書く — 鋳造直後の初期スケルトン。
//!
//! # なぜ合成ルートの仕事なのか
//!
//! 投影がそう決めているからである。RMU の `started` 投影は「状態ファイルの**骨格が
//! 無ければ**（本文が空）`ScaffoldMissing` で止まる — **骨格を書くのは合成ルートであって
//! 投影ではない**（オーナー裁定 2026-08-29）」と逐語で書いており、`Stages to Execute` /
//! `Stages to Skip` の 2 行も明示的に「書くのは合成ルート」と名指ししている。
//!
//! 投影が書くのは**その後の差分**（初期化ステージの完了・最初のゲート付きステージへの
//! 着地・総数・カーソル）だけである。ここが用意するのは、投影が書き換える先の**行**である。
//!
//! # バイト一致は未検証である
//!
//! upstream のテンプレート（`aidlc-utility.ts:4252`）と同じ節・同じフィールド名・同じ順序で
//! 組んでいるが、**バイト一致は突き合わせていない** — 0b ゴールデンの CLI 面は採取できて
//! おらず（`tests/golden/upstream-3c3146cf/cli/cases-missing.json`）、比べる相手が存在しない。
//! 現時点で保証しているのは「投影とクエリ側パーサが読める構造」までである。

use core_command_domain::orchestration::{Intent, StageEntry};
use core_command_domain::workflow_definition::{PhaseId, PlanAction};

/// フェーズの提示順（upstream の `phaseProgressLines` と同じ 5 行）。
const PHASES: [PhaseId; 5] = [
    PhaseId::Initialization,
    PhaseId::Ideation,
    PhaseId::Inception,
    PhaseId::Construction,
    PhaseId::Operation,
];

/// 状態ファイルの版（`- **State Version**:`）。
const STATE_VERSION: &str = "8";

/// チェックボックス語彙の注記（upstream の `## Stage Progress` 直下のコメント）。
const CHECKBOX_LEGEND: &str = "<!-- Checkbox states: [ ] not started, [-] in progress, [?] awaiting approval (gate open), [R] revising (user rejected gate), [x] completed, [S] skipped via --stage/--phase jump -->";

/// 鋳造した intent から初期スケルトンを組む。
///
/// `project_root` と `started_at` は合成ルートが持つ環境の値（ドメインは知らない）。
#[must_use]
pub fn compose(intent: &Intent, project_root: &str, started_at: &str) -> String {
    let scan = intent.scan();
    let execute: Vec<&str> = in_scope(intent).map(|s| s.slug().as_str()).collect();
    let skip: Vec<&str> = intent
        .stages()
        .iter()
        .filter(|s| s.plan_action() == PlanAction::Skip)
        .map(|s| s.slug().as_str())
        .collect();
    let first = first_post_initialization(intent);
    let mut out = String::new();

    out.push_str("# AI-DLC State Tracking\n\n");

    out.push_str("## Project Information\n");
    push_field(&mut out, "Project", intent.request());
    push_field(&mut out, "Project Type", scan.project_type());
    push_field(&mut out, "Scope", intent.scope());
    push_field(&mut out, "Start Date", started_at);
    push_field(&mut out, "State Version", STATE_VERSION);
    push_field(
        &mut out,
        "Active Agent",
        first.map_or("", |s| s.display().lead_agent()),
    );
    push_field(&mut out, "Worktree Path", "");
    push_field(&mut out, "Bolt Refs", "");
    push_field(&mut out, "Practices Affirmed Timestamp", "");

    out.push_str("\n## Scope Configuration\n");
    push_field(&mut out, "Stages to Execute", &execute.join(", "));
    let skipped = if skip.is_empty() {
        "none".to_string()
    } else {
        skip.join(", ")
    };
    push_field(&mut out, "Stages to Skip", &skipped);
    push_field(&mut out, "Depth", intent.depth().unwrap_or(""));
    push_field(
        &mut out,
        "Test Strategy",
        intent.test_strategy().unwrap_or(""),
    );
    push_field(&mut out, "Review Override", stored_review(intent.review()));

    out.push_str("\n## Workspace State\n");
    push_field(&mut out, "Project Root", project_root);
    push_field(&mut out, "Languages", scan.languages());
    push_field(&mut out, "Frameworks", scan.frameworks());
    push_field(&mut out, "Build System", scan.build_system());

    out.push_str("\n## Execution Plan Summary\n");
    push_field(&mut out, "Total Stages", &execute.len().to_string());
    // 完了数とカーソルは**投影が書く** — ここは書き換え先の行を用意するだけである。
    push_field(&mut out, "Completed", "0");
    push_field(&mut out, "In Progress", "");

    out.push_str("\n## Runtime State\n");
    push_field(&mut out, "Revision Count", "0");
    // **M12 の修正**（逸脱台帳 #2 / 10 §10）。upstream の birth はこの行を書かないため、
    // `setFieldStrict` を使う set-autonomy が新規状態ファイルで必ず
    // `State update failed: Field not found …` で失敗する（upstream 03 の文書化済み
    // discrepancy）。我々は**書く**と裁定済みなので、骨格に初めから置く。
    // 値は fail-closed の既定（`AutonomyMode::from_state_field` の吸収先）。
    push_field(&mut out, "Construction Autonomy Mode", "gated");

    out.push_str("\n## Phase Progress\n");
    out.push_str("<!-- Status values: Pending, Active, Verified, Skipped -->\n\n");
    for phase in PHASES {
        push_field(&mut out, phase_label(phase), &phase_status(intent, phase));
    }

    out.push_str("\n## Stage Progress\n");
    out.push_str(CHECKBOX_LEGEND);
    out.push('\n');
    push_stage_progress(&mut out, intent);

    out.push_str("## Current Status\n");
    // 値は**大文字**のフェーズ名（投影の `field::LIFECYCLE_PHASE` の doc — `INCEPTION`）。
    let lifecycle = first.map_or_else(String::new, |s| phase_label(s.phase()).to_uppercase());
    push_field(&mut out, "Lifecycle Phase", &lifecycle);
    push_field(&mut out, "Current Stage", "");
    push_field(&mut out, "Next Stage", "");
    push_field(&mut out, "Status", "Running");
    push_field(&mut out, "Last Updated", started_at);

    out.push_str("\n## Session Resume Point\n");
    push_field(&mut out, "Last Completed Stage", "");
    push_field(&mut out, "Next Action", "");
    push_field(&mut out, "Pending Artifacts", "none");

    out
}

/// `Review Override` の格納形（upstream `storedReviewOverride` `aidlc-utility.ts:164-168`）。
///
/// `adversarial` は「このランに上限を掛けない」の意であり、**空欄で表す** —
/// ステージ宣言と scope の上限がそのまま効く状態を、`config-change` も同じ空欄で書く。
/// 値を書いてしまうと「上限が設定されている」に読める。
fn stored_review(review: Option<&str>) -> &str {
    match review {
        Some("adversarial") | None => "",
        Some(value) => value,
    }
}

/// `- **<name>**: <value>` の 1 行（値が空でもコロンまでは書く — 投影の書き換え先になる）。
fn push_field(out: &mut String, name: &str, value: &str) {
    if value.is_empty() {
        out.push_str(&format!("- **{name}**:\n"));
    } else {
        out.push_str(&format!("- **{name}**: {value}\n"));
    }
}

/// フェーズ見出しごとに、そのフェーズの実行対象ステージ行を並べる。
fn push_stage_progress(out: &mut String, intent: &Intent) {
    for phase in PHASES {
        let stages: Vec<&StageEntry> = intent
            .stages()
            .iter()
            .filter(|s| s.phase() == phase && s.plan_action() == PlanAction::Execute)
            .collect();
        if stages.is_empty() {
            continue;
        }
        out.push_str(&format!(
            "\n### {} PHASE\n",
            phase_label(phase).to_uppercase()
        ));
        for stage in stages {
            out.push_str(&format!("- [ ] {} — EXECUTE\n", stage.slug().as_str()));
        }
    }
    out.push('\n');
}

/// 実行対象（EXECUTE）のステージ。
fn in_scope(intent: &Intent) -> impl Iterator<Item = &StageEntry> {
    intent
        .stages()
        .iter()
        .filter(|s| s.plan_action() == PlanAction::Execute)
}

/// initialization より後の最初の実行対象ステージ（着地先）。
fn first_post_initialization(intent: &Intent) -> Option<&StageEntry> {
    in_scope(intent).find(|s| s.phase() != PhaseId::Initialization)
}

/// upstream の `phaseStatus` — initialization は Verified、着地先のフェーズは Active、
/// 実行対象が 1 つも無いフェーズは Skipped、それ以外は Pending。
fn phase_status(intent: &Intent, phase: PhaseId) -> String {
    if phase == PhaseId::Initialization {
        return "Verified".to_string();
    }
    if first_post_initialization(intent).is_some_and(|s| s.phase() == phase) {
        return "Active".to_string();
    }
    if in_scope(intent).any(|s| s.phase() == phase) {
        "Pending".to_string()
    } else {
        "Skipped".to_string()
    }
}

/// 見出し・フィールド名で使う表記（`initialization` → `Initialization`）。
const fn phase_label(phase: PhaseId) -> &'static str {
    match phase {
        PhaseId::Initialization => "Initialization",
        PhaseId::Ideation => "Ideation",
        PhaseId::Inception => "Inception",
        PhaseId::Construction => "Construction",
        PhaseId::Operation => "Operation",
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use core_command_domain::orchestration::{
        Created, IntentId, StageDisplay, StartRequest, WorkspaceScan,
    };
    use core_command_domain::workflow_definition::{
        BrownfieldGreenfield, DefinitionRevision, StageNumber, StageSlug, WorkflowDefinitionId,
    };

    fn stage(slug: &str, number: &str, phase: PhaseId, action: PlanAction) -> StageEntry {
        StageEntry::new(
            StageSlug::parse(slug).expect("slug"),
            phase,
            action,
            false,
            StageDisplay::new(
                StageNumber::parse(number).expect("番号"),
                "Stage",
                "orchestrator",
            )
            .expect("単一行"),
        )
    }

    fn intent(stages: Vec<StageEntry>) -> Intent {
        intent_with(
            stages,
            StartRequest::new("classic", "build the auth service"),
        )
    }

    fn intent_with(stages: Vec<StageEntry>, request: StartRequest) -> Intent {
        Intent::from((
            Created::new(
                IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("UUIDv7"),
                WorkflowDefinitionId::parse("claude").expect("定義 id"),
                DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision"),
                request,
                stages,
                WorkspaceScan::new(BrownfieldGreenfield::Greenfield, "Rust", "None", "Cargo")
                    .expect("単一行"),
            ),
            chrono::DateTime::parse_from_rfc3339("2026-08-23T00:00:00Z")
                .expect("固定時刻")
                .with_timezone(&chrono::Utc),
        ))
    }

    fn three_stage() -> Intent {
        intent(three_stage_plan())
    }

    fn three_stage_plan() -> Vec<StageEntry> {
        vec![
            stage(
                "state-init",
                "0.1",
                PhaseId::Initialization,
                PlanAction::Execute,
            ),
            stage(
                "domain-design",
                "1.1",
                PhaseId::Inception,
                PlanAction::Execute,
            ),
            stage(
                "contract-design",
                "1.2",
                PhaseId::Inception,
                PlanAction::Execute,
            ),
        ]
    }

    /// スコープ外のフェーズは `Skipped` と書く（`Pending` と区別が要る — 実施しないため）。
    #[test]
    fn a_phase_with_no_in_scope_stage_is_skipped() {
        let plan = vec![
            stage(
                "state-init",
                "0.1",
                PhaseId::Initialization,
                PlanAction::Execute,
            ),
            stage(
                "domain-design",
                "1.1",
                PhaseId::Inception,
                PlanAction::Execute,
            ),
            stage(
                "code-generation",
                "3.5",
                PhaseId::Construction,
                PlanAction::Skip,
            ),
            stage(
                "incident-response",
                "4.2",
                PhaseId::Operation,
                PlanAction::Execute,
            ),
        ];

        let state = compose(&intent(plan), "/ws", "2026-08-31T00:00:00Z");

        assert!(state.contains("- **Construction**: Skipped"), "{state}");
        assert!(state.contains("- **Inception**: Active"), "{state}");
        // 着地フェーズより後ろで EXECUTE が残っているものは `Pending`（未実施であって
        // 実施しないわけではない — `Skipped` と混ぜない）。
        assert!(state.contains("- **Operation**: Pending"), "{state}");
    }

    /// `--review` は状態ファイルまで貫通する（`--depth` / `--test-strategy` と同じ形）。
    #[test]
    fn the_review_override_reaches_the_state_file() {
        let request = StartRequest::new("classic", "build it")
            .with_depth("standard")
            .with_test_strategy("minimal")
            .with_review("advisory");

        let state = compose(
            &intent_with(three_stage_plan(), request),
            "/ws",
            "2026-08-31T00:00:00Z",
        );

        assert!(state.contains("- **Depth**: standard"), "{state}");
        assert!(state.contains("- **Test Strategy**: minimal"), "{state}");
        assert!(state.contains("- **Review Override**: advisory"), "{state}");
    }

    /// `adversarial` は「上限なし」なので**空欄**で書く（upstream `storedReviewOverride`）。
    #[test]
    fn an_adversarial_review_is_stored_as_the_empty_field() {
        let request = StartRequest::new("classic", "build it").with_review("adversarial");

        let state = compose(
            &intent_with(three_stage_plan(), request),
            "/ws",
            "2026-08-31T00:00:00Z",
        );

        assert!(state.contains("- **Review Override**:\n"), "{state}");
    }

    #[test]
    fn the_scaffold_carries_the_sections_the_projection_writes_into() {
        let state = compose(&three_stage(), "/ws", "2026-08-31T00:00:00Z");
        for section in [
            "# AI-DLC State Tracking",
            "## Project Information",
            "## Scope Configuration",
            "## Workspace State",
            "## Execution Plan Summary",
            "## Runtime State",
            "## Phase Progress",
            "## Stage Progress",
            "## Current Status",
        ] {
            assert!(state.contains(section), "{section} が要る:\n{state}");
        }
    }

    /// 投影が書き換える先の行が**存在する**こと（無いと `StateField` で倒れる）。
    #[test]
    fn the_scaffold_pre_creates_every_field_the_projection_updates() {
        let state = compose(&three_stage(), "/ws", "2026-08-31T00:00:00Z");
        for field in [
            "- **Completed**:",
            "- **Total Stages**:",
            "- **In Progress**:",
            "- **Current Stage**:",
            "- **Next Stage**:",
            "- **Status**:",
            "- **Last Updated**:",
            "- **Revision Count**:",
        ] {
            assert!(state.contains(field), "{field} が要る:\n{state}");
        }
    }

    #[test]
    fn stage_rows_start_unchecked_and_are_grouped_by_phase() {
        let state = compose(&three_stage(), "/ws", "2026-08-31T00:00:00Z");
        assert!(state.contains("### INITIALIZATION PHASE\n- [ ] state-init — EXECUTE\n"));
        assert!(state.contains(
            "### INCEPTION PHASE\n- [ ] domain-design — EXECUTE\n- [ ] contract-design — EXECUTE\n"
        ));
    }

    /// SKIP のステージは行を持たず、`Stages to Skip` にだけ現れる。
    #[test]
    fn skipped_stages_are_listed_but_get_no_row() {
        let intent = intent(vec![
            stage(
                "state-init",
                "0.1",
                PhaseId::Initialization,
                PlanAction::Execute,
            ),
            stage(
                "market-research",
                "1.1",
                PhaseId::Ideation,
                PlanAction::Skip,
            ),
            stage(
                "domain-design",
                "2.1",
                PhaseId::Inception,
                PlanAction::Execute,
            ),
        ]);

        let state = compose(&intent, "/ws", "2026-08-31T00:00:00Z");

        assert!(state.contains("- **Stages to Skip**: market-research"));
        assert!(!state.contains("- [ ] market-research"));
        assert!(state.contains("- **Stages to Execute**: state-init, domain-design"));
        assert!(state.contains("- **Total Stages**: 2"));
    }

    /// 実行対象が 1 つも無いフェーズは Skipped、着地先は Active、initialization は Verified。
    #[test]
    fn phase_progress_reflects_the_resolved_plan() {
        let state = compose(&three_stage(), "/ws", "2026-08-31T00:00:00Z");
        assert!(state.contains("- **Initialization**: Verified"));
        assert!(state.contains("- **Ideation**: Skipped"));
        assert!(state.contains("- **Inception**: Active"));
        assert!(state.contains("- **Construction**: Skipped"));
        assert!(state.contains("- **Operation**: Skipped"));
    }

    /// 走査結果と依頼文は骨格へそのまま載る（誕生の材料が読み面に現れる）。
    #[test]
    fn the_scan_and_request_reach_the_read_face() {
        let state = compose(&three_stage(), "/ws", "2026-08-31T00:00:00Z");
        assert!(state.contains("- **Project**: build the auth service"));
        assert!(state.contains("- **Project Type**: Greenfield"));
        assert!(state.contains("- **Languages**: Rust"));
        assert!(state.contains("- **Build System**: Cargo"));
        assert!(state.contains("- **Project Root**: /ws"));
        assert!(state.contains("- **Scope**: classic"));
    }

    /// 値が空でもコロンまでは書く — 投影はそこを書き換え先として探すからである。
    #[test]
    fn empty_fields_still_get_their_line() {
        let state = compose(&three_stage(), "/ws", "2026-08-31T00:00:00Z");
        assert!(state.contains("- **Worktree Path**:\n"));
        assert!(state.contains("- **Bolt Refs**:\n"));
    }
}
