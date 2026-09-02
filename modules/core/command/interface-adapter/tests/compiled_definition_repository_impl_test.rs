//! 統合テスト: `CompiledDefinitionRepositoryImpl` が 12-workflow-definition §4 の失敗態度表を
//! 全行満たすこと。
//!
//! # 集約 `CompiledDefinition` の Repository のテストである (2026-09-02 オーナー裁定、b36)
//!
//! ここに並ぶ検収は元は旧 `WorkflowDefinitionRepositoryImpl` (ファイルから集約を組む実装 —
//! b30 で破棄) に張られ、b30 で取込境界 (外部システムクライアント擬制) へ移った。b36 で
//! その擬制が棄却され、対象は配布束の集約 `CompiledDefinition` を `find_by_id(&id)` で
//! 再構成する Repository になった — 失敗態度の表そのものは**観測可能な契約**であって
//! 対象の分類とは独立なので、全行を維持する。読んだ内容が定義の述語として立つことは、
//! `WorkflowDefinition::define` で集約へ materialize してから突く。
//!
//! 各テストは tempdir に合成 `stage-graph.json` / `scope-grid.json` / `harness.json` /
//! `scopes/aidlc-*.md` を書いて 1 行ずつ検証する:
//! (a) 正常読取と述語の疎通 / (b) graph 欠損 = Err / (c) 不正 JSON = Err /
//! (d) grid 欠損 = 転置導出 (initialization 特例込み) / (e) `.md` あり × 列なし = zero-EXECUTE /
//! (f) 列あり × `.md` なし = `valid_scopes` に不出現 / (g) 未知フィールド入り JSON が読めること。
// indexing_slicing (固定長フィクスチャの添字参照) と panic (想定外ケースの即時失敗という
// 検証用途) も unwrap_used と同じ理由で file 単位の allow が要る。expect は `#[test]` の外の
// ヘルパで使うため clippy.toml の allow-expect-in-tests が効かず、同じく file 単位で要る。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, PhaseId, PlanAction, ReviewClass, RuleScope, StageMode, StageSlug,
    WorkflowDefinition, WorkflowDefinitionId,
};
use core_command_domain::workflow_definition::{CompiledDefinition, CompiledDefinitionId};
use core_command_interface_adapter::orchestration::CompiledDefinitionRepositoryImpl;
use core_command_use_case::orchestration::{CompiledDefinitionRepository, RepositoryError};
use std::error::Error;
use std::path::PathBuf;
use tempfile::TempDir;

/// 配布束を読めない形 (Repository 契約のジェネリックエラー)。
type ReadError = RepositoryError<CompiledDefinitionId>;

/// フィクスチャの harness.json が名乗る識別子。
fn claude_id() -> CompiledDefinitionId {
    CompiledDefinitionId::parse("claude").expect("フィクスチャの配布束 id")
}

/// 同期の呼び口 — 本ファイルの検収は失敗態度の表を 1 行ずつ見るだけなので、都度
/// current-thread ランタイムで `find_by_id` を回す (I/O は同期 fs 読取)。
fn find(
    compiled_definition_repository: &CompiledDefinitionRepositoryImpl,
) -> Result<CompiledDefinition, ReadError> {
    find_by(compiled_definition_repository, &claude_id())
}

/// 任意の id で引く (id 照合の検収用)。
fn find_by(
    compiled_definition_repository: &CompiledDefinitionRepositoryImpl,
    id: &CompiledDefinitionId,
) -> Result<CompiledDefinition, ReadError> {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("current-thread ランタイム")
        .block_on(compiled_definition_repository.find_by_id(id))
}

/// `Corrupt` が原因連鎖で運ぶ診断表示を取り出す。
///
/// 契約は「壊れていた」としか約束しないので、**どう壊れていたか**はアダプタ私有の型が
/// `Error::source` の連鎖でだけ運ぶ (裁定 6)。テストはその表示文字列で判定する
/// (`RepositoryError<WorkflowDefinitionId>` は `source` が比較不能なため `PartialEq` を持たない)。
fn corrupt_cause(error: &ReadError) -> String {
    assert!(
        matches!(error, RepositoryError::Corrupt { .. }),
        "Corrupt を期待した: {error:?}"
    );
    Error::source(error)
        .expect("Corrupt は原因を連鎖する")
        .to_string()
}

