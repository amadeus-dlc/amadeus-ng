//! 統合テスト: クエリ側の DAO 実装 (`WorkflowDefinitionDaoImpl`) と純 parse の合成が
//! 12-workflow-definition §4 の失敗態度表を全行満たすこと。
//!
//! 読み終えた先は**クエリモデル** (`core_query_use_case::orchestration` の `~View` 型) であり、
//! コマンド側の集約ではない (`coding-rules/cqrs-boundaries.md` 規則 6)。
//!
//! 各テストは tempdir に合成 `stage-graph.json` / `scope-grid.json` / `scopes/aidlc-*.md` を
//! 書いて 1 行ずつ検証する:
//! (a) 正常読取と述語の疎通 / (b) graph 欠損 = Err / (c) 不正 JSON = Err /
//! (d) grid 欠損 = 転置導出 (initialization 特例込み) / (e) `.md` あり × 列なし = zero-EXECUTE /
//! (f) 列あり × `.md` なし = `valid_scopes` に不出現 / (g) 未知フィールド入り JSON が読めること。
// indexing_slicing (固定長フィクスチャの添字参照) と panic (想定外ケースの即時失敗という
// 検証用途) も unwrap_used と同じ理由で file 単位の allow が要る。
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use core_query_interface_adapter::{DefinitionPaths, WorkflowDefinitionDaoImpl};
use core_query_use_case::orchestration::{
    BrownfieldGreenfieldView, DefinitionIdView, DefinitionView, PhaseView, PlanActionView,
    ReviewClassView, RuleScopeView, StageModeView, StageSlugView, WorkflowDefinitionDao,
    WorkflowDefinitionReadError,
};
use std::path::PathBuf;
use tempfile::TempDir;

/// 3 入力を DAO 経由で読む (ポート契約そのものを通す)。
fn read(paths: &DefinitionPaths) -> Result<DefinitionView, WorkflowDefinitionReadError> {
    WorkflowDefinitionDaoImpl::new(paths.clone()).find()
}

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
    /// graph / grid / identity 3 ファイル群と `harness.json` を書いた tempdir。
    /// `grid` が `None` なら `scope-grid.json` を置かない (§4 #3 の材料)。
    /// `harness.json` は既定の `{"name":"claude"}` を置く。
    fn new(graph: Option<&str>, grid: Option<&str>, scopes: &[(&str, &str)]) -> Fixture {
        Fixture::with_harness(graph, grid, scopes, Some(DEFAULT_HARNESS_JSON))
    }

    /// `harness.json` の内容まで指定する版 (`None` ならファイルを置かない)。
    fn with_harness(
        graph: Option<&str>,
        grid: Option<&str>,
        scopes: &[(&str, &str)],
        harness: Option<&str>,
    ) -> Fixture {
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
        if let Some(harness) = harness {
            std::fs::write(data_dir.join("harness.json"), harness).unwrap();
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

    fn reader(&self) -> DefinitionPaths {
        DefinitionPaths::new(self.data_dir.clone(), self.scopes_dir.clone())
    }

    fn graph_path(&self) -> PathBuf {
        self.data_dir.join("stage-graph.json")
    }

    fn harness_path(&self) -> PathBuf {
        self.data_dir.join("harness.json")
    }

    /// `scope-grid.json` を書き換える (revision の変化を見るため)。
    fn rewrite_grid(&self, grid: &str) {
        std::fs::write(self.data_dir.join("scope-grid.json"), grid).unwrap();
    }
}

/// 出荷ハーネスの `harness.json` と同じ形 (upstream 実バイトは
/// `tests/golden/upstream-3c3146cf/harness.json`)。
const DEFAULT_HARNESS_JSON: &str = r#"{
  "name": "claude",
  "harnessDir": ".claude",
  "rulesSubdir": "rules"
}
"#;

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

fn slug(s: &str) -> StageSlugView {
    StageSlugView::parse(s).unwrap()
}

fn slugs(nodes: &[&core_query_use_case::orchestration::StageView]) -> Vec<String> {
    nodes
        .iter()
        .map(|n| n.slug().as_str().to_string())
        .collect()
}

