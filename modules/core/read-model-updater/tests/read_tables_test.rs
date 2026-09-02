//! 構造化投影核 `ReadTables` の契約 — **行の値は集約のクエリの答えの写しである**。
//!
//! 検収の形はどの表も同じである: 同じ履歴をこのテスト自身が再生して集約を起こし、
//! 集約のクエリを直接呼んだ答えと行の値を突き合わせる。期待値をテストに書き下さないのは、
//! 書き下した瞬間に「RMU が判断を持っていないこと」を確かめられなくなるからである
//! (`coding-rules/cqrs-boundaries.md` 規則 3 の 2026-09-02 追記 — 投影核は集約を `replay` で
//! 起こしてクエリメソッドを呼ぶ)。

// テストコードでは unwrap / expect / panic / 添字を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, SecondsFormat, Utc};
use core_command_domain::orchestration::{
    AutonomyMode, Created, Intent, IntentEventId, IntentExecution, IntentExecutionEvent,
    IntentExecutionEventId, IntentExecutionId, IntentId, Recomposed, StageDisplay, StageEntry,
    StartRequest, WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, ConsumeDecl, Defined, DefinitionRevision, ExecutionKind, PhaseId,
    PlanAction, Redefined, ReviewCapValue, ReviewClass, RuleInContext, RuleScope, ScopeGrid,
    ScopeMetadata, SensorRef, SkeletonDefault, StageGraph, StageMode, StageNode, StageNodeBuilder,
    StageNumber, StageSlug, WorkflowDefinition, WorkflowDefinitionEvent, WorkflowDefinitionEventId,
    WorkflowDefinitionId,
};
use core_read_model_updater::orchestration::{
    DefinitionEntry, GlobalSeqNr, JournalBatch, JournalEntry,
};
use core_read_model_updater::read_tables::{ReadTables, ReadTablesError, RequestKind, RunStageRow};

const DEFINITION: &str = "claude";
const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";
const EXECUTION_A: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";
const EXECUTION_B: &str = "0190bbbb-cccc-7ddd-8eee-ffff00001111";

fn at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-09-02T00:00:00Z")
        .expect("固定の ISO 8601 UTC")
        .with_timezone(&Utc)
}

fn slug(value: &str) -> StageSlug {
    StageSlug::parse(value).expect("テストの slug は文法内")
}

fn event_id() -> IntentExecutionEventId {
    IntentExecutionEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002").expect("UUIDv7")
}

fn execution_id() -> IntentExecutionId {
    execution_a()
}

fn intent_event_id() -> IntentEventId {
    IntentEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0001").expect("UUIDv7")
}

fn definition_event_id() -> WorkflowDefinitionEventId {
    WorkflowDefinitionEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0003").expect("UUIDv7")
}

fn definition_id() -> WorkflowDefinitionId {
    WorkflowDefinitionId::parse(DEFINITION).expect("テストの定義 id")
}

fn intent_id() -> IntentId {
    IntentId::parse(INTENT).expect("テストの IntentId は UUIDv7")
}

fn execution_a() -> IntentExecutionId {
    IntentExecutionId::parse(EXECUTION_A).expect("テストの IntentExecutionId は UUIDv7")
}

fn execution_b() -> IntentExecutionId {
    IntentExecutionId::parse(EXECUTION_B).expect("テストの IntentExecutionId は UUIDv7")
}

fn revision(fill: char) -> DefinitionRevision {
    DefinitionRevision::parse(&format!("sha256:{}", fill.to_string().repeat(64)))
        .expect("テストの定義 revision")
}

// ---- 定義ストリームの材料 ----

/// 誕生時点の痩せたグラフ (改訂で差し替えられることを見るための 1 ノード)。
fn genesis_graph() -> StageGraph {
    StageGraph::new(vec![
        StageNodeBuilder::new(
            slug("state-init"),
            StageNumber::parse("0.1").expect("番号"),
            "State Init".to_string(),
            PhaseId::Initialization,
            ExecutionKind::Always,
            StageMode::Inline,
        )
        .build(),
    ])
    .expect("1 ノードのグラフ")
}

/// **任意フィールドを 1 つ残らず埋めた**ノード (全 29 アクセサを行へ写す検収の的)。
fn saturated_node() -> StageNode {
    StageNodeBuilder::new(
        slug("intent-capture"),
        StageNumber::parse("1.1").expect("番号"),
        "Intent Capture".to_string(),
        PhaseId::Ideation,
        ExecutionKind::Conditional,
        StageMode::Mob,
    )
    .condition("brownfield".to_string())
    .lead_agent("aidlc-product-agent".to_string())
    .support_agents(vec!["aidlc-design-agent".to_string()])
    .for_each("unit-of-work".to_string())
    .workspace_requires(true)
    .produces(vec!["intent.md".to_string()])
    .optional_produces(vec!["notes.md".to_string()])
    .produces_kinds(vec![(
        "intent.md".to_string(),
        vec!["markdown".to_string()],
    )])
    .consumes(vec![ConsumeDecl::new(
        "scan.md",
        true,
        Some(BrownfieldGreenfield::Brownfield),
    )])
    .requires_stage(vec![slug("state-init")])
    .sensors(vec!["aidlc-claim-sources".to_string()])
    .scopes(vec!["classic".to_string()])
    .reviewer("aidlc-product-lead-agent".to_string())
    .reviewer_max_iterations(2)
    .review_class(ReviewClass::Adversarial)
    .summary_confirmation("required".to_string())
    .plugin("acme".to_string())
    .enabled(true)
    .inputs("the human's words".to_string())
    .outputs("the captured intent".to_string())
    .rules_in_context(vec![RuleInContext::new("org.md", RuleScope::Org)])
    .sensors_applicable(vec![SensorRef::new(
        "aidlc-claim-sources",
        "sensors/aidlc-claim-sources.md",
        Some("*.md".to_string()),
    )])
    .build()
}

/// 改訂後のグラフ (4 ノード — 実行の計画と同じ slug 列)。
fn revised_graph() -> StageGraph {
    StageGraph::new(vec![
        StageNodeBuilder::new(
            slug("state-init"),
            StageNumber::parse("0.1").expect("番号"),
            "State Init".to_string(),
            PhaseId::Initialization,
            ExecutionKind::Always,
            StageMode::Inline,
        )
        .build(),
        saturated_node(),
        StageNodeBuilder::new(
            slug("scope-definition"),
            StageNumber::parse("1.4").expect("番号"),
            "Scope Definition".to_string(),
            PhaseId::Ideation,
            ExecutionKind::Always,
            StageMode::Subagent,
        )
        .build(),
        StageNodeBuilder::new(
            slug("requirements-analysis"),
            StageNumber::parse("2.1").expect("番号"),
            "Requirements Analysis".to_string(),
            PhaseId::Inception,
            ExecutionKind::Always,
            StageMode::Subagent,
        )
        .build(),
    ])
    .expect("4 ノードのグラフ")
}

