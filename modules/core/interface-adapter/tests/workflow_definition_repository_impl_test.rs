//! 統合テスト: `WorkflowDefinitionRepositoryImpl` が 12-workflow-definition §4 の失敗態度表を全行満たすこと。
//!
//! 各テストは tempdir に合成 `stage-graph.json` / `scope-grid.json` / `scopes/aidlc-*.md` を
//! 書いて 1 行ずつ検証する:
//! (a) 正常読取と述語の疎通 / (b) graph 欠損 = Err / (c) 不正 JSON = Err /
//! (d) grid 欠損 = 転置導出 (initialization 特例込み) / (e) `.md` あり × 列なし = zero-EXECUTE /
//! (f) 列あり × `.md` なし = `valid_scopes` に不出現 / (g) 未知フィールド入り JSON が読めること。
#![allow(clippy::unwrap_used)]

use core_domain::orchestration::PlanAction;
use core_domain::workflow_definition::{
    BrownfieldGreenfield, PhaseId, ReviewClass, RuleScope, StageMode, StageSlug, WorkflowDefinition,
};
use core_domain::workspace::CheckboxState;
use core_interface_adapter::orchestration::WorkflowDefinitionRepositoryImpl;
use core_use_case::orchestration::{GraphReadError, WorkflowDefinitionRepository};
use std::collections::BTreeMap;
use std::path::PathBuf;
use tempfile::TempDir;

/// 出荷グラフを縮めた 5 ステージ。`bootstrap` / `workspace-init` が initialization 特例の材料、
/// `code-generation` が 28 フィールドのうち任意フィールド群の写像を通す代表ノード。
const GRAPH_JSON: &str = r#"[
  {
    "slug": "bootstrap",
    "number": "0.1",
    "name": "Bootstrap",
    "phase": "initialization",
    "execution": "ALWAYS",
    "condition": "always",
    "lead_agent": "orchestrator",
    "mode": "inline",
    "inputs": "none",
    "outputs": "workspace skeleton",
    "scopes": []
  },
  {
    "slug": "workspace-init",
    "number": "0.2",
    "name": "Workspace Init",
    "phase": "initialization",
    "execution": "ALWAYS",
    "condition": "always",
    "lead_agent": "orchestrator",
    "mode": "inline",
    "inputs": "none",
    "outputs": "workspace",
    "scopes": []
  },
  {
    "slug": "intent-capture",
    "number": "1.1",
    "name": "Intent Capture",
    "phase": "ideation",
    "execution": "ALWAYS",
    "condition": "always",
    "lead_agent": "analyst",
    "mode": "inline",
    "inputs": "user request",
    "outputs": "intent",
    "produces": ["intent"],
    "scopes": ["feature", "bugfix"]
  },
  {
    "slug": "requirements-analysis",
    "number": "2.1",
    "name": "Requirements Analysis",
    "phase": "inception",
    "execution": "ALWAYS",
    "condition": "always",
    "lead_agent": "analyst",
    "mode": "subagent",
    "inputs": "intent",
    "outputs": "requirements",
    "produces": ["requirements"],
    "requires_stage": ["intent-capture"],
    "reviewer": "requirements-reviewer",
    "reviewer_max_iterations": 2,
    "review_class": "advisory",
    "scopes": ["feature"]
  },
  {
    "slug": "code-generation",
    "number": "3.1",
    "name": "Code Generation",
    "phase": "construction",
    "execution": "CONDITIONAL",
    "condition": "when units of work exist",
    "lead_agent": "developer",
    "support_agents": ["tester"],
    "mode": "pipeline",
    "for_each": "unit-of-work",
    "workspace_requires": true,
    "produces": ["code"],
    "optional_produces": ["migration"],
    "produces_kinds": { "code": ["service", "ui"] },
    "consumes": [
      { "artifact": "requirements", "required": true },
      { "artifact": "legacy-survey", "required": false, "conditional_on": "brownfield" }
    ],
    "requires_stage": ["requirements-analysis"],
    "sensors": ["code-quality"],
    "scopes": ["feature"],
    "reviewer": "adversarial-reviewer",
    "reviewer_max_iterations": 3,
    "review_class": "adversarial",
    "summary_confirmation": "required",
    "enabled": true,
    "inputs": "requirements.md",
    "outputs": "source files",
    "rules_in_context": [
      { "path": "aidlc/spaces/default/memory/org.md", "scope": "org" },
      { "path": "aidlc/spaces/default/memory/construction.md", "scope": "phase" }
    ],
    "sensors_applicable": [
      { "id": "code-quality", "path": ".claude/sensors/code-quality.ts", "matches": "**/*.rs" }
    ]
  }
]
"#;