fn definition_id(value: &str) -> DefinitionIdView {
    DefinitionIdView::parse(value).unwrap()
}

fn load_definition(fixture: &Fixture) -> DefinitionView {
    read(&fixture.reader()).unwrap()
}

// ---------------------------------------------------------------------------
// (a) 正常読取と述語の疎通
// ---------------------------------------------------------------------------

#[test]
fn a_full_read_maps_every_field_group_onto_the_query_model() {
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    let definition = load_definition(&fixture);

    assert_eq!(definition.graph().len(), 5);
    let node = definition.graph().get(&slug("code-generation")).unwrap();
    assert_eq!(node.number().as_str(), "3.1");
    assert_eq!(node.phase(), PhaseView::Construction);
    assert_eq!(node.mode(), StageModeView::Pipeline);
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
        Some(BrownfieldGreenfieldView::Brownfield)
    );
    assert_eq!(node.requires_stage(), [slug("requirements-analysis")]);
    assert_eq!(node.reviewer(), Some("adversarial-reviewer"));
    assert_eq!(node.reviewer_max_iterations(), Some(3));
    assert_eq!(node.review_class(), Some(ReviewClassView::Adversarial));
    assert_eq!(node.summary_confirmation(), Some("required"));
    assert_eq!(node.enabled(), Some(true));
    assert!(node.is_enabled());
    assert_eq!(node.inputs(), "requirements.md");
    assert_eq!(node.outputs(), "source files");

    // F4: オブジェクト配列のまま保持し、directive 射影は別 API で取り出す。
    assert_eq!(node.rules_in_context().len(), 2);
    assert_eq!(node.rules_in_context()[0].scope(), RuleScopeView::Org);
    assert_eq!(node.rules_in_context()[1].scope(), RuleScopeView::Phase);
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
    let definition = load_definition(&fixture);

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
            .first_in_scope_stage_of_phase(PhaseView::Construction, "feature")
            .map(|n| n.slug().as_str()),
        Some("code-generation")
    );
    assert_eq!(
        definition.first_in_scope_stage_of_phase(PhaseView::Construction, "bugfix"),
        None
    );

    // stages_in_scope は全ステージ分の 3 値を文書順で返す。
    let rows = definition.stages_in_scope("feature");
    assert_eq!(rows.len(), 5);
    let listed: Vec<&str> = rows.iter().map(|(s, _, _)| s.as_str()).collect();
    assert_eq!(
        listed,
        [
            "bootstrap",
            "workspace-init",
            "intent-capture",
            "requirements-analysis",
            "code-generation"
        ]
    );
    let rows = definition.stages_in_scope("bugfix");
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].1, PhaseView::Initialization);
    assert_eq!(rows[0].2, Some(PlanActionView::Execute));
    assert_eq!(rows[4].2, Some(PlanActionView::Skip));

    // 静的グリッドの照会は 3 値。実効プランの合成 (recompose オーバレイとの重ね合わせ) は
    // FR8.4 で 実行側 (コマンド側の集約) の仕事なのでここには無い。
    assert_eq!(
        definition
            .grid()
            .action("feature", &slug("code-generation")),
        Some(PlanActionView::Execute)
    );
    // グリッド列に載っていない slug は 3 値の None (SKIP に畳まない)。
    assert_eq!(
        definition.grid().action("ghost", &slug("code-generation")),
        None
    );
}

// ---------------------------------------------------------------------------
// (b) graph 欠損 = Err
// ---------------------------------------------------------------------------