/// `classic` だけが列を持つグリッド (`express` は列なし = zero-EXECUTE)。
fn revised_grid() -> ScopeGrid {
    let column: BTreeMap<StageSlug, PlanAction> = [
        (slug("state-init"), PlanAction::Execute),
        (slug("intent-capture"), PlanAction::Execute),
        (slug("scope-definition"), PlanAction::Skip),
        (slug("requirements-analysis"), PlanAction::Execute),
    ]
    .into_iter()
    .collect();
    ScopeGrid::new([("classic".to_string(), column)].into_iter().collect())
}

/// スコープカタログ 2 件。`shared` は両方が宣言する — 辞書順の先着を見るための重なりである。
fn revised_scopes() -> BTreeMap<String, ScopeMetadata> {
    let classic = ScopeMetadata::new("classic")
        .expect("名前あり")
        .with_depth("standard".to_string())
        .with_keywords(vec!["api".to_string(), "shared".to_string()])
        .with_skeleton(SkeletonDefault::Off)
        .with_review_cap(ReviewCapValue::Adversarial)
        .with_freeform_default(true);
    let express = ScopeMetadata::new("express")
        .expect("名前あり")
        .with_keywords(vec!["shared".to_string(), "quick".to_string()]);
    [
        ("classic".to_string(), classic),
        ("express".to_string(), express),
    ]
    .into_iter()
    .collect()
}

fn defined_event() -> WorkflowDefinitionEvent {
    WorkflowDefinitionEvent::Defined(Defined::new(
        definition_event_id(),
        definition_id(),
        revision('0'),
        genesis_graph(),
        ScopeGrid::from_graph(&genesis_graph()),
        BTreeMap::new(),
    ))
}

fn redefined_event() -> WorkflowDefinitionEvent {
    WorkflowDefinitionEvent::Redefined(Redefined::new(
        definition_event_id(),
        definition_id(),
        revision('1'),
        revised_graph(),
        revised_grid(),
        revised_scopes(),
    ))
}

/// テスト自身が再生した定義 (行の検収の的)。
fn replayed_definition() -> WorkflowDefinition {
    let WorkflowDefinitionEvent::Defined(defined) = defined_event() else {
        panic!("誕生記録である");
    };
    WorkflowDefinition::replay(
        WorkflowDefinition::from((defined, at())),
        [(2, at(), redefined_event())],
    )
}

// ---- intent ストリームの材料 ----

fn stages() -> Vec<StageEntry> {
    let display = |number: &str, name: &str, agent: &str| {
        StageDisplay::new(StageNumber::parse(number).expect("番号"), name, agent).expect("単一行")
    };
    vec![
        StageEntry::new(
            slug("state-init"),
            PhaseId::Initialization,
            PlanAction::Execute,
            false,
            display("0.1", "State Init", "orchestrator"),
        ),
        StageEntry::new(
            slug("intent-capture"),
            PhaseId::Ideation,
            PlanAction::Execute,
            false,
            display("1.1", "Intent Capture", "aidlc-product-agent"),
        ),
        StageEntry::new(
            slug("scope-definition"),
            PhaseId::Ideation,
            PlanAction::Execute,
            true,
            display("1.4", "Scope Definition", "aidlc-product-agent"),
        ),
        StageEntry::new(
            slug("requirements-analysis"),
            PhaseId::Inception,
            PlanAction::Execute,
            false,
            display("2.1", "Requirements Analysis", "aidlc-product-agent"),
        ),
    ]
}

fn scan() -> WorkspaceScan {
    WorkspaceScan::new(BrownfieldGreenfield::Brownfield, "Rust", "tokio", "cargo").expect("単一行")
}

fn intent() -> Intent {
    Intent::from((
        Created::new(
            intent_event_id(),
            intent_id(),
            definition_id(),
            revision('1'),
            StartRequest::new("classic", "build the thing")
                .with_depth("standard")
                .with_test_strategy("standard")
                .with_review("adversarial"),
            stages(),
            scan(),
        ),
        at(),
    ))
}

// ---- 実行ストリームの材料 ----

/// 稼働中の実行 (ゲートを開けて承認 → カーソルが前進する)。
fn running_events() -> (IntentExecution, Vec<(usize, IntentExecutionEvent)>) {
    let intent = intent();
    let (mut aggregate, started) = IntentExecution::start(execution_a(), &intent, at());
    let mut events = vec![(aggregate.seq_nr(), started)];
    let opened = aggregate
        .open_gate(&intent, vec!["intent.md".to_string()], at())
        .expect("ゲートは開く");
    events.push((aggregate.seq_nr(), opened));
    let approved = aggregate
        .approve_gate(&intent, Some("ok".to_string()), at())
        .expect("ゲートは承認される");
    events.push((aggregate.seq_nr(), approved));
    (aggregate, events)
}

/// park した実行 (受理不能 — jump は全件 `not-running` で拒否される)。
fn parked_events() -> (IntentExecution, Vec<(usize, IntentExecutionEvent)>) {
    let intent = intent();
    let (mut aggregate, started) = IntentExecution::start(execution_b(), &intent, at());
    let mut events = vec![(aggregate.seq_nr(), started)];
    let switched = aggregate
        .switch_autonomy(&intent, AutonomyMode::Gated, at())
        .expect("gated への切替は受理される");
    events.push((aggregate.seq_nr(), switched));
    let parked = aggregate.park(&intent, at()).expect("park は受理される");
    events.push((aggregate.seq_nr(), parked));
    (aggregate, events)
}

/// 履歴 1 本 — 定義 2 行・intent 1 件・実行 2 本。
fn history() -> JournalBatch {
    let mut executions = Vec::new();
    let mut global = 3_u64;
    for (id, (_, events)) in [
        (execution_a(), running_events()),
        (execution_b(), parked_events()),
    ] {
        for (seq_nr, event) in events {
            executions.push(JournalEntry::new(
                GlobalSeqNr::new(global),
                id.clone(),
                seq_nr,
                at(),
                event,
            ));
            global += 1;
        }
    }
    let definitions = vec![
        DefinitionEntry::new(
            GlobalSeqNr::new(1),
            definition_id(),
            1,
            at(),
            defined_event(),
        ),
        DefinitionEntry::new(
            GlobalSeqNr::new(2),
            definition_id(),
            2,
            at(),
            redefined_event(),
        ),
    ];
    let scanned_to = GlobalSeqNr::new(global - 1);
    JournalBatch::new(executions, vec![intent()], definitions, Some(scanned_to))
}