/// `feature` / `bugfix` に加えて、`.md` を持たない `ghost` 列を含む (§4 #6 の材料)。
const GRID_JSON: &str = r#"{
  "bugfix": {
    "stages": {
      "bootstrap": "EXECUTE",
      "workspace-init": "EXECUTE",
      "intent-capture": "EXECUTE",
      "requirements-analysis": "SKIP",
      "code-generation": "SKIP"
    }
  },
  "feature": {
    "stages": {
      "bootstrap": "EXECUTE",
      "workspace-init": "EXECUTE",
      "intent-capture": "EXECUTE",
      "requirements-analysis": "EXECUTE",
      "code-generation": "EXECUTE"
    }
  },
  "ghost": {
    "stages": {
      "bootstrap": "EXECUTE",
      "intent-capture": "EXECUTE"
    }
  }
}
"#;

struct Fixture {
    _dir: TempDir,
    data_dir: PathBuf,
    scopes_dir: PathBuf,
}

impl Fixture {
    /// graph / grid / identity 3 ファイル群を書いた tempdir。`grid` が `None` なら
    /// `scope-grid.json` を置かない (§4 #3 の材料)。
    fn new(graph: Option<&str>, grid: Option<&str>, scopes: &[(&str, &str)]) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("tools/data");
        let scopes_dir = dir.path().join("scopes");
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::create_dir_all(&scopes_dir).unwrap();
        if let Some(graph) = graph {
            std::fs::write(data_dir.join("stage-graph.json"), graph).unwrap();
        }
        if let Some(grid) = grid {
            std::fs::write(data_dir.join("scope-grid.json"), grid).unwrap();
        }
        for (name, content) in scopes {
            std::fs::write(scopes_dir.join(format!("aidlc-{name}.md")), content).unwrap();
        }
        Fixture {
            _dir: dir,
            data_dir,
            scopes_dir,
        }
    }

    fn reader(&self) -> WorkflowDefinitionRepositoryImpl {
        WorkflowDefinitionRepositoryImpl::new(self.data_dir.clone(), self.scopes_dir.clone())
    }

    fn graph_path(&self) -> PathBuf {
        self.data_dir.join("stage-graph.json")
    }
}

/// `.md` は 3 つ: `feature` / `bugfix` はグリッド列を持ち、`express` は持たない (§4 #5 の材料)。
fn scope_files() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "feature",
            "---\nname: feature\ndepth: standard\nkeywords: [api, endpoint]\nskeleton: on\nreview_cap: adversarial\n---\n\n# Feature scope\n",
        ),
        ("bugfix", "---\nname: bugfix\ndepth: light\n---\n"),
        ("express", "---\nname: express\n---\n"),
    ]
}

fn slug(s: &str) -> StageSlug {
    StageSlug::parse(s).unwrap()
}

fn slugs(nodes: &[&core_domain::workflow_definition::StageNode]) -> Vec<String> {
    nodes
        .iter()
        .map(|n| n.slug().as_str().to_string())
        .collect()
}

fn load(fixture: &Fixture) -> WorkflowDefinition {
    fixture.reader().load().unwrap()
}

// ---------------------------------------------------------------------------
// (a) 正常読取と述語の疎通
// ---------------------------------------------------------------------------