#[test]
fn b_a_missing_stage_graph_is_fatal() {
    let fixture = Fixture::new(None, Some(GRID_JSON), &scope_files());
    let error = read(&fixture.reader()).unwrap_err();
    let WorkflowDefinitionReadError::NotReadable {
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
    let error = read(&reader).unwrap_err();
    let WorkflowDefinitionReadError::NotReadable {
        path, env_override, ..
    } = error
    else {
        panic!("expected NotReadable, got {error:?}");
    };
    assert_eq!(path, missing.display().to_string());
    assert!(env_override);
}

#[test]
fn b_the_scope_grid_override_points_the_read_at_the_injected_path() {
    // AIDLC_SCOPE_GRID 相当 — 既定の場所に grid が無くても、注入されたパスの grid が読まれる
    // (欠損は fatal ではないので、オーバライドが効いていることは列の実在で観測する)。
    let fixture = Fixture::new(Some(GRAPH_JSON), None, &scope_files());
    let pinned = fixture.data_dir.join("pinned-grid.json");
    std::fs::write(&pinned, GRID_JSON).unwrap();
    let reader = fixture.reader().with_scope_grid_override(pinned);
    let definition = read(&reader).unwrap();
    // 判別はグラフからの転置導出では**現れ得ない**セルで行う — `requirements-analysis` の
    // scopes は ["feature"] なので、導出 grid の bugfix 列にはこのセルが無い。注入 grid
    // だけが SKIP を持つ (CodeRabbit 指摘: feature 列の実在だけでは導出と区別できない)。
    let column = definition.grid().column("bugfix").expect("bugfix 列");
    assert_eq!(
        column.get(&slug("requirements-analysis")),
        Some(&PlanActionView::Skip),
        "注入した grid のセルが読まれている"
    );
}

// ---------------------------------------------------------------------------
// (c) 不正 JSON = Err (欠損とは別文言)
// ---------------------------------------------------------------------------

#[test]
fn c_a_malformed_stage_graph_is_fatal_under_a_different_variant() {
    let fixture = Fixture::new(Some("[ { \"slug\": "), Some(GRID_JSON), &scope_files());
    let error = read(&fixture.reader()).unwrap_err();
    assert!(
        matches!(error, WorkflowDefinitionReadError::InvalidJson { ref path, .. } if *path == fixture.graph_path().display().to_string()),
        "expected InvalidJson, got {error:?}"
    );
}

#[test]
fn c_a_stage_graph_object_root_is_rejected_because_the_root_is_an_array() {
    let fixture = Fixture::new(Some("{\"stages\": []}"), Some(GRID_JSON), &scope_files());
    let error = read(&fixture.reader()).unwrap_err();
    assert!(matches!(
        error,
        WorkflowDefinitionReadError::InvalidJson { .. }
    ));
}

// ---------------------------------------------------------------------------
// (d) grid 欠損 = 転置導出 (initialization 特例込み)
// ---------------------------------------------------------------------------

#[test]
fn d_a_missing_scope_grid_falls_back_to_the_transpose_instead_of_failing() {
    let fixture = Fixture::new(Some(GRAPH_JSON), None, &scope_files());
    let definition = load_definition(&fixture);

    // 列はノードが宣言したスコープ名の和集合。`ghost` はグリッド由来なので消える。
    assert_eq!(definition.grid().scope_names(), ["bugfix", "feature"]);
    // 有効スコープは `.md` 側の権威のまま。
    assert_eq!(definition.valid_scopes(), ["bugfix", "express", "feature"]);

    // initialization 特例: frontmatter に関係なく全列で EXECUTE。
    for scope in ["bugfix", "feature"] {
        for init in ["bootstrap", "workspace-init"] {
            assert_eq!(
                definition.grid().action(scope, &slug(init)),
                Some(PlanActionView::Execute),
                "{scope}/{init}"
            );
        }
    }
    assert_eq!(
        definition.grid().action("bugfix", &slug("code-generation")),
        Some(PlanActionView::Skip)
    );
    assert_eq!(
        slugs(&definition.subgraph_for_scope("bugfix").unwrap()),
        ["bootstrap", "workspace-init", "intent-capture"]
    );
}

#[test]
fn d_an_unreadable_scope_grid_falls_back_the_same_way() {
    let fixture = Fixture::new(Some(GRAPH_JSON), Some("{ not json"), &scope_files());
    let definition = load_definition(&fixture);
    assert_eq!(definition.grid().scope_names(), ["bugfix", "feature"]);
    assert_eq!(
        definition
            .grid()
            .action("feature", &slug("code-generation")),
        Some(PlanActionView::Execute)
    );
}

// ---------------------------------------------------------------------------
// (e) `.md` あり × 列なし = zero-EXECUTE な正当スコープ
// ---------------------------------------------------------------------------

#[test]
fn e_an_identity_file_without_a_grid_column_is_a_zero_execute_scope_not_an_unknown_one() {
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    let definition = load_definition(&fixture);

    assert!(definition.is_valid_scope("express"));
    assert!(!definition.grid().contains_scope("express"));
    // 拒否ではなく空。
    assert!(definition.subgraph_for_scope("express").unwrap().is_empty());
    assert_eq!(
        definition.first_in_scope_stage_of_phase(PhaseView::Ideation, "express"),
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
    let definition = load_definition(&fixture);

    // 列は読めているが、有効スコープではない。
    assert!(definition.grid().contains_scope("ghost"));
    assert!(!definition.is_valid_scope("ghost"));
    assert!(!definition.valid_scopes().contains(&"ghost"));

    // 未知スコープの非対称: subgraph だけが Err、他は None / 空。
    let error = definition.subgraph_for_scope("ghost").unwrap_err();
    assert_eq!(error.scope(), "ghost");
    assert_eq!(error.valid_scopes(), ["bugfix", "express", "feature"]);
    assert_eq!(
        definition.first_in_scope_stage_of_phase(PhaseView::Ideation, "ghost"),
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
    let definition = load_definition(&fixture);

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
    let definition = load_definition(&fixture);

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

    // stages_in_scope は文書順走査なので配列順そのまま。
    let listed: Vec<&str> = definition
        .stages_in_scope("feature")
        .iter()
        .map(|(s, _, _)| s.as_str())
        .collect();
    assert_eq!(listed, ["later", "earlier"]);
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
    let error = read(&fixture.reader()).unwrap_err();
    let WorkflowDefinitionReadError::ScopeFile { message } = error else {
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
    let error = read(&fixture.reader()).unwrap_err();
    let WorkflowDefinitionReadError::ScopeFile { message } = error else {
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
    let error = read(&fixture.reader()).unwrap_err();
    // upstream 逐語 (aidlc-lib.ts:8666-8668 @3c3146cf) の形を pin する
    assert!(
        matches!(error, WorkflowDefinitionReadError::ScopeFile { ref message }
            if message.starts_with("Duplicate scope name \"feature\" in ")
                && message.contains(": already declared in ")
                && message.ends_with(". Rename one of them.")),
        "{error:?}"
    );
}

#[test]
fn a_missing_scopes_directory_yields_an_empty_catalog_rather_than_a_failure() {
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &[]);
    let reader = DefinitionPaths::new(
        fixture.data_dir.clone(),
        fixture.scopes_dir.join("does-not-exist"),
    );
    let definition = read(&reader).unwrap();
    assert!(definition.valid_scopes().is_empty());
    // グリッド列は読めているが、権威が無いので全スコープが未知になる。
    assert!(definition.grid().contains_scope("feature"));
    assert!(definition.subgraph_for_scope("feature").is_err());
}

// ---------------------------------------------------------------------------
// 追加: ビュー型へ写せない値
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
    let error = read(&fixture.reader()).unwrap_err();
    assert!(
        matches!(error, WorkflowDefinitionReadError::Malformed { ref message } if message.contains("unknown phase")),
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
    let definition = load_definition(&fixture);
    let node = definition.graph().get(&slug("s")).unwrap();
    assert_eq!(node.mode(), StageModeView::AgentTeam);
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
    let definition = load_definition(&fixture);
    assert_eq!(
        definition.grid().action("feature", &slug("intent-capture")),
        Some(PlanActionView::Execute)
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

// ---------------------------------------------------------------------------
// 追加: 定義の識別子と内容版 (ADR-008 / C4)
// ---------------------------------------------------------------------------

#[test]
fn the_loaded_definition_is_stamped_with_the_harness_identity() {
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    let definition = read(&fixture.reader()).unwrap();

    // id は harness.json の `name`。
    assert_eq!(definition.id(), &definition_id("claude"));
    // revision は 3 入力の正準ダイジェスト。
    assert_eq!(definition.revision().as_str().len(), "sha256:".len() + 64);
    assert!(definition.revision().as_str().starts_with("sha256:"));
}

#[test]
fn a_missing_harness_identity_file_is_fatal() {
    let fixture = Fixture::with_harness(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files(), None);
    let error = read(&fixture.reader()).unwrap_err();
    let WorkflowDefinitionReadError::HarnessIdentity { path, cause } = error else {
        panic!("HarnessIdentity を期待した");
    };
    assert_eq!(path, fixture.harness_path().display().to_string());
    assert!(!cause.is_empty(), "OS 由来の理由を材料として運ぶ");
}

#[test]
fn a_harness_identity_file_that_is_not_json_or_has_no_name_is_fatal() {
    for harness in ["{", r#"{"harnessDir": ".claude"}"#, r#"{"name": ""}"#] {
        let fixture = Fixture::with_harness(
            Some(GRAPH_JSON),
            Some(GRID_JSON),
            &scope_files(),
            Some(harness),
        );
        let error = read(&fixture.reader()).unwrap_err();
        assert!(
            matches!(error, WorkflowDefinitionReadError::HarnessIdentity { .. }),
            "harness.json {harness:?} は HarnessIdentity で落ちるはず: {error:?}"
        );
    }
}

#[test]
fn the_revision_is_stable_for_the_same_inputs_and_changes_with_them() {
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    let first = load_definition(&fixture);
    let second = load_definition(&fixture);
    // 同一入力を 2 回読んでも同じ内容版。
    assert_eq!(first.revision(), second.revision());

    // グリッドを 1 文字変えれば変わる (EXECUTE → SKIP)。
    fixture.rewrite_grid(&GRID_JSON.replacen(
        "\"intent-capture\": \"EXECUTE\"",
        "\"intent-capture\": \"SKIP\"",
        1,
    ));
    let after = load_definition(&fixture);
    assert_ne!(first.revision(), after.revision());
    // 系譜 ID は変わらない (ADR-008)。
    assert_eq!(first.id(), after.id());
}

#[test]
fn the_revision_covers_the_scope_identity_files_as_well_as_the_two_json_inputs() {
    let base = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    let mut altered_scopes = scope_files();
    // `feature` の depth だけを変える — グラフもグリッドも同一。
    altered_scopes[0] = (
        "feature",
        "---\nname: feature\ndepth: deep\nkeywords: [api, endpoint]\nskeleton: on\nreview_cap: adversarial\n---\n\n# Feature scope\n",
    );
    let altered = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &altered_scopes);

    assert_ne!(
        load_definition(&base).revision(),
        load_definition(&altered).revision(),
        "scope identity は revision の入力の 1 つ"
    );
}

#[test]
fn a_missing_grid_still_yields_a_revision_derived_from_the_transposed_grid() {
    // グリッド欠損は fatal にしない (§4 #3) — revision は導出グリッドから作る。
    let without = Fixture::new(Some(GRAPH_JSON), None, &scope_files());
    let definition = load_definition(&without);
    assert!(definition.revision().as_str().starts_with("sha256:"));

    // 導出グリッドと配布グリッドは中身が違うので revision も違う。
    let with = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    assert_ne!(definition.revision(), load_definition(&with).revision());
}

#[test]
fn every_enum_valued_field_is_reported_as_malformed_with_the_key_that_caused_it() {
    // 未知の列挙値は load 時に落とす (12 §10 表 #3) — ドメイン型に `Unknown` variant を
    // 持たせず Always Valid を保つため。診断文言はキーごとに違い、どのフィールドが原因かが
    // 1 行で分かる。`slug` / `phase` は既存テストが押さえているので残り 7 キーを埋める。
    let cases: [(&str, &str); 7] = [
        (
            r#"[{ "slug": "s", "number": "one", "name": "S", "phase": "ideation",
                  "execution": "ALWAYS", "condition": "c", "lead_agent": "a", "mode": "inline",
                  "inputs": "i", "outputs": "o", "scopes": [] }]"#,
            "has invalid number",
        ),
        (
            r#"[{ "slug": "s", "number": "1.1", "name": "S", "phase": "ideation",
                  "execution": "SOMETIMES", "condition": "c", "lead_agent": "a", "mode": "inline",
                  "inputs": "i", "outputs": "o", "scopes": [] }]"#,
            "has unknown execution",
        ),
        (
            r#"[{ "slug": "s", "number": "1.1", "name": "S", "phase": "ideation",
                  "execution": "ALWAYS", "condition": "c", "lead_agent": "a", "mode": "telepathy",
                  "inputs": "i", "outputs": "o", "scopes": [] }]"#,
            "has unknown mode",
        ),
        (
            r#"[{ "slug": "s", "number": "1.1", "name": "S", "phase": "ideation",
                  "execution": "ALWAYS", "condition": "c", "lead_agent": "a", "mode": "inline",
                  "inputs": "i", "outputs": "o", "scopes": [],
                  "consumes": [{ "artifact": "x", "required": true, "conditional_on": "bluefield" }] }]"#,
            "has unknown conditional_on",
        ),
        (
            r#"[{ "slug": "s", "number": "1.1", "name": "S", "phase": "ideation",
                  "execution": "ALWAYS", "condition": "c", "lead_agent": "a", "mode": "inline",
                  "inputs": "i", "outputs": "o", "scopes": [],
                  "requires_stage": ["Not A Slug"] }]"#,
            "requires invalid slug",
        ),
        (
            r#"[{ "slug": "s", "number": "1.1", "name": "S", "phase": "ideation",
                  "execution": "ALWAYS", "condition": "c", "lead_agent": "a", "mode": "inline",
                  "inputs": "i", "outputs": "o", "scopes": [],
                  "rules_in_context": [{ "path": "memory/org.md", "scope": "galaxy" }] }]"#,
            "has unknown rule scope",
        ),
        (
            r#"[{ "slug": "s", "number": "1.1", "name": "S", "phase": "ideation",
                  "execution": "ALWAYS", "condition": "c", "lead_agent": "a", "mode": "inline",
                  "inputs": "i", "outputs": "o", "scopes": [],
                  "review_class": "casual" }]"#,
            "has unknown review_class",
        ),
    ];

    for (graph, fragment) in cases {
        let fixture = Fixture::new(Some(graph), None, &[]);
        let error = read(&fixture.reader()).unwrap_err();
        assert!(
            matches!(error, WorkflowDefinitionReadError::Malformed { ref message }
                if message.contains(fragment) && message.contains("stage-graph.json")),
            "{fragment} を期待したが {error:?}"
        );
    }
}