fn tables() -> ReadTables {
    ReadTables::project(&history()).expect("健全な履歴は投影できる")
}

/// 定義ストリームだけの履歴 (グラフを差し替えて run-stage の列だけを見る)。
///
/// スコープカタログは `revised_scopes()` のまま — 有効 scope は `.md` の存在が権威なので、
/// グラフを差し替えても `classic` / `express` の 2 つで変わらない。
fn tables_with_graph(graph: StageGraph) -> ReadTables {
    let redefined = WorkflowDefinitionEvent::Redefined(Redefined::new(
        definition_event_id(),
        definition_id(),
        revision('2'),
        graph,
        ScopeGrid::new(BTreeMap::new()),
        revised_scopes(),
    ));
    ReadTables::project(&JournalBatch::new(
        Vec::new(),
        Vec::new(),
        vec![
            DefinitionEntry::new(
                GlobalSeqNr::new(1),
                definition_id(),
                1,
                at(),
                defined_event(),
            ),
            DefinitionEntry::new(GlobalSeqNr::new(2), definition_id(), 2, at(), redefined),
        ],
        Some(GlobalSeqNr::new(2)),
    ))
    .expect("定義だけの履歴も投影できる")
}

// ---- 検収 ----

#[test]
fn the_scanned_position_is_the_as_of_of_the_whole_snapshot() {
    let history = history();
    assert_eq!(tables().as_of(), history.scanned_to());
}

#[test]
fn the_definition_row_mirrors_the_replayed_aggregate() {
    let definition = replayed_definition();
    let tables = tables();
    assert_eq!(tables.definitions().len(), 1);
    let row = &tables.definitions()[0];
    assert_eq!(row.definition_id(), definition.id().as_str());
    assert_eq!(row.revision(), definition.revision().as_str());
    assert_eq!(row.stage_count(), definition.graph().len());
    assert_eq!(row.scope_count(), definition.scopes().len());
    // 改訂が畳まれていること (誕生の 1 ノード・0 スコープではない)。
    assert_eq!(row.stage_count(), 4);
    assert_eq!(row.scope_count(), 2);
}