#[test]
fn a_full_read_maps_every_field_group_onto_the_domain_model() {
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    let definition = load(&fixture);

    assert_eq!(definition.graph().len(), 5);
    let node = definition.graph().get(&slug("code-generation")).unwrap();
    assert_eq!(node.number().as_str(), "3.1");
    assert_eq!(node.phase(), PhaseId::Construction);
    assert_eq!(node.mode(), StageMode::Pipeline);
    assert_eq!(node.for_each(), Some("unit-of-work"));
    assert!(node.workspace_requires());
    assert_eq!(node.support_agents(), ["tester".to_string()]);
    assert_eq!(node.optional_produces(), ["migration".to_string()]);
    assert_eq!(
        node.produces_kinds().get("code").map(Vec::as_slice),
        Some(["service".to_string(), "ui".to_string()].as_slice())
    );
    assert_eq!(node.consumes().len(), 2);
    assert!(node.consumes()[0].required());
    assert_eq!(node.consumes()[0].conditional_on(), None);
    assert!(!node.consumes()[1].required());
    assert_eq!(
        node.consumes()[1].conditional_on(),
        Some(BrownfieldGreenfield::Brownfield)
    );
    assert_eq!(node.requires_stage(), [slug("requirements-analysis")]);
    assert_eq!(node.reviewer(), Some("adversarial-reviewer"));
    assert_eq!(node.reviewer_max_iterations(), Some(3));
    assert_eq!(node.review_class(), Some(ReviewClass::Adversarial));
    assert_eq!(node.summary_confirmation(), Some("required"));
    assert_eq!(node.enabled(), Some(true));
    assert!(node.is_enabled());
    assert_eq!(node.inputs(), "requirements.md");
    assert_eq!(node.outputs(), "source files");

    // F4: オブジェクト配列のまま保持し、directive 射影は別 API で取り出す。
    assert_eq!(node.rules_in_context().len(), 2);
    assert_eq!(node.rules_in_context()[0].scope(), RuleScope::Org);
    assert_eq!(node.rules_in_context()[1].scope(), RuleScope::Phase);
    assert_eq!(
        node.rule_paths(),
        [
            "aidlc/spaces/default/memory/org.md",
            "aidlc/spaces/default/memory/construction.md"
        ]
    );
    assert_eq!(node.sensors_applicable().len(), 1);
    assert_eq!(node.sensors_applicable()[0].matches(), Some("**/*.rs"));
    assert_eq!(node.sensor_ids(), ["code-quality"]);

    // キー不在の任意フィールドは既定値のまま。
    let bootstrap = definition.graph().get(&slug("bootstrap")).unwrap();
    assert_eq!(bootstrap.enabled(), None);
    assert!(bootstrap.is_enabled());
    assert_eq!(bootstrap.reviewer(), None);
    assert!(bootstrap.produces().is_empty());
    assert!(!bootstrap.workspace_requires());

    // scope identity の frontmatter。
    let feature = definition.scope_metadata("feature").unwrap();
    assert_eq!(feature.depth(), Some("standard"));
    assert_eq!(
        feature.keywords(),
        ["api".to_string(), "endpoint".to_string()]
    );
    assert!(feature.skeleton().is_some());
    assert!(feature.review_cap().is_some());
    assert!(!feature.freeform_default());
}