#[test]
fn a_scopes_path_that_is_not_a_directory_is_reported_instead_of_being_treated_as_empty() {
    // ディレクトリ**欠落**だけが空カタログ扱い (12 §4)。存在するのに読めない場合は、
    // 黙って 0 スコープにすると「有効スコープが 1 つも無い」と区別できなくなるので報告する。
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &[]);
    let not_a_dir = fixture.data_dir.join("scopes-as-a-file");
    std::fs::write(&not_a_dir, "これはディレクトリではない\n").unwrap();

    let reader = DefinitionPaths::new(
        fixture.data_dir.clone(),
        fixture.data_dir.join("scopes-as-a-file"),
    );
    let error = read(&reader).unwrap_err();
    assert!(
        matches!(error, WorkflowDefinitionReadError::ScopeFile { ref message }
            if message.starts_with(&format!("{}: ", not_a_dir.display()))),
        "{error:?}"
    );
}

#[test]
fn an_identity_entry_that_cannot_be_read_as_a_file_is_reported_with_its_path() {
    // 列挙は名前だけを見るので、`aidlc-*.md` という名のディレクトリも候補に入る。
    // 読めない候補は 1 件でも致命 — 有効スコープの権威が欠けたまま進まない (F7)。
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &[]);
    let masquerading = fixture.scopes_dir.join("aidlc-not-a-file.md");
    std::fs::create_dir_all(&masquerading).unwrap();

    let error = read(&fixture.reader()).unwrap_err();
    assert!(
        matches!(error, WorkflowDefinitionReadError::ScopeFile { ref message }
            if message.starts_with(&format!("{}: ", masquerading.display()))),
        "{error:?}"
    );
}