/// `Io` が運ぶ対象パス。
fn io_path(error: &ReadError) -> PathBuf {
    let RepositoryError::Io { path, .. } = error else {
        panic!("Io を期待した: {error:?}");
    };
    path.clone().expect("読取失敗は対象パスを運ぶ")
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

    fn compiled_definition_repository(&self) -> CompiledDefinitionRepositoryImpl {
        CompiledDefinitionRepositoryImpl::new(self.data_dir.clone(), self.scopes_dir.clone())
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

fn slug(s: &str) -> StageSlug {
    StageSlug::parse(s).unwrap()
}

fn slugs(nodes: &[&core_command_domain::workflow_definition::StageNode]) -> Vec<String> {
    nodes
        .iter()
        .map(|n| n.slug().as_str().to_string())
        .collect()
}

/// 配布束を読む (成功を期待する経路)。
fn load(fixture: &Fixture) -> CompiledDefinition {
    find(&fixture.compiled_definition_repository()).unwrap()
}

/// 取り込んだ材料をそのまま定義集約へ立てる。
///
/// 述語 (`valid_scopes` / `subgraph_for_scope` / `stages_in_scope` /
/// `first_in_scope_stage_of_phase`) は集約が所有するので、取込の検収でそれを突くには
/// 一度 genesis を通す。`define` は (集約, 誕生イベント) の対を返すので集約側だけを取る
/// (`coding-rules/aggregate-commands.md`)。
fn materialize(artifacts: CompiledDefinition) -> WorkflowDefinition {
    // 集約ごとに自前の ID 型を持つ — 系譜は同じ name なので値で写す (合成ルートが両 ID を
    // 同じ源から鋳造するのと同じ突合せ)。
    let id = WorkflowDefinitionId::parse(artifacts.id().as_str()).expect("同一文法の系譜 ID");
    let revision = artifacts.revision().clone();
    let (graph, grid, scopes) = artifacts.into_content();
    WorkflowDefinition::define(id, revision, graph, grid, scopes, at()).0
}

/// 定義イベントの発生時刻 (取込の検収では時刻そのものは問わないので固定値)。
fn at() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2026-08-31T00:00:00Z")
        .expect("固定の ISO 8601 UTC")
        .with_timezone(&chrono::Utc)
}

/// 取込 → materialize の短縮形。
fn definition_of(fixture: &Fixture) -> WorkflowDefinition {
    materialize(load(fixture))
}

// ---------------------------------------------------------------------------
// (a) 正常読取と述語の疎通
// ---------------------------------------------------------------------------

#[test]
fn a_full_read_maps_every_field_group_onto_the_domain_model() {
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    let artifacts = load(&fixture);

    assert_eq!(artifacts.graph().len(), 5);
    let node = artifacts.graph().get(&slug("code-generation")).unwrap();
    assert_eq!(node.number().as_str(), "3.1");
    assert_eq!(node.phase(), PhaseId::Construction);
    assert_eq!(node.mode(), StageMode::Pipeline);
    assert_eq!(node.for_each(), Some("unit-of-work"));
    assert!(node.workspace_requires());
    assert_eq!(node.support_agents(), ["tester".to_string()]);
    assert_eq!(node.optional_produces(), ["migration".to_string()]);
    assert_eq!(
        node.produces_kinds()
            .iter()
            .find(|(kind, _)| kind == "code")
            .map(|(_, artifacts)| artifacts.as_slice()),
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
    let bootstrap = artifacts.graph().get(&slug("bootstrap")).unwrap();
    assert_eq!(bootstrap.enabled(), None);
    assert!(bootstrap.is_enabled());
    assert_eq!(bootstrap.reviewer(), None);
    assert!(bootstrap.produces().is_empty());
    assert!(!bootstrap.workspace_requires());

    // scope identity の frontmatter。
    let feature = artifacts.scopes().get("feature").unwrap();
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
    // 取り込んだ材料がそのまま定義の述語として立つこと — 取込境界の検収は「材料が揃うこと」
    // だが、揃った材料の意味は集約側の述語でしか確かめられない。
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    let definition = definition_of(&fixture);

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
    assert_eq!(rows[0].1, PhaseId::Initialization);
    assert_eq!(rows[0].2, Some(PlanAction::Execute));
    assert_eq!(rows[4].2, Some(PlanAction::Skip));

    // 静的グリッドの照会は 3 値。実効プランの合成 (recompose オーバレイとの重ね合わせ) は
    // FR8.4 で `IntentExecution` へ移設したのでここには無い。
    assert_eq!(
        definition
            .grid()
            .action("feature", &slug("code-generation")),
        Some(PlanAction::Execute)
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
    let error = find(&fixture.compiled_definition_repository()).unwrap_err();
    // OS 由来の読取失敗は `Io` — 対象パスと `ErrorKind` だけを運ぶ。
    assert_eq!(io_path(&error), fixture.graph_path());
}

#[test]
fn b_the_reported_path_follows_the_injected_override() {
    // env オーバライドは**パス解決**の話であって、契約に載る分類ではない (逐語文言の hint
    // 分岐はクエリ側が所有する — b26 段階 2)。ここで固定するのは「注入したパスがそのまま
    // 失敗の対象として報告される」ことだけ。
    let fixture = Fixture::new(None, Some(GRID_JSON), &scope_files());
    let missing = fixture.data_dir.join("pinned-graph.json");
    let compiled_definition_repository = fixture
        .compiled_definition_repository()
        .with_stage_graph_override(missing.clone());
    let error = find(&compiled_definition_repository).unwrap_err();
    assert_eq!(io_path(&error), missing);
}

// ---------------------------------------------------------------------------
// (c) 不正 JSON = Err (欠損とは別変種)
// ---------------------------------------------------------------------------

#[test]
fn c_a_malformed_stage_graph_is_fatal_under_a_different_variant() {
    // 読めたが内容が壊れている = `Corrupt`。欠損 (`Io`) とは別変種で、どう壊れていたかは
    // 原因連鎖にだけ現れる。
    let fixture = Fixture::new(Some("[ { \"slug\": "), Some(GRID_JSON), &scope_files());
    let error = find(&fixture.compiled_definition_repository()).unwrap_err();
    let cause = corrupt_cause(&error);
    assert!(cause.contains("not valid JSON"), "{cause}");
    assert!(
        cause.contains(&fixture.graph_path().display().to_string()),
        "{cause}"
    );
}

#[test]
fn c_a_stage_graph_object_root_is_rejected_because_the_root_is_an_array() {
    let fixture = Fixture::new(Some("{\"stages\": []}"), Some(GRID_JSON), &scope_files());
    let error = find(&fixture.compiled_definition_repository()).unwrap_err();
    assert!(corrupt_cause(&error).contains("not valid JSON"));
}

// ---------------------------------------------------------------------------
// (d) grid 欠損 = 転置導出 (initialization 特例込み)
// ---------------------------------------------------------------------------

#[test]
fn d_a_missing_scope_grid_falls_back_to_the_transpose_instead_of_failing() {
    let fixture = Fixture::new(Some(GRAPH_JSON), None, &scope_files());
    let definition = definition_of(&fixture);

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
    let definition = definition_of(&fixture);
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
    let definition = definition_of(&fixture);

    assert!(definition.is_valid_scope("express"));
    assert!(!definition.grid().contains_scope("express"));
    // 拒否ではなく空。
    assert!(definition.subgraph_for_scope("express").unwrap().is_empty());
    assert_eq!(
        definition.first_in_scope_stage_of_phase(PhaseId::Ideation, "express"),
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
    let definition = definition_of(&fixture);

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
    let definition = definition_of(&fixture);

    let node = definition.graph().get(&slug("acme-scan")).unwrap();
    assert_eq!(node.plugin(), Some("acme"));
    // `enabled: false` のノードも取込モデルからは除外しない (判断は呼出側)。
    assert_eq!(node.enabled(), Some(false));
    assert!(!node.is_enabled());
    assert_eq!(definition.valid_scopes(), ["feature"]);
}

// ---------------------------------------------------------------------------
// 追加: 文書順の保持 (F2 — 2 経路の使い分けを潰さない)
// ---------------------------------------------------------------------------

#[test]
fn the_client_preserves_document_order_and_keeps_the_two_ordering_paths_distinct() {
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
    let definition = definition_of(&fixture);

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
// 追加: scope identity ファイルの拒否 (診断は原因連鎖にだけ現れる)
// ---------------------------------------------------------------------------

#[test]
fn an_invalid_skeleton_value_is_rejected_with_the_offending_value_in_the_cause() {
    let fixture = Fixture::new(
        Some(GRAPH_JSON),
        Some(GRID_JSON),
        &[("feature", "---\nname: feature\nskeleton: enabled\n---\n")],
    );
    let error = find(&fixture.compiled_definition_repository()).unwrap_err();
    let cause = corrupt_cause(&error);
    let path = fixture.scopes_dir.join("aidlc-feature.md");
    assert!(cause.contains(&path.display().to_string()), "{cause}");
    assert!(cause.contains("skeleton"), "{cause}");
    assert!(cause.contains("enabled"), "{cause}");
}

#[test]
fn a_scope_file_without_a_name_is_rejected() {
    let fixture = Fixture::new(
        Some(GRAPH_JSON),
        Some(GRID_JSON),
        &[("feature", "---\ndepth: standard\n---\n")],
    );
    let error = find(&fixture.compiled_definition_repository()).unwrap_err();
    let cause = corrupt_cause(&error);
    assert!(
        cause.ends_with("missing required frontmatter: name"),
        "{cause}"
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
    let error = find(&fixture.compiled_definition_repository()).unwrap_err();
    // 重複した名前と両方のファイルが診断に載る (どちらを直せばよいかが分かる材料)。
    let cause = corrupt_cause(&error);
    assert!(cause.contains("feature"), "{cause}");
    assert!(cause.contains("already declared in"), "{cause}");
}

#[test]
fn a_missing_scopes_directory_yields_an_empty_catalog_rather_than_a_failure() {
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &[]);
    let compiled_definition_repository = CompiledDefinitionRepositoryImpl::new(
        fixture.data_dir.clone(),
        fixture.scopes_dir.join("does-not-exist"),
    );
    let definition = materialize(find(&compiled_definition_repository).unwrap());
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
    let error = find(&fixture.compiled_definition_repository()).unwrap_err();
    assert!(corrupt_cause(&error).contains("unknown phase"));
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
    let artifacts = load(&fixture);
    let node = artifacts.graph().get(&slug("s")).unwrap();
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
    let definition = definition_of(&fixture);
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

// ---------------------------------------------------------------------------
// 追加: 定義の識別子と内容版 (ADR-008 — 配布物が名乗る id / 3 入力の正準ダイジェスト)
// ---------------------------------------------------------------------------

#[test]
fn load_returns_the_artifacts_stamped_with_the_harness_identity() {
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    let artifacts = load(&fixture);

    // id は harness.json の `name`。
    assert_eq!(artifacts.id(), &claude_id());
    // revision は 3 入力の正準ダイジェスト。
    assert_eq!(artifacts.revision().as_str().len(), "sha256:".len() + 64);
    assert!(artifacts.revision().as_str().starts_with("sha256:"));
}

#[test]
fn a_distribution_naming_another_id_is_not_found_under_the_requested_one() {
    // Repository は自集約の ID で引く — 要求 id と配布束が名乗る id (harness.json の
    // `name`、ADR-008) が食い違えば、要求された id の配布定義は「無い」(`NotFound`)。
    // 旧取込境界は「配布物が名乗る id をそのまま返す」形だったが、Repository への昇格で
    // 照合が契約に戻った (b36)。合成ルートは同じ harness.json から両 ID を鋳造するので、
    // 実際にここへ落ちるのは配布物が要求と食い違う異常系だけである。
    let claude = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    assert_eq!(load(&claude).id(), &claude_id());

    let kiro = Fixture::with_harness(
        Some(GRAPH_JSON),
        Some(GRID_JSON),
        &scope_files(),
        Some(r#"{"name": "kiro"}"#),
    );
    let error = find(&kiro.compiled_definition_repository()).unwrap_err();
    assert!(
        matches!(&error, RepositoryError::NotFound { id } if id == &claude_id()),
        "要求 id で引けない: {error:?}"
    );

    // 名乗っている id で引けば見つかる。系譜 ID が違えば別の定義だが、3 入力が同一なので
    // 内容版は同じ — id だけが違う。
    let kiro_id = CompiledDefinitionId::parse("kiro").unwrap();
    let found = find_by(&kiro.compiled_definition_repository(), &kiro_id).unwrap();
    assert_eq!(found.id(), &kiro_id);
    assert_eq!(found.revision(), load(&claude).revision());
}

#[test]
fn the_identity_is_read_before_the_three_inputs() {
    // harness.json もグラフも無い状態で報告されるのは harness.json のほう —
    // 識別子の読取が 3 入力より前にあることの検収 (id を与えられない配布物は、内容を
    // 読めても定義を確立できない)。
    let fixture = Fixture::with_harness(None, None, &[], None);
    let error = find(&fixture.compiled_definition_repository()).unwrap_err();
    assert_eq!(
        io_path(&error),
        fixture.harness_path(),
        "識別子の読取はグラフ読取より前"
    );
}

#[test]
fn a_missing_harness_identity_file_is_fatal() {
    let fixture = Fixture::with_harness(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files(), None);
    let error = find(&fixture.compiled_definition_repository()).unwrap_err();
    // ファイルが無いのは OS 由来の読取失敗 — 内容の破損ではない。
    assert_eq!(io_path(&error), fixture.harness_path());
}

#[test]
fn a_harness_identity_file_that_is_not_json_or_has_no_name_is_corrupt_not_io() {
    // 読めたが内容が定義 id を与えない — 欠損 (`Io`) と読み分ける。
    for harness in ["{", r#"{"harnessDir": ".claude"}"#, r#"{"name": ""}"#] {
        let fixture = Fixture::with_harness(
            Some(GRAPH_JSON),
            Some(GRID_JSON),
            &scope_files(),
            Some(harness),
        );
        let error = find(&fixture.compiled_definition_repository()).unwrap_err();
        let cause = corrupt_cause(&error);
        assert!(
            cause.contains(&fixture.harness_path().display().to_string()),
            "harness.json {harness:?}: {cause}"
        );
    }
}

#[test]
fn the_corrupt_variant_carries_only_a_diagnostic_not_a_classification() {
    // Repository の契約は**分類を載せない** (裁定 6 — 内部実装がバレる情報を含めない):
    // `Corrupt` の `Display` は「どの集約が壊れていたか」(id・通番) までしか言わず、どの
    // ファイルがどう壊れていたかは `Error::source` に連なる診断表示だけが運ぶ。
    let fixture = Fixture::new(Some("[ { \"slug\": "), Some(GRID_JSON), &scope_files());
    let error = find(&fixture.compiled_definition_repository()).unwrap_err();
    assert!(matches!(error, RepositoryError::Corrupt { .. }));
    let rendered = error.to_string();
    assert!(rendered.starts_with("corrupt"), "{rendered}");
    assert!(
        !rendered.contains("stage-graph") && !rendered.contains("JSON"),
        "分類・材料が Display に漏れている: {rendered}"
    );
    // 材料 (どのファイルがどう壊れていたか) は原因連鎖にだけ現れる。
    let cause = corrupt_cause(&error);
    assert!(
        cause.contains(&fixture.graph_path().display().to_string()),
        "{cause}"
    );
}

#[test]
fn the_revision_is_stable_for_the_same_inputs_and_changes_with_them() {
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    let first = load(&fixture);
    let second = load(&fixture);
    // 同一入力を 2 回読んでも同じ内容版。
    assert_eq!(first.revision(), second.revision());

    // グリッドを 1 文字変えれば変わる (EXECUTE → SKIP)。
    fixture.rewrite_grid(&GRID_JSON.replacen(
        "\"intent-capture\": \"EXECUTE\"",
        "\"intent-capture\": \"SKIP\"",
        1,
    ));
    let after = load(&fixture);
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
        load(&base).revision(),
        load(&altered).revision(),
        "scope identity は revision の入力の 1 つ"
    );
}

#[test]
fn a_missing_grid_still_yields_a_revision_derived_from_the_transposed_grid() {
    // グリッド欠損は fatal にしない (§4 #3) — revision は導出グリッドから作る。
    let without = Fixture::new(Some(GRAPH_JSON), None, &scope_files());
    let artifacts = load(&without);
    assert!(artifacts.revision().as_str().starts_with("sha256:"));

    // 導出グリッドと配布グリッドは中身が違うので revision も違う。
    let with = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    assert_ne!(artifacts.revision(), load(&with).revision());
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
        let error = find(&fixture.compiled_definition_repository()).unwrap_err();
        let cause = corrupt_cause(&error);
        assert!(
            cause.contains(fragment) && cause.contains("stage-graph.json"),
            "{fragment} を期待したが {cause}"
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

    let compiled_definition_repository = CompiledDefinitionRepositoryImpl::new(
        fixture.data_dir.clone(),
        fixture.data_dir.join("scopes-as-a-file"),
    );
    let error = find(&compiled_definition_repository).unwrap_err();
    assert_eq!(io_path(&error), not_a_dir);
}

#[test]
fn an_identity_entry_that_cannot_be_read_as_a_file_is_reported_with_its_path() {
    // 列挙は名前だけを見るので、`aidlc-*.md` という名のディレクトリも候補に入る。
    // 読めない候補は 1 件でも致命 — 有効スコープの権威が欠けたまま進まない (F7)。
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &[]);
    let masquerading = fixture.scopes_dir.join("aidlc-not-a-file.md");
    std::fs::create_dir_all(&masquerading).unwrap();

    let error = find(&fixture.compiled_definition_repository()).unwrap_err();
    assert_eq!(io_path(&error), masquerading);
}

// ---------------------------------------------------------------------------
// store — 書き側 (b36)。graph / grid のバイト忠実は `golden_parity_test.rs` が検収する。
// ここは scope identity ファイル群の書出しと掃除、(イベント, 集約) の対の照合、I/O 失敗の
// 報告を見る。
// ---------------------------------------------------------------------------

/// 同期の書き口 — genesis で対を鋳造し、他リポジトリと同じ形で `store` へ渡す。
fn store_into(
    compiled_definition_repository: &mut CompiledDefinitionRepositoryImpl,
    compiled_definition: &CompiledDefinition,
) -> Result<(), ReadError> {
    let (reborn, event) = CompiledDefinition::compile(
        compiled_definition.id().clone(),
        compiled_definition.revision().clone(),
        compiled_definition.graph().clone(),
        compiled_definition.grid().clone(),
        compiled_definition.scopes().clone(),
    );
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("current-thread ランタイム")
        .block_on(compiled_definition_repository.store(&event, &reborn))
}

/// 書き先の tempdir を持つ書き手 (`<dir>/tools/data` / `<dir>/scopes`)。
fn empty_writer() -> (CompiledDefinitionRepositoryImpl, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let writer = CompiledDefinitionRepositoryImpl::new(
        dir.path().join("tools/data"),
        dir.path().join("scopes"),
    );
    (writer, dir)
}

#[test]
fn storing_writes_one_identity_file_per_scope_and_sweeps_the_stale_ones() {
    // 集約が持つ scope 集合と `aidlc-*.md` の集合を一致させる — 集合に無い既存ファイルは
    // 残すと次の find_by_id が余分なスコープとして読み戻すので消す。パターン外のファイルは
    // 触らない。
    let fixture = Fixture::new(
        Some(GRAPH_JSON),
        Some(GRID_JSON),
        &[
            (
                "feature",
                "---\nname: feature\ndepth: standard\nkeywords: [api, endpoint]\nskeleton: on\nreview_cap: adversarial\n---\n\n# Feature scope\n",
            ),
            (
                "express",
                "---\nname: express\nfreeform_default: true\n---\n",
            ),
        ],
    );
    let compiled = load(&fixture);

    let (mut writer, out) = empty_writer();
    let scopes_out = out.path().join("scopes");
    std::fs::create_dir_all(&scopes_out).unwrap();
    std::fs::write(scopes_out.join("aidlc-stale.md"), "---\nname: stale\n---\n").unwrap();
    std::fs::write(scopes_out.join("README.md"), "kept\n").unwrap();

    store_into(&mut writer, &compiled).unwrap();

    let mut names: Vec<String> = std::fs::read_dir(&scopes_out)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["README.md", "aidlc-express.md", "aidlc-feature.md"]);
    // frontmatter は読み手が受ける最小サブセットと対称 — 散文本文は集約の内容ではないので
    // 書かない。
    assert_eq!(
        std::fs::read_to_string(scopes_out.join("aidlc-feature.md")).unwrap(),
        "---\nname: feature\ndepth: standard\nkeywords: [api, endpoint]\nskeleton: on\nreview_cap: adversarial\n---\n"
    );
    assert_eq!(
        std::fs::read_to_string(scopes_out.join("aidlc-express.md")).unwrap(),
        "---\nname: express\nfreeform_default: true\n---\n"
    );

    // 書いた面を読み戻すと同じ内容になる (store ⇄ find_by_id の往復)。内容版だけは
    // 「読めた入力の内容の版」なので、手書きフィクスチャの表記ゆれ (既定値のキー明示など)
    // を書き手が正規化した分だけ変わりうる — バイト同一の場合は golden 検収が固定する。
    let reread = find(&writer).unwrap();
    assert_eq!(reread.id(), compiled.id());
    assert_eq!(reread.graph(), compiled.graph());
    assert_eq!(reread.grid(), compiled.grid());
    assert_eq!(reread.scopes(), compiled.scopes());
}

#[test]
fn storing_a_pair_whose_event_does_not_describe_the_aggregate_is_refused() {
    // `Compiled` は内容そのものを運ぶ — 別の内容版を名乗る誕生記録と組ませた対は、歴史と
    // 保存像が別の内容を語る書込契約違反として拒む (`IntentRepositoryImpl` の写し)。
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    let compiled = load(&fixture);
    let foreign_revision = core_command_domain::workflow_definition::DefinitionRevision::parse(
        &format!("sha256:{}", "f".repeat(64)),
    )
    .unwrap();
    let (_, foreign_event) = CompiledDefinition::compile(
        compiled.id().clone(),
        foreign_revision,
        compiled.graph().clone(),
        compiled.grid().clone(),
        compiled.scopes().clone(),
    );

    let (mut writer, out) = empty_writer();
    let error = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(writer.store(&foreign_event, &compiled))
        .unwrap_err();
    assert!(
        corrupt_cause(&error).contains("store pair mismatch"),
        "{error:?}"
    );
    // 拒んだ書込は何も残さない。
    assert!(!out.path().join("tools/data/harness.json").exists());
}

#[test]
fn a_store_target_that_cannot_be_written_is_reported_as_io_with_the_offending_path() {
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    let compiled = load(&fixture);
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, "not a directory").unwrap();

    // data_dir の親が通常ファイル — 既存 identity の読取もディレクトリ作成もできない。
    // 最初に触る harness.json のパスで報告する。
    let data_dir = blocker.join("data");
    let mut writer =
        CompiledDefinitionRepositoryImpl::new(data_dir.clone(), dir.path().join("scopes"));
    let error = store_into(&mut writer, &compiled).unwrap_err();
    assert_eq!(io_path(&error), data_dir.join("harness.json"));

    // data_dir は作れるが harness.json の位置にディレクトリが居座る — 書けない。
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(data_dir.join("harness.json")).unwrap();
    let mut writer =
        CompiledDefinitionRepositoryImpl::new(data_dir.clone(), dir.path().join("scopes"));
    let error = store_into(&mut writer, &compiled).unwrap_err();
    assert_eq!(io_path(&error), data_dir.join("harness.json"));

    // 3 ファイルは書けたが scopes_dir を作れない。
    let data_dir = dir.path().join("data2");
    let scopes_dir = blocker.join("scopes");
    let mut writer = CompiledDefinitionRepositoryImpl::new(data_dir, scopes_dir.clone());
    let error = store_into(&mut writer, &compiled).unwrap_err();
    assert_eq!(io_path(&error), scopes_dir);
}

#[cfg(unix)]
#[test]
fn a_stale_identity_file_that_cannot_be_removed_is_reported_as_io_with_its_path() {
    use std::os::unix::fs::PermissionsExt as _;

    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    let compiled = load(&fixture);
    let (mut writer, out) = empty_writer();
    let scopes_out = out.path().join("scopes");
    std::fs::create_dir_all(&scopes_out).unwrap();
    let stale = scopes_out.join("aidlc-stale.md");
    std::fs::write(&stale, "---\nname: stale\n---\n").unwrap();
    // ディレクトリの書込を禁じると、その中のファイルは消せない。
    std::fs::set_permissions(&scopes_out, std::fs::Permissions::from_mode(0o555)).unwrap();

    let outcome = store_into(&mut writer, &compiled);

    // tempdir の後始末のため、判定より先に戻す。
    std::fs::set_permissions(&scopes_out, std::fs::Permissions::from_mode(0o755)).unwrap();
    let error = outcome.unwrap_err();
    assert_eq!(io_path(&error), stale);
}

#[cfg(unix)]
#[test]
fn a_scopes_dir_that_cannot_be_listed_is_reported_as_io_instead_of_skipping_the_sweep() {
    use std::os::unix::fs::PermissionsExt as _;

    // 一覧が取れないのに続けると、消すべき stale ファイルを残したまま新しい識別ファイルだけが
    // 増える (Bugbot 指摘)。一覧の失敗も `Io` として報告する。
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    let compiled = load(&fixture);
    let (mut writer, out) = empty_writer();
    let scopes_out = out.path().join("scopes");
    std::fs::create_dir_all(&scopes_out).unwrap();
    std::fs::set_permissions(&scopes_out, std::fs::Permissions::from_mode(0o000)).unwrap();

    let outcome = store_into(&mut writer, &compiled);

    std::fs::set_permissions(&scopes_out, std::fs::Permissions::from_mode(0o755)).unwrap();
    let error = outcome.unwrap_err();
    assert_eq!(io_path(&error), scopes_out);
    assert!(
        std::fs::read_dir(&scopes_out).unwrap().next().is_none(),
        "一覧に失敗した時点で止まり、識別ファイルは書かれない"
    );
}

#[test]
fn storing_keeps_the_prose_of_identity_files_and_the_extra_keys_of_harness_json() {
    // 「散文本文と harness.json の付随キーは書かない」= 壊さない。既存ファイルがあれば
    // 集約が所有する部分 (frontmatter / `name`) だけを差し替える (CodeRabbit 指摘)。
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    let compiled = load(&fixture);
    let (mut writer, out) = empty_writer();
    let data_out = out.path().join("tools/data");
    let scopes_out = out.path().join("scopes");
    std::fs::create_dir_all(&data_out).unwrap();
    std::fs::create_dir_all(&scopes_out).unwrap();
    std::fs::write(
        data_out.join("harness.json"),
        "{\n  \"name\": \"old\",\n  \"harnessDir\": \".claude\",\n  \"rulesSubdir\": \"rules\"\n}\n",
    )
    .unwrap();
    std::fs::write(
        scopes_out.join("aidlc-feature.md"),
        "---\nname: feature\ndepth: light\n---\n\n# Feature scope\n\nprose stays\n",
    )
    .unwrap();

    store_into(&mut writer, &compiled).unwrap();

    assert_eq!(
        std::fs::read_to_string(data_out.join("harness.json")).unwrap(),
        "{\n  \"name\": \"claude\",\n  \"harnessDir\": \".claude\",\n  \"rulesSubdir\": \"rules\"\n}\n",
        "付随キーとその順序を保ったまま name だけが差し替わる"
    );
    assert_eq!(
        std::fs::read_to_string(scopes_out.join("aidlc-feature.md")).unwrap(),
        "---\nname: feature\ndepth: standard\nkeywords: [api, endpoint]\nskeleton: on\nreview_cap: adversarial\n---\n\n# Feature scope\n\nprose stays\n",
        "frontmatter は集約の内容に差し替わり、本文はそのまま"
    );
    assert_eq!(find(&writer).unwrap().scopes(), compiled.scopes());
}

#[test]
fn a_harness_identity_that_is_not_a_json_object_is_not_overwritten() {
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    let compiled = load(&fixture);
    let (mut writer, out) = empty_writer();
    let data_out = out.path().join("tools/data");
    std::fs::create_dir_all(&data_out).unwrap();
    std::fs::write(data_out.join("harness.json"), "[\"not\", \"an object\"]\n").unwrap();

    let error = store_into(&mut writer, &compiled).unwrap_err();

    assert!(
        corrupt_cause(&error).contains("is not a JSON object"),
        "{error:?}"
    );
    assert_eq!(
        std::fs::read_to_string(data_out.join("harness.json")).unwrap(),
        "[\"not\", \"an object\"]\n",
        "読めない内容を黙って捨てない"
    );
}

#[test]
fn storing_writes_to_the_override_paths_so_the_round_trip_reads_what_it_wrote() {
    // override を設定した Repository は、読む先と同じパスへ書く (CodeRabbit 指摘)。
    let fixture = Fixture::new(Some(GRAPH_JSON), Some(GRID_JSON), &scope_files());
    let compiled = load(&fixture);
    let out = tempfile::tempdir().unwrap();
    let graph_override = out.path().join("pinned/graph.json");
    let grid_override = out.path().join("pinned/grid.json");
    let mut writer = CompiledDefinitionRepositoryImpl::new(
        out.path().join("tools/data"),
        out.path().join("scopes"),
    )
    .with_stage_graph_override(graph_override.clone())
    .with_scope_grid_override(grid_override.clone());

    store_into(&mut writer, &compiled).unwrap();

    assert!(graph_override.is_file() && grid_override.is_file());
    assert!(!out.path().join("tools/data/stage-graph.json").exists());
    assert!(!out.path().join("tools/data/scope-grid.json").exists());
    let reread = find(&writer).unwrap();
    assert_eq!(reread.graph(), compiled.graph());
    assert_eq!(reread.grid(), compiled.grid());
}