#[test]
fn a_full_read_wires_up_the_five_predicates() {
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    let definition = load(&fixture);

    // valid_scopes の権威は `.md` の存在。グリッド専用の `ghost` は現れない。
    assert_eq!(definition.valid_scopes(), ["bugfix", "express", "feature"]);

    // subgraph_for_scope は数値順。
    assert_eq!(
        slugs(&definition.subgraph_for_scope("feature").unwrap()),
        [
            "bootstrap",
            "workspace-init",
            "intent-capture",
            "requirements-analysis",
            "code-generation"
        ]
    );
    assert_eq!(
        slugs(&definition.subgraph_for_scope("bugfix").unwrap()),
        ["bootstrap", "workspace-init", "intent-capture"]
    );

    // first_in_scope_stage_of_phase はハードコードではなく subgraph からの導出。
    assert_eq!(
        definition
            .first_in_scope_stage_of_phase(PhaseId::Construction, "feature")
            .map(|n| n.slug().as_str()),
        Some("code-generation")
    );
    assert_eq!(
        definition.first_in_scope_stage_of_phase(PhaseId::Construction, "bugfix"),
        None
    );

    // next_in_scope_stage は文書順で前進走査し、completed / skipped を読み飛ばす。
    let checkboxes = BTreeMap::new();
    let suffixes = BTreeMap::new();
    assert_eq!(
        definition
            .next_in_scope_stage(&slug("intent-capture"), "feature", &checkboxes, &suffixes)
            .map(|n| n.slug().as_str()),
        Some("requirements-analysis")
    );
    let mut checkboxes = BTreeMap::new();
    checkboxes.insert(slug("requirements-analysis"), CheckboxState::Completed);
    assert_eq!(
        definition
            .next_in_scope_stage(&slug("intent-capture"), "feature", &checkboxes, &suffixes)
            .map(|n| n.slug().as_str()),
        Some("code-generation")
    );

    // stages_in_scope は全ステージ分の 3 値を文書順で返す。
    let rows = definition.stages_in_scope("bugfix");
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].1, PhaseId::Initialization);
    assert_eq!(rows[0].2, Some(PlanAction::Execute));
    assert_eq!(rows[4].2, Some(PlanAction::Skip));

    // effective_plan_action: recompose サフィックスが静的グリッドに勝つ。
    let mut suffixes = BTreeMap::new();
    suffixes.insert(slug("code-generation"), PlanAction::Skip);
    assert_eq!(
        definition.effective_plan_action(&suffixes, "feature", &slug("code-generation")),
        Some(PlanAction::Skip)
    );
    // グリッド列に載っていない slug は 3 値の None (SKIP に畳まない)。
    assert_eq!(
        definition.effective_plan_action(&BTreeMap::new(), "ghost", &slug("code-generation")),
        None
    );
}

// ---------------------------------------------------------------------------
// (b) graph 欠損 = Err
// ---------------------------------------------------------------------------

#[test]
fn b_a_missing_stage_graph_is_fatal() {
    let fixture = Fixture::new(None, Some(GRID_JSON), &scope_files());
    let error = fixture.reader().load().unwrap_err();
    let GraphReadError::NotReadable {
        path, env_override, ..
    } = error
    else {
        panic!("expected NotReadable, got {error:?}");
    };
    assert_eq!(path, fixture.graph_path().display().to_string());
    assert!(!env_override);
}

#[test]
fn b_the_env_override_flag_follows_the_injected_path() {
    let fixture = Fixture::new(None, Some(GRID_JSON), &scope_files());
    let missing = fixture.data_dir.join("pinned-graph.json");
    let reader = fixture.reader().with_stage_graph_override(missing.clone());
    let error = reader.load().unwrap_err();
    let GraphReadError::NotReadable {
        path, env_override, ..
    } = error
    else {
        panic!("expected NotReadable, got {error:?}");
    };
    assert_eq!(path, missing.display().to_string());
    assert!(env_override);
}

// ---------------------------------------------------------------------------
// (c) 不正 JSON = Err (欠損とは別文言)
// ---------------------------------------------------------------------------

#[test]
fn c_a_malformed_stage_graph_is_fatal_under_a_different_variant() {
    let fixture = Fixture::new(Some("[ { \"slug\": "), Some(GRID_JSON), &scope_files());
    let error = fixture.reader().load().unwrap_err();
    assert!(
        matches!(error, GraphReadError::InvalidJson { ref path, .. } if *path == fixture.graph_path().display().to_string()),
        "expected InvalidJson, got {error:?}"
    );
}

#[test]
fn c_a_stage_graph_object_root_is_rejected_because_the_root_is_an_array() {
    let fixture = Fixture::new(Some("{\"stages\": []}"), Some(GRID_JSON), &scope_files());
    let error = fixture.reader().load().unwrap_err();
    assert!(matches!(error, GraphReadError::InvalidJson { .. }));
}

// ---------------------------------------------------------------------------
// (d) grid 欠損 = 転置導出 (initialization 特例込み)
// ---------------------------------------------------------------------------