#[test]
fn every_definition_stage_row_mirrors_every_stage_node_accessor() {
    let definition = replayed_definition();
    let tables = tables();
    let rows = tables.definition_stages();
    assert_eq!(rows.len(), definition.graph().len());
    for (position, node) in definition.graph().nodes().iter().enumerate() {
        let row = &rows[position];
        assert_eq!(row.stage_slug(), node.slug().as_str());
        assert_eq!(row.position(), position);
        assert_eq!(row.number(), node.number().as_str());
        assert_eq!(row.name(), node.name());
        assert_eq!(row.phase(), node.phase().as_str());
        assert_eq!(row.execution(), node.execution().as_str());
        assert_eq!(row.condition(), node.condition());
        assert_eq!(row.lead_agent(), node.lead_agent());
        assert_eq!(row.mode(), node.mode().as_str());
        assert_eq!(row.for_each(), node.for_each());
        assert_eq!(row.workspace_requires(), node.workspace_requires());
        assert_eq!(row.reviewer(), node.reviewer());
        assert_eq!(
            row.reviewer_max_iterations(),
            node.reviewer_max_iterations()
        );
        assert_eq!(
            row.review_class(),
            node.review_class().map(ReviewClass::as_str)
        );
        assert_eq!(row.summary_confirmation(), node.summary_confirmation());
        assert_eq!(row.plugin(), node.plugin());
        assert_eq!(row.enabled(), node.enabled());
        assert_eq!(row.inputs(), node.inputs());
        assert_eq!(row.outputs(), node.outputs());
        // ゲート付きの規則はドメインの述語と同じ (initialization だけが非ゲート)。
        assert_eq!(row.gated(), node.phase() != PhaseId::Initialization);
    }

    // 飽和ノードの JSON 列 (配列・構造は ContractCompact の 1 行 JSON)。
    let saturated = &rows[1];
    assert_eq!(saturated.support_agents(), r#"["aidlc-design-agent"]"#);
    assert_eq!(saturated.produces(), r#"["intent.md"]"#);
    assert_eq!(saturated.optional_produces(), r#"["notes.md"]"#);
    assert_eq!(
        saturated.produces_kinds(),
        r#"[{"artifact":"intent.md","kinds":["markdown"]}]"#
    );
    assert_eq!(
        saturated.consumes(),
        r#"[{"artifact":"scan.md","required":true,"conditional_on":"brownfield"}]"#
    );
    assert_eq!(saturated.requires_stage(), r#"["state-init"]"#);
    assert_eq!(saturated.sensors(), r#"["aidlc-claim-sources"]"#);
    assert_eq!(saturated.scopes(), r#"["classic"]"#);
    assert_eq!(
        saturated.rules_in_context(),
        r#"[{"path":"org.md","scope":"org"}]"#
    );
    assert_eq!(
        saturated.sensors_applicable(),
        r#"[{"id":"aidlc-claim-sources","path":"sensors/aidlc-claim-sources.md","matches":"*.md"}]"#
    );
    // 空の配列列も NULL ではなく空配列の JSON である。
    assert_eq!(rows[0].support_agents(), "[]");
}

#[test]
fn definition_scope_rows_mirror_the_catalog_and_the_cost() {
    let definition = replayed_definition();
    let tables = tables();
    let rows = tables.definition_scopes();
    assert_eq!(rows.len(), definition.scopes().len());
    for (row, (name, metadata)) in rows.iter().zip(definition.scopes()) {
        assert_eq!(row.scope(), name.as_str());
        assert_eq!(row.depth(), metadata.depth());
        assert_eq!(
            row.skeleton(),
            metadata.skeleton().map(SkeletonDefault::as_str)
        );
        assert_eq!(
            row.review_cap(),
            metadata.review_cap().map(ReviewCapValue::as_str)
        );
        assert_eq!(row.freeform_default(), metadata.freeform_default());
        assert_eq!(
            row.has_grid_column(),
            definition.grid().contains_scope(name)
        );
        let cost = definition.scope_cost(name);
        assert_eq!(row.cost_total(), cost.as_ref().map(|c| c.total()));
        assert_eq!(row.cost_execute(), cost.as_ref().map(|c| c.execute()));
        assert_eq!(row.cost_gates(), cost.as_ref().map(|c| c.gates()));
        assert_eq!(
            row.cost_per_unit_stages(),
            cost.as_ref().map(|c| c.per_unit_stages())
        );
    }
    assert_eq!(rows[0].keywords(), r#"["api","shared"]"#);
    // 列を持たない有効スコープは費用が無い (NULL)。
    assert!(!rows[1].has_grid_column());
    assert_eq!(rows[1].cost_total(), None);
}

#[test]
fn a_keyword_declared_twice_takes_the_lexicographically_first_scope() {
    let tables = tables();
    let rows = tables.definition_scope_keywords();
    let pairs: Vec<(&str, &str)> = rows
        .iter()
        .map(|row| (row.keyword(), row.scope()))
        .collect();
    assert_eq!(
        pairs,
        [
            ("api", "classic"),
            ("quick", "express"),
            ("shared", "classic")
        ]
    );
    // 逆引きは系譜ごとに分かれる — どの行も自分の定義 id を主キーの一部として運ぶ。
    assert!(
        rows.iter()
            .all(|row| row.definition_id() == definition_id().as_str()),
        "語の逆引きは系譜 ID を伴う"
    );
}

#[test]
fn scope_stage_rows_number_only_the_execute_cells_in_document_order() {
    let definition = replayed_definition();
    let tables = tables();
    let rows = tables.definition_scope_stages();

    let classic: Vec<_> = rows.iter().filter(|row| row.scope() == "classic").collect();
    let expected = definition.stages_in_scope("classic");
    assert_eq!(classic.len(), expected.len());
    let mut order = 0_usize;
    for (row, (slug, _, action)) in classic.iter().zip(&expected) {
        assert_eq!(row.stage_slug(), slug.as_str());
        assert_eq!(row.action(), action.map(PlanAction::as_str));
        if *action == Some(PlanAction::Execute) {
            assert_eq!(row.in_scope_order(), Some(order));
            order += 1;
        } else {
            assert_eq!(row.in_scope_order(), None);
        }
    }
    assert_eq!(order, 3, "classic の EXECUTE は 3 件");

    // 列を持たない有効スコープは全行 action NULL。
    let express: Vec<_> = rows.iter().filter(|row| row.scope() == "express").collect();
    assert_eq!(express.len(), definition.graph().len());
    assert!(express.iter().all(|row| row.action().is_none()));
    assert!(express.iter().all(|row| row.in_scope_order().is_none()));
}

#[test]
fn scope_phase_entry_rows_exist_only_where_the_definition_answers_some() {
    let definition = replayed_definition();
    let tables = tables();
    let rows = tables.definition_scope_phase_entries();
    for row in rows {
        let expected = definition
            .first_in_scope_stage_of_phase(
                PhaseId::parse(row.phase()).expect("行の phase はドメインの綴り"),
                row.scope(),
            )
            .expect("行が在るなら Some");
        assert_eq!(row.first_stage_slug(), expected.slug().as_str());
    }
    let classic: Vec<&str> = rows
        .iter()
        .filter(|row| row.scope() == "classic")
        .map(|row| row.phase())
        .collect();
    assert_eq!(classic, ["initialization", "ideation", "inception"]);
    assert!(rows.iter().all(|row| row.scope() != "express"));
}

#[test]
fn the_intent_row_mirrors_the_intent_aggregate() {
    let intent = intent();
    let tables = tables();
    assert_eq!(tables.intents().len(), 1);
    let row = &tables.intents()[0];
    assert_eq!(row.intent_id(), intent.id().as_str());
    assert_eq!(row.definition_id(), intent.definition_id().as_str());
    assert_eq!(
        row.definition_revision(),
        intent.definition_revision().as_str()
    );
    assert_eq!(row.scope(), intent.scope());
    assert_eq!(row.request(), intent.request());
    assert_eq!(row.depth(), intent.depth());
    assert_eq!(row.test_strategy(), intent.test_strategy());
    assert_eq!(row.review(), intent.review());
    assert_eq!(
        row.created_at(),
        intent
            .created_at()
            .to_rfc3339_opts(SecondsFormat::Secs, true)
    );
    assert_eq!(row.project_type(), intent.scan().project_type());
    assert_eq!(row.project_kind(), intent.scan().project_kind().as_str());
    assert_eq!(row.languages(), intent.scan().languages());
    assert_eq!(row.frameworks(), intent.scan().frameworks());
    assert_eq!(row.build_system(), intent.scan().build_system());
}

#[test]
fn intent_stage_rows_mirror_every_stage_entry() {
    let intent = intent();
    let tables = tables();
    let rows = tables.intent_stages();
    assert_eq!(rows.len(), intent.stages().len());
    for (index, entry) in intent.stages().iter().enumerate() {
        let row = &rows[index];
        assert_eq!(row.intent_id(), intent.id().as_str());
        assert_eq!(row.stage_index(), index);
        assert_eq!(row.slug(), entry.slug().as_str());
        assert_eq!(row.phase(), entry.phase().as_str());
        assert_eq!(row.plan_action(), entry.plan_action().as_str());
        assert_eq!(row.conditional(), entry.is_conditional());
        assert_eq!(row.number(), entry.display().number().as_str());
        assert_eq!(row.name(), entry.display().name());
        assert_eq!(row.lead_agent(), entry.display().lead_agent());
        assert_eq!(row.gated(), entry.is_gated());
    }
}

#[test]
fn execution_rows_mirror_the_replayed_executions() {
    let tables = tables();
    let rows = tables.executions();
    assert_eq!(rows.len(), 2);
    for (aggregate, _) in [running_events(), parked_events()] {
        let row = rows
            .iter()
            .find(|row| row.execution_id() == aggregate.id().as_str())
            .expect("両方の実行が行になる");
        assert_eq!(row.intent_id(), aggregate.intent_id().as_str());
        assert_eq!(row.cursor_index(), Some(aggregate.cursor().to_usize()));
        assert_eq!(
            row.cursor_slug(),
            aggregate
                .stage_keys()
                .get(aggregate.cursor().to_usize())
                .map(|key| key.slug().as_str())
        );
        assert_eq!(
            row.parked_at_index(),
            aggregate.parked_at().map(|stage| stage.to_usize())
        );
        assert_eq!(row.parked_active(), aggregate.parked_active());
        assert_eq!(row.accepts_commands(), aggregate.accepts_commands());
        assert_eq!(row.autonomy(), aggregate.autonomy().as_state_field());
        assert_eq!(row.seq_nr(), aggregate.seq_nr());
        assert_eq!(
            row.last_updated_at(),
            aggregate
                .last_updated_at()
                .to_rfc3339_opts(SecondsFormat::Secs, true)
        );
        assert_eq!(row.state_binding(), aggregate.state_binding().as_str());
    }
    // park はワークフローの状態ではなく実行時の重ね書きである — 2 本の実行が分かれるのは
    // `status` ではなく `parked_active` のほうで、park 中の実行も `running` のままである。
    let statuses: BTreeSet<&str> = rows.iter().map(|row| row.status()).collect();
    assert_eq!(statuses, ["running"].into_iter().collect());
    assert!(rows.iter().any(|row| row.parked_active()));
    assert!(rows.iter().any(|row| !row.parked_active()));
}

#[test]
fn execution_stage_rows_mirror_the_per_stage_queries() {
    let intent = intent();
    let tables = tables();
    let (aggregate, _) = running_events();
    let rows: Vec<_> = tables
        .execution_stages()
        .iter()
        .filter(|row| row.execution_id() == aggregate.id().as_str())
        .collect();
    assert_eq!(rows.len(), aggregate.stage_count());
    for (index, row) in rows.iter().enumerate() {
        let stage = aggregate.stage_index(index).expect("範囲内");
        assert_eq!(row.stage_index(), index);
        assert_eq!(row.slug(), aggregate.stage_keys()[index].slug().as_str());
        assert_eq!(row.phase(), aggregate.stage_keys()[index].phase().as_str());
        assert_eq!(row.approved(), aggregate.approved(stage));
        assert_eq!(row.revision_count(), aggregate.revision_count(stage));
        assert_eq!(
            row.effective_plan(),
            aggregate.effective_plan(stage).map(PlanAction::as_str)
        );
        assert_eq!(row.gated(), aggregate.gated(&intent, stage));
        assert!(row.checkbox().is_some(), "範囲内の checkbox は必ず在る");
    }
    // 誕生 = 初期化完了済み — 索引 0 は completed、承認済みのゲートは approved。
    assert_eq!(rows[0].checkbox(), Some("completed"));
    assert_eq!(rows[1].approved(), Some(true));
}

#[test]
fn next_answer_rows_cover_the_four_request_kinds() {
    let tables = tables();
    for (aggregate, _) in [running_events(), parked_events()] {
        for kind in RequestKind::ALL {
            let row = tables
                .next_answers()
                .iter()
                .find(|row| {
                    row.execution_id() == aggregate.id().as_str()
                        && row.request_kind() == kind.as_str()
                })
                .expect("4 kind すべてに行が在る");
            let decision = aggregate.next_decision(&kind.to_request());
            // 決定の綴りと材料が集約の答えと一致する。
            assert_eq!(row.decision_kind(), decision_kind_of(&decision));
            assert_eq!(row.stage_index(), stage_of(&decision));
            assert_eq!(
                row.stage_slug(),
                stage_of(&decision)
                    .and_then(|index| aggregate.stage_keys().get(index))
                    .map(|key| key.slug().as_str())
            );
        }
    }
    let kinds: BTreeSet<&str> = tables
        .next_answers()
        .iter()
        .map(|row| row.decision_kind())
        .collect();
    assert!(kinds.contains("run-stage"));
    assert!(kinds.contains("parked"));
    assert!(kinds.contains("unpark-then-resume"));
    assert!(kinds.contains("resume-menu"));
    assert!(kinds.contains("new-work-routing"));
}

/// 期待側の綴り (テストが独立に持つ — 実装の写しを使わない)。
const fn decision_kind_of(
    decision: &core_command_domain::orchestration::NextDecision,
) -> &'static str {
    use core_command_domain::orchestration::NextDecision;
    match decision {
        NextDecision::RunStage { .. } => "run-stage",
        NextDecision::Done => "done",
        NextDecision::Parked { .. } => "parked",
        NextDecision::UnparkThenResume => "unpark-then-resume",
        NextDecision::ResumeMenu => "resume-menu",
        NextDecision::NewWorkRouting => "new-work-routing",
        NextDecision::RecoverSkipInconsistency { .. } => "recover-skip-inconsistency",
        NextDecision::InconsistentSkip { .. } => "inconsistent-skip",
    }
}

const fn stage_of(decision: &core_command_domain::orchestration::NextDecision) -> Option<usize> {
    use core_command_domain::orchestration::NextDecision;
    match decision {
        NextDecision::RunStage { stage, .. }
        | NextDecision::Parked { stage }
        | NextDecision::RecoverSkipInconsistency { stage, .. }
        | NextDecision::InconsistentSkip { stage, .. } => Some(stage.to_usize()),
        NextDecision::Done
        | NextDecision::UnparkThenResume
        | NextDecision::ResumeMenu
        | NextDecision::NewWorkRouting => None,
    }
}

/// カーソル上のステージが実効 SKIP に反転した実行 — checkbox 不整合の 2 形。
///
/// この歴史はコマンドだけでは作れない。`recompose` はカーソル以下の反転を `InvalidTarget`
/// で拒み、不変条件 `cursor_in_scope` も実効 SKIP のカーソルを禁じるからである。到達しうる
/// のは **park 中 (受理述語が偽なので `cursor_in_scope` を検査しない) に反転のイベントが
/// 畳まれた歴史**の全再生であり、`next` はそれを読み替えず**不整合として報告する**責務を
/// 持つ (BR3.1 (5) の防御腕)。行にも同じ答えが載らなければならない。
///
/// 誕生直後のカーソルは in-progress なので自力復旧の前提集合に居る (`recover-skip-…`)。
/// ゲートを開いた後は awaiting-approval になり前提集合から外れる (`inconsistent-skip`)。
fn skip_inconsistency_events(
    id: &IntentExecutionId,
    open_the_gate: bool,
) -> (IntentExecution, Vec<(usize, IntentExecutionEvent)>) {
    let intent = intent();
    let (mut aggregate, started) = IntentExecution::start(id.clone(), &intent, at());
    let mut events = vec![(aggregate.seq_nr(), started)];
    if open_the_gate {
        let opened = aggregate
            .open_gate(&intent, vec!["intent.md".to_string()], at())
            .expect("ゲートは開く");
        events.push((aggregate.seq_nr(), opened));
    }
    let switched = aggregate
        .switch_autonomy(&intent, AutonomyMode::Gated, at())
        .expect("gated への切替は受理される");
    events.push((aggregate.seq_nr(), switched));
    let parked = aggregate.park(&intent, at()).expect("park は受理される");
    events.push((aggregate.seq_nr(), parked));

    // park 中にカーソルの slug をそのまま SKIP へ反転させる (改竄された歴史の再現)。
    let cursor_slug = aggregate.stage_keys()[aggregate.cursor().to_usize()]
        .slug()
        .clone();
    let tampering = IntentExecutionEvent::Recomposed(Recomposed::new(
        event_id(),
        execution_id(),
        vec![cursor_slug],
        Vec::new(),
    ));
    let seq_nr = aggregate.seq_nr() + 1;
    aggregate = IntentExecution::replay(aggregate, [(seq_nr, at(), tampering.clone())]);
    events.push((seq_nr, tampering));
    (aggregate, events)
}

/// 不整合 2 形だけを載せた履歴 (定義 1 行・intent 1 件・実行 2 本)。
fn skip_inconsistency_history() -> JournalBatch {
    let mut executions = Vec::new();
    let mut global = 2_u64;
    for (id, open_the_gate) in [(execution_a(), false), (execution_b(), true)] {
        let (_, events) = skip_inconsistency_events(&id, open_the_gate);
        for (seq_nr, event) in events {
            executions.push(JournalEntry::new(
                GlobalSeqNr::new(global),
                id.clone(),
                seq_nr,
                at(),
                event,
            ));
            global += 1;
        }
    }
    let definitions = vec![DefinitionEntry::new(
        GlobalSeqNr::new(1),
        definition_id(),
        1,
        at(),
        defined_event(),
    )];
    JournalBatch::new(
        executions,
        vec![intent()],
        definitions,
        Some(GlobalSeqNr::new(global - 1)),
    )
}

#[test]
fn next_answer_rows_carry_the_observed_checkbox_of_both_skip_inconsistencies() {
    let tables = ReadTables::project(&skip_inconsistency_history()).expect("不整合も投影できる");

    for (id, open_the_gate, expected_kind, expected_checkbox) in [
        (
            execution_a(),
            false,
            "recover-skip-inconsistency",
            "in-progress",
        ),
        (
            execution_b(),
            true,
            "inconsistent-skip",
            "awaiting-approval",
        ),
    ] {
        let (aggregate, _) = skip_inconsistency_events(&id, open_the_gate);
        // 再入の読みだけが park 分岐を素通りして不整合に到達する。
        let decision = aggregate.next_decision(&RequestKind::Reentry.to_request());
        assert_eq!(
            decision_kind_of(&decision),
            expected_kind,
            "集約自身がこの不整合を報告する"
        );

        let row = tables
            .next_answers()
            .iter()
            .find(|row| {
                row.execution_id() == aggregate.id().as_str()
                    && row.request_kind() == RequestKind::Reentry.as_str()
            })
            .expect("再入の行が在る");
        assert_eq!(row.decision_kind(), expected_kind);
        assert_eq!(row.stage_index(), stage_of(&decision));
        assert_eq!(row.stage_slug(), Some("intent-capture"));
        // 不整合 2 形だけが観測 checkbox を運ぶ — ゲートは名指さない。
        assert_eq!(row.checkbox(), Some(expected_checkbox));
        assert_eq!(row.gated(), None);
    }

    // 材料を運ばない分岐は checkbox 列を空のままにする (行に嘘を書かない)。
    let bare = tables
        .next_answers()
        .iter()
        .find(|row| {
            row.execution_id() == execution_a().as_str()
                && row.request_kind() == RequestKind::Bare.as_str()
        })
        .expect("素の要求の行が在る");
    assert_eq!(bare.checkbox(), None);
}

#[test]
fn next_jump_rows_answer_every_target_index() {
    let intent = intent();
    let tables = tables();
    for (aggregate, _) in [running_events(), parked_events()] {
        let rows: Vec<_> = tables
            .next_jumps()
            .iter()
            .filter(|row| row.execution_id() == aggregate.id().as_str())
            .collect();
        assert_eq!(rows.len(), aggregate.stage_count());
        for (index, row) in rows.iter().enumerate() {
            let target = aggregate.stage_index(index).expect("範囲内");
            assert_eq!(row.target_index(), index);
            assert_eq!(
                row.target_slug(),
                aggregate.stage_keys()[index].slug().as_str()
            );
            match aggregate.jump_resolve(&intent, target) {
                Ok(direction) => {
                    assert_eq!(row.refusal(), None);
                    assert_eq!(row.outcome(), direction_of(direction));
                }
                Err(error) => {
                    assert_eq!(row.outcome(), "refused");
                    assert_eq!(row.refusal(), Some(refusal_of(&error)));
                }
            }
        }
    }
    let outcomes: BTreeSet<&str> = tables
        .next_jumps()
        .iter()
        .map(|row| row.outcome())
        .collect();
    assert_eq!(
        outcomes,
        ["forward", "backward", "redo", "refused"]
            .into_iter()
            .collect()
    );
    let refusals: BTreeSet<&str> = tables
        .next_jumps()
        .iter()
        .filter_map(|row| row.refusal())
        .collect();
    assert_eq!(
        refusals,
        ["not-running", "invalid-target"].into_iter().collect()
    );
}

const fn direction_of(
    direction: core_command_domain::orchestration::JumpDirection,
) -> &'static str {
    use core_command_domain::orchestration::JumpDirection;
    match direction {
        JumpDirection::Forward => "forward",
        JumpDirection::Backward => "backward",
        JumpDirection::Redo => "redo",
    }
}

fn refusal_of(error: &core_command_domain::orchestration::CommandError) -> &'static str {
    use core_command_domain::orchestration::CommandError;

    match error {
        CommandError::NotRunning => "not-running",
        CommandError::InvalidTarget(_) => "invalid-target",
        other => panic!("jump_resolve はこの拒否を返さない: {other}"),
    }
}

#[test]
fn next_jump_phase_rows_mirror_first_in_scope_of_phase() {
    let tables = tables();
    let (aggregate, _) = running_events();
    let rows: Vec<_> = tables
        .next_jump_phases()
        .iter()
        .filter(|row| row.execution_id() == aggregate.id().as_str())
        .collect();
    for row in &rows {
        let phase = PhaseId::parse(row.phase()).expect("行の phase はドメインの綴り");
        let expected = aggregate
            .first_in_scope_of_phase(phase)
            .expect("行が在るなら Some");
        assert_eq!(row.target_index(), expected.to_usize());
        assert_eq!(
            row.target_slug(),
            Some(aggregate.stage_keys()[expected.to_usize()].slug().as_str())
        );
    }
    let phases: Vec<&str> = rows.iter().map(|row| row.phase()).collect();
    assert_eq!(phases, ["initialization", "ideation", "inception"]);
}

// ---- run-stage の材料 (定義 × scope × ステージ) ----

/// 行 1 件を主キーで引く。
fn run_stage_row<'a>(tables: &'a ReadTables, scope: &str, stage_slug: &str) -> &'a RunStageRow {
    tables
        .run_stages()
        .iter()
        .find(|row| row.scope() == scope && row.stage_slug() == stage_slug)
        .unwrap_or_else(|| panic!("{scope} × {stage_slug} の行"))
}

#[test]
fn run_stage_rows_cover_every_valid_scope_crossed_with_every_stage() {
    let definition = replayed_definition();
    let tables = tables();
    let scopes = definition.valid_scopes();
    assert_eq!(
        tables.run_stages().len(),
        scopes.len() * definition.graph().len(),
        "行は定義 × 全有効 scope × 全ステージ (実行には依らない)"
    );
    // 列を持たない有効 scope (`express`) にも行が立つ — 有効性の権威はスコープ `.md` で
    // あってグリッド列ではない。
    assert!(scopes.contains(&"express"));
    assert_eq!(
        tables
            .run_stages()
            .iter()
            .filter(|row| row.scope() == "express")
            .count(),
        definition.graph().len()
    );
}

#[test]
fn the_run_stage_row_mirrors_the_definition_node() {
    let definition = replayed_definition();
    let node = definition.graph().get(&slug("intent-capture")).unwrap();
    let tables = tables();
    let row = run_stage_row(&tables, "classic", "intent-capture");

    assert_eq!(row.definition_id(), definition.id().as_str());
    assert_eq!(row.phase(), node.phase().as_str());
    assert_eq!(row.lead_agent(), node.lead_agent());
    assert_eq!(row.mode(), node.mode().as_str());
    assert_eq!(row.support_agents(), r#"["aidlc-design-agent"]"#);
    assert_eq!(
        row.sensors_applicable(),
        r#"[{"id":"aidlc-claim-sources","path":"sensors/aidlc-claim-sources.md","matches":"*.md"}]"#
    );
    // reviewer 3 列は**対で載る** (どちらか欠ければ 3 つとも NULL — クエリ側の組み立て規則)。
    assert_eq!(row.reviewer(), node.reviewer());
    assert_eq!(row.review_class(), Some("adversarial"));
    assert_eq!(row.reviewer_max_iterations(), Some(2));
}

#[test]
fn a_stage_without_a_declared_review_iteration_count_defaults_to_one() {
    // 定義が回数を宣言しないときの既定は 1 (クエリ側 `build_run_stage` と同じ)。
    let node = StageNodeBuilder::new(
        slug("nfr-design"),
        StageNumber::parse("3.4").expect("番号"),
        "NFR Design".to_string(),
        PhaseId::Construction,
        ExecutionKind::Always,
        StageMode::Inline,
    )
    .reviewer("aidlc-architecture-reviewer-agent".to_string())
    .review_class(ReviewClass::Advisory)
    .build();
    assert_eq!(node.reviewer_max_iterations(), None);
    let tables = tables_with_graph(StageGraph::new(vec![node]).expect("1 ノード"));
    let row = run_stage_row(&tables, "classic", "nfr-design");
    assert_eq!(row.reviewer_max_iterations(), Some(1));
}

#[test]
fn a_stage_that_names_a_reviewer_without_a_class_carries_neither() {
    // クエリ側は reviewer と review_class が**両方**揃ったときだけ 3 列を載せる。
    // 片方だけを載せると、読み手は階級なしのレビューを回せると誤解する。
    let node = StageNodeBuilder::new(
        slug("nfr-design"),
        StageNumber::parse("3.4").expect("番号"),
        "NFR Design".to_string(),
        PhaseId::Construction,
        ExecutionKind::Always,
        StageMode::Inline,
    )
    .reviewer("aidlc-architecture-reviewer-agent".to_string())
    .build();
    let tables = tables_with_graph(StageGraph::new(vec![node]).expect("1 ノード"));
    let row = run_stage_row(&tables, "classic", "nfr-design");
    assert_eq!(row.reviewer(), None);
    assert_eq!(row.review_class(), None);
    assert_eq!(row.reviewer_max_iterations(), None);
    assert_eq!(
        row.protocol_modules(),
        r#"["reviewer","construction"]"#,
        "protocol_modules は reviewer の宣言だけを見る (階級は見ない)"
    );
}

#[test]
fn the_relative_paths_follow_the_phase_directory_rule() {
    let tables = tables();
    let row = run_stage_row(&tables, "classic", "intent-capture");
    assert_eq!(row.stage_file_rel(), "ideation/intent-capture.md");
    assert_eq!(row.memory_path_rel(), "ideation/intent-capture/memory.md");
    // consumes は record 直下の成果物名そのもの、produces は当該ステージの部屋の下。
    assert_eq!(row.consumes_rel(), r#"["scan.md"]"#);
    assert_eq!(
        row.produces_rel(),
        r#"["ideation/intent-capture/intent.md"]"#
    );
}

#[test]
fn the_inline_context_paths_follow_the_stage_mode() {
    let tables = tables();
    // Mob = lead だけ。
    assert_eq!(
        run_stage_row(&tables, "classic", "intent-capture").inline_context_paths_rel(),
        r#"["agents/aidlc-product-agent.md"]"#
    );
    // Subagent / Pipeline / AgentTeam = 空。
    assert_eq!(
        run_stage_row(&tables, "classic", "scope-definition").inline_context_paths_rel(),
        "[]"
    );
    // Inline = lead + support。
    let node = StageNodeBuilder::new(
        slug("nfr-design"),
        StageNumber::parse("3.4").expect("番号"),
        "NFR Design".to_string(),
        PhaseId::Construction,
        ExecutionKind::Always,
        StageMode::Inline,
    )
    .lead_agent("aidlc-architect-agent".to_string())
    .support_agents(vec!["aidlc-quality-agent".to_string()])
    .build();
    let inline = tables_with_graph(StageGraph::new(vec![node]).expect("1 ノード"));
    assert_eq!(
        run_stage_row(&inline, "classic", "nfr-design").inline_context_paths_rel(),
        r#"["agents/aidlc-architect-agent.md","agents/aidlc-quality-agent.md"]"#
    );
}

#[test]
fn the_default_gate_is_the_domain_predicate_not_a_rewritten_rule() {
    let definition = replayed_definition();
    let tables = tables();
    for node in definition.graph().nodes() {
        let row = run_stage_row(&tables, "classic", node.slug().as_str());
        let key =
            core_command_domain::orchestration::StageKey::new(node.slug().clone(), node.phase());
        assert_eq!(
            row.gate_default(),
            key.is_gated(),
            "{}",
            node.slug().as_str()
        );
    }
    assert!(!run_stage_row(&tables, "classic", "state-init").gate_default());
    assert!(run_stage_row(&tables, "classic", "intent-capture").gate_default());
}

#[test]
fn the_next_stage_name_is_the_display_name_of_the_next_execute_in_document_order() {
    let tables = tables();
    // classic: state-init(E) → intent-capture(E) → scope-definition(SKIP) →
    // requirements-analysis(E)。SKIP は飛ばす。
    assert_eq!(
        run_stage_row(&tables, "classic", "state-init").next_stage_name(),
        Some("Intent Capture")
    );
    assert_eq!(
        run_stage_row(&tables, "classic", "intent-capture").next_stage_name(),
        Some("Requirements Analysis"),
        "SKIP のステージは次段にならない"
    );
    assert_eq!(
        run_stage_row(&tables, "classic", "scope-definition").next_stage_name(),
        Some("Requirements Analysis"),
        "自分が SKIP でも「後ろの最初の EXECUTE」は答えられる"
    );
    assert_eq!(
        run_stage_row(&tables, "classic", "requirements-analysis").next_stage_name(),
        None,
        "最後の EXECUTE の後には次段が無い"
    );
    // 列を持たない scope は EXECUTE が 1 つも無いので、どのステージにも次段が無い。
    assert_eq!(
        run_stage_row(&tables, "express", "state-init").next_stage_name(),
        None
    );
}

#[test]
fn the_route_digest_is_the_hash_of_the_whole_scope_membership() {
    let definition = replayed_definition();
    let tables = tables();
    // 同じ scope の行は「対象ステージ」だけが違う — 顔ぶれは共有する。
    let a = run_stage_row(&tables, "classic", "state-init").route_digest();
    let b = run_stage_row(&tables, "classic", "intent-capture").route_digest();
    assert_ne!(a, b);
    // scope が違えば顔ぶれが違いうる (grid 列の有無で in-scope の action が変わる)。
    // ここでは stages_in_scope の slug 列そのものが素材であることを、列を持たない scope が
    // 同じ slug 列を返すこと (= 同じ route) で確かめる。
    assert_eq!(
        run_stage_row(&tables, "express", "state-init").route_digest(),
        a,
        "素材は slug 全列であって EXECUTE の絞り込みではない"
    );
    assert_eq!(
        definition
            .stage_route(
                "classic",
                definition.graph().get(&slug("state-init")).unwrap()
            )
            .stages_in_scope()
            .len(),
        4,
        "顔ぶれは EXECUTE で絞らない"
    );
}

#[test]
fn the_directive_digest_moves_with_the_environment_and_not_with_the_scope_alone() {
    let tables = tables();
    let classic = run_stage_row(&tables, "classic", "intent-capture");
    let express = run_stage_row(&tables, "express", "intent-capture");
    assert_eq!(
        classic.stage_file_rel(),
        express.stage_file_rel(),
        "パスは scope に依らない"
    );
    assert_ne!(
        classic.directive_digest(),
        express.directive_digest(),
        "次段が違えば別の directive (classic は次段あり、express は無し)"
    );
    assert_ne!(
        classic.directive_digest(),
        run_stage_row(&tables, "classic", "state-init").directive_digest()
    );
    assert_eq!(classic.directive_digest().len(), 64);
}

// ---- scope-change ----

#[test]
fn scope_change_rows_answer_every_valid_scope_for_every_execution() {
    let definition = replayed_definition();
    let tables = tables();
    let scopes = definition.valid_scopes();
    assert_eq!(
        tables.scope_changes().len(),
        scopes.len() * 2,
        "実行 2 本 × 有効 scope"
    );
    for row in tables.scope_changes() {
        assert!(scopes.contains(&row.scope()), "無効 scope の行は無い");
    }
    let of = |scope: &str| {
        tables
            .scope_changes()
            .iter()
            .find(|row| row.execution_id() == EXECUTION_A && row.scope() == scope)
            .unwrap_or_else(|| panic!("{scope} の行"))
            .kind()
            .to_string()
    };
    assert_eq!(intent().scope(), "classic");
    assert_eq!(of("classic"), "same-as-state", "state の scope と一致");
    assert_eq!(of("express"), "scope-change");
}

#[test]
fn the_execution_row_carries_the_scope_denormalised_from_the_intent() {
    let tables = tables();
    for row in tables.executions() {
        assert_eq!(row.scope(), intent().scope());
    }
}

#[test]
fn a_stream_that_does_not_start_at_its_genesis_is_refused() {
    let (_, events) = running_events();
    let tail: Vec<JournalEntry> = events
        .into_iter()
        .skip(1)
        .enumerate()
        .map(|(offset, (seq_nr, event))| {
            JournalEntry::new(
                GlobalSeqNr::new(offset as u64 + 1),
                execution_a(),
                seq_nr,
                at(),
                event,
            )
        })
        .collect();
    let batch = JournalBatch::new(tail, vec![intent()], Vec::new(), Some(GlobalSeqNr::new(2)));
    assert_eq!(
        ReadTables::project(&batch),
        Err(ReadTablesError::MissingGenesis {
            aggregate_id: EXECUTION_A.to_string()
        })
    );
}

#[test]
fn an_execution_whose_intent_is_absent_is_refused() {
    let (_, events) = running_events();
    let entries: Vec<JournalEntry> = events
        .into_iter()
        .enumerate()
        .map(|(offset, (seq_nr, event))| {
            JournalEntry::new(
                GlobalSeqNr::new(offset as u64 + 1),
                execution_a(),
                seq_nr,
                at(),
                event,
            )
        })
        .collect();
    let batch = JournalBatch::new(entries, Vec::new(), Vec::new(), Some(GlobalSeqNr::new(3)));
    assert_eq!(
        ReadTables::project(&batch),
        Err(ReadTablesError::IntentUnavailable {
            execution_id: EXECUTION_A.to_string(),
            intent_id: INTENT.to_string()
        })
    );
}

#[test]
fn an_empty_history_projects_to_an_empty_snapshot() {
    let tables = ReadTables::project(&JournalBatch::empty()).expect("空も投影できる");
    assert_eq!(tables.as_of(), None);
    assert!(tables.definitions().is_empty());
    assert!(tables.intents().is_empty());
    assert!(tables.executions().is_empty());
    assert!(tables.next_answers().is_empty());
}

#[test]
fn a_definition_stream_without_its_genesis_is_refused() {
    let batch = JournalBatch::new(
        Vec::new(),
        Vec::new(),
        vec![DefinitionEntry::new(
            GlobalSeqNr::new(1),
            definition_id(),
            2,
            at(),
            redefined_event(),
        )],
        Some(GlobalSeqNr::new(1)),
    );
    assert_eq!(
        ReadTables::project(&batch),
        Err(ReadTablesError::MissingGenesis {
            aggregate_id: DEFINITION.to_string()
        })
    );
}