#[test]
fn d_a_missing_scope_grid_falls_back_to_the_transpose_instead_of_failing() {
    let fixture = Fixture::new(Some(GRAPH_JSON), None, &scope_files());
    let definition = load(&fixture);

    // 列はノードが宣言したスコープ名の和集合。`ghost` はグリッド由来なので消える。
    assert_eq!(definition.grid().scope_names(), ["bugfix", "feature"]);
    // 有効スコープは `.md` 側の権威のまま。
    assert_eq!(definition.valid_scopes(), ["bugfix", "express", "feature"]);

    // initialization 特例: frontmatter に関係なく全列で EXECUTE。
    for scope in ["bugfix", "feature"] {
        for init in ["bootstrap", "workspace-init"] {
            assert_eq!(
                definition.grid().action(scope, &slug(init)),
                Some(PlanAction::Execute),
                "{scope}/{init}"
            );
        }
    }
    assert_eq!(
        definition.grid().action("bugfix", &slug("code-generation")),
        Some(PlanAction::Skip)
    );
    assert_eq!(
        slugs(&definition.subgraph_for_scope("bugfix").unwrap()),
        ["bootstrap", "workspace-init", "intent-capture"]
    );
}

#[test]
fn d_an_unreadable_scope_grid_falls_back_the_same_way() {
    let fixture = Fixture::new(Some(GRAPH_JSON), Some("{ not json"), &scope_files());
    let definition = load(&fixture);
    assert_eq!(definition.grid().scope_names(), ["bugfix", "feature"]);
    assert_eq!(
        definition
            .grid()
            .action("feature", &slug("code-generation")),
        Some(PlanAction::Execute)
    );
}

// ---------------------------------------------------------------------------
// (e) `.md` あり × 列なし = zero-EXECUTE な正当スコープ
// ---------------------------------------------------------------------------

#[test]
fn e_an_identity_file_without_a_grid_column_is_a_zero_execute_scope_not_an_unknown_one() {
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    let definition = load(&fixture);

    assert!(definition.is_valid_scope("express"));
    assert!(!definition.grid().contains_scope("express"));
    // 拒否ではなく空。
    assert!(definition.subgraph_for_scope("express").unwrap().is_empty());
    assert_eq!(
        definition.next_in_scope_stage(
            &slug("bootstrap"),
            "express",
            &BTreeMap::new(),
            &BTreeMap::new()
        ),
        None
    );
    // 全ステージ分の行は返るが、action はすべて 3 値の None。
    let rows = definition.stages_in_scope("express");
    assert_eq!(rows.len(), 5);
    assert!(rows.iter().all(|(_, _, action)| action.is_none()));
}

// ---------------------------------------------------------------------------
// (f) 列あり × `.md` なし = ランタイムから不可視
// ---------------------------------------------------------------------------

#[test]
fn f_a_grid_column_without_an_identity_file_is_invisible_to_the_runtime() {
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    let definition = load(&fixture);

    // 列は読めているが、有効スコープではない。
    assert!(definition.grid().contains_scope("ghost"));
    assert!(!definition.is_valid_scope("ghost"));
    assert!(!definition.valid_scopes().contains(&"ghost"));

    // 未知スコープの非対称: subgraph だけが Err、他は None / 空。
    let error = definition.subgraph_for_scope("ghost").unwrap_err();
    assert_eq!(error.scope(), "ghost");
    assert_eq!(error.valid_scopes(), ["bugfix", "express", "feature"]);
    assert_eq!(
        definition.first_in_scope_stage_of_phase(PhaseId::Ideation, "ghost"),
        None
    );
    assert_eq!(
        definition.next_in_scope_stage(
            &slug("bootstrap"),
            "ghost",
            &BTreeMap::new(),
            &BTreeMap::new()
        ),
        None
    );
    assert!(definition.stages_in_scope("ghost").is_empty());
}

// ---------------------------------------------------------------------------
// (g) 未知フィールドの許容 (F1)
// ---------------------------------------------------------------------------

#[test]
fn g_unknown_fields_are_ignored_so_future_versions_and_plugins_stay_readable() {
    let graph = r#"[
      {
        "slug": "acme-scan",
        "number": "0.1",
        "name": "Acme Scan",
        "phase": "initialization",
        "execution": "ALWAYS",
        "condition": "always",
        "lead_agent": "orchestrator",
        "mode": "inline",
        "inputs": "none",
        "outputs": "report",
        "plugin": "acme",
        "enabled": false,
        "scopes": ["feature"],
        "when": "producer-in-plan",
        "required_sections": ["summary"],
        "plugin_source": "acme@1.2.3",
        "bundle": { "kind": "extra", "weight": 3 },
        "category": ["a", "b"]
      }
    ]
    "#;
    let fixture = Fixture::new(
        Some(graph),
        None,
        &[(
            "feature",
            "---\nname: feature\nunknown_key: whatever\n---\n",
        )],
    );
    let definition = load(&fixture);

    let node = definition.graph().get(&slug("acme-scan")).unwrap();
    assert_eq!(node.plugin(), Some("acme"));
    // `enabled: false` のノードも読取モデルからは除外しない (判断は呼出側)。
    assert_eq!(node.enabled(), Some(false));
    assert!(!node.is_enabled());
    assert_eq!(definition.valid_scopes(), ["feature"]);
}

// ---------------------------------------------------------------------------
// 追加: 文書順の保持 (F2 — 2 経路の使い分けを潰さない)
// ---------------------------------------------------------------------------

#[test]
fn the_reader_preserves_document_order_and_keeps_the_two_ordering_paths_distinct() {
    // 文書順が数値順と食い違う手編集グラフ。読込時に数値順へ正規化してはならない。
    let graph = r#"[
      { "slug": "later", "number": "1.10", "name": "Later", "phase": "ideation",
        "execution": "ALWAYS", "condition": "always", "lead_agent": "a", "mode": "inline",
        "inputs": "i", "outputs": "o", "scopes": ["feature"] },
      { "slug": "earlier", "number": "1.9", "name": "Earlier", "phase": "ideation",
        "execution": "ALWAYS", "condition": "always", "lead_agent": "a", "mode": "inline",
        "inputs": "i", "outputs": "o", "scopes": ["feature"] }
    ]
    "#;
    let fixture = Fixture::new(
        Some(graph),
        None,
        &[("feature", "---\nname: feature\n---\n")],
    );
    let definition = load(&fixture);

    // 文書順はディスクの配列順そのまま。
    let document_order: Vec<&str> = definition
        .graph()
        .nodes()
        .iter()
        .map(|n| n.slug().as_str())
        .collect();
    assert_eq!(document_order, ["later", "earlier"]);

    // subgraph_for_scope だけが数値順に並べ替える ("1.10" > "1.9")。
    assert_eq!(
        slugs(&definition.subgraph_for_scope("feature").unwrap()),
        ["earlier", "later"]
    );

    // next_in_scope_stage は文書順走査なので "later" の次は "earlier"。
    assert_eq!(
        definition
            .next_in_scope_stage(
                &slug("later"),
                "feature",
                &BTreeMap::new(),
                &BTreeMap::new()
            )
            .map(|n| n.slug().as_str()),
        Some("earlier")
    );
}

// ---------------------------------------------------------------------------
// 追加: scope identity ファイルの拒否文言
// ---------------------------------------------------------------------------

#[test]
fn an_invalid_skeleton_value_is_rejected_with_the_verbatim_wording() {
    let fixture = Fixture::new(
        Some(GRAPH_JSON),
        Some(GRID_JSON),
        &[("feature", "---\nname: feature\nskeleton: enabled\n---\n")],
    );
    let error = fixture.reader().load().unwrap_err();
    let GraphReadError::ScopeFile { message } = error else {
        panic!("expected ScopeFile, got {error:?}");
    };
    let path = fixture.scopes_dir.join("aidlc-feature.md");
    assert_eq!(
        message,
        format!(
            "Scope file {} has invalid skeleton value \"enabled\". Expected \"on\" or \"off\".",
            path.display()
        )
    );
}

#[test]
fn a_scope_file_without_a_name_is_rejected() {
    let fixture = Fixture::new(
        Some(GRAPH_JSON),
        Some(GRID_JSON),
        &[("feature", "---\ndepth: standard\n---\n")],
    );
    let error = fixture.reader().load().unwrap_err();
    let GraphReadError::ScopeFile { message } = error else {
        panic!("expected ScopeFile, got {error:?}");
    };
    assert!(
        message.ends_with("missing required frontmatter: name"),
        "{message}"
    );
}

#[test]
fn two_identity_files_declaring_the_same_name_are_fatal() {
    let fixture = Fixture::new(
        Some(GRAPH_JSON),
        Some(GRID_JSON),
        &[
            ("feature", "---\nname: feature\n---\n"),
            ("feature-alias", "---\nname: feature\n---\n"),
        ],
    );
    let error = fixture.reader().load().unwrap_err();
    // upstream 逐語 (aidlc-lib.ts:8666-8668 @3c3146cf) の形を pin する
    assert!(
        matches!(error, GraphReadError::ScopeFile { ref message }
            if message.starts_with("Duplicate scope name \"feature\" in ")
                && message.contains(": already declared in ")
                && message.ends_with(". Rename one of them.")),
        "{error:?}"
    );
}

#[test]
fn a_missing_scopes_directory_yields_an_empty_catalog_rather_than_a_failure() {
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &[]);
    let reader = WorkflowDefinitionRepositoryImpl::new(
        fixture.data_dir.clone(),
        fixture.scopes_dir.join("does-not-exist"),
    );
    let definition = reader.load().unwrap();
    assert!(definition.valid_scopes().is_empty());
    // グリッド列は読めているが、権威が無いので全スコープが未知になる。
    assert!(definition.grid().contains_scope("feature"));
    assert!(definition.subgraph_for_scope("feature").is_err());
}

// ---------------------------------------------------------------------------
// 追加: ドメイン型へ写せない値
// ---------------------------------------------------------------------------

#[test]
fn an_unknown_phase_is_reported_as_malformed_rather_than_falling_through() {
    let graph = r#"[
      { "slug": "s", "number": "1.1", "name": "S", "phase": "delivery",
        "execution": "ALWAYS", "condition": "c", "lead_agent": "a", "mode": "inline",
        "inputs": "i", "outputs": "o", "scopes": [] }
    ]
    "#;
    let fixture = Fixture::new(Some(graph), None, &[]);
    let error = fixture.reader().load().unwrap_err();
    assert!(
        matches!(error, GraphReadError::Malformed { ref message } if message.contains("unknown phase")),
        "{error:?}"
    );
}

#[test]
fn the_reserved_agent_team_mode_is_carried_through_instead_of_being_defaulted() {
    let graph = r#"[
      { "slug": "s", "number": "1.1", "name": "S", "phase": "ideation",
        "execution": "ALWAYS", "condition": "c", "lead_agent": "a", "mode": "agent-team",
        "inputs": "i", "outputs": "o", "scopes": [] }
    ]
    "#;
    let fixture = Fixture::new(Some(graph), None, &[]);
    let definition = load(&fixture);
    let node = definition.graph().get(&slug("s")).unwrap();
    assert_eq!(node.mode(), StageMode::AgentTeam);
    assert!(node.mode().is_reserved());
}

#[test]
fn grid_cells_that_cannot_be_represented_collapse_to_the_third_value_not_to_skip() {
    let grid = r#"{
      "feature": {
        "stages": {
          "intent-capture": "EXECUTE",
          "requirements-analysis": "MAYBE",
          "Not A Slug": "EXECUTE"
        }
      }
    }
    "#;
    let fixture = Fixture::new(
        Some(GRAPH_JSON),
        Some(grid),
        &[("feature", "---\nname: feature\n---\n")],
    );
    let definition = load(&fixture);
    assert_eq!(
        definition.grid().action("feature", &slug("intent-capture")),
        Some(PlanAction::Execute)
    );
    assert_eq!(
        definition
            .grid()
            .action("feature", &slug("requirements-analysis")),
        None
    );
    assert_eq!(
        slugs(&definition.subgraph_for_scope("feature").unwrap()),
        ["intent-capture"]
    );
}
