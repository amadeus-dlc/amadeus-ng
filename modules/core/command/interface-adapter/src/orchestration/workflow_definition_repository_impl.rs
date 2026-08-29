//! `WorkflowDefinitionRepository` の実装 (Gateway) — Published Language 3 入力
//! (`stage-graph.json` / `scope-grid.json` / `<harnessRoot>/scopes/aidlc-<name>.md`) を
//! ディスクから読み、集約 `WorkflowDefinition` へ写す (12-workflow-definition §6)。
//!
//! ポート trait は use-case 層が所有し、その実装は `XxxRepositoryImpl` としてアダプタ層に
//! 置く (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md)。**格納形式がファイルであることはこの実装の内部
//! 詳細**であり、ポート名にも facade にも現れない。
//!
//! **この実装が所有するもの** (12 §6):
//! - パス解決とテストシーム (`<data_dir>/{stage-graph,scope-grid}.json` / `<scopes_dir>`、
//!   および `AIDLC_STAGE_GRAPH` / `AIDLC_SCOPE_GRID` 相当のオーバライド)。
//!   **env の読取そのものは合成ルートの責務**で、ここは注入されたパスだけを見る
//!   (テストを hermetic に保つため)。
//! - JSON コーデック (serde ワイヤ構造体) と frontmatter パーサ (手書き — 00-policy R9)。
//! - 逐語文言の組み立て。
//!
//! **serde の厳格度** (12 §10 の表):
//! 1. 未知フィールドは**許容**する (`deny_unknown_fields` を付けない — F1)。将来版や
//!    プラグインが `FIELD_ORDER` を増やしても読めなくなってはならない。
//! 2. 欠損 optional は `Option` ないし空 default (`#[serde(default)]`)。
//! 3. 未知の列挙値は**全列挙 (`phase` / `execution` / `review_class` / `mode`) を load 時に
//!    厳密 enum で落とす** (12 §10 表 #3 — 2026-08-22 裁定)。ドメイン型に `Unknown` variant を
//!    持たせず Always Valid を維持する。upstream との観測差は手編集グラフの未知値に限られ、
//!    dist の正規データでは生じない — ピン留め `3c3146cf` の配布実バイト 33 ノードが全数
//!    load できることは `tests/golden_parity_test.rs` が固定した。
//!
//! **失敗態度** (12 §4): グラフは fatal、グリッドは転置導出フォールバック、identity と
//! グリッド列の不一致は双方向とも正当。

use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, ConsumeDecl, DefinitionRevision, ExecutionKind, PhaseId, PlanAction,
    ReviewCapValue, ReviewClass, RuleInContext, RuleScope, ScopeGrid, ScopeMetadata, SensorRef,
    SkeletonDefault, StageGraph, StageMode, StageNode, StageNodeBuilder, StageNumber, StageSlug,
    WorkflowDefinition, WorkflowDefinitionId,
};
use core_command_use_case::orchestration::{GraphReadError, WorkflowDefinitionRepository};
use core_infrastructure::canon_json::{hash_canonical, to_value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// グラフ成果物のファイル名 (`<harnessRoot>/tools/data/` 直下)。
const STAGE_GRAPH_FILE: &str = "stage-graph.json";
/// グリッド成果物のファイル名 (同上)。
const SCOPE_GRID_FILE: &str = "scope-grid.json";
/// ハーネス identity ファイルの名前 (同上)。定義 id の供給元 (ADR-008)。
const HARNESS_FILE: &str = "harness.json";
/// scope identity ファイルの接頭辞 (`aidlc-<name>.md`)。
const SCOPE_FILE_PREFIX: &str = "aidlc-";
/// scope identity ファイルの拡張子。
const SCOPE_FILE_SUFFIX: &str = ".md";
/// frontmatter の区切り行。
const FRONTMATTER_FENCE: &str = "---";

// ---------------------------------------------------------------------------
// 逐語文言
// ---------------------------------------------------------------------------

/// `stage-graph.json` が読めないときの逐語文言 (12 §4 #1)。
///
/// `env_override` が真のときだけ hint 節が「`AIDLC_STAGE_GRAPH` を unset して既定に戻せ」形へ
/// 切り替わる — この分岐自体が観測可能な契約。
///
/// ピン留めソース採取で逐語確認済み (`aidlc-lib.ts:8565-8570` @3c3146cf、`loadStageGraphAll`
/// `:8558-8585` の内側 — `docs/specs/research/golden-3c3146cf-lib.md` §8.2)。hint 分岐の条件は
/// `process.env.AIDLC_STAGE_GRAPH` の **truthy 判定**なので、空文字列の env では既定 hint に落ちる。
/// 文言カタログ (ADR 0002) へ移すかどうかは未判断で、現状は Gateway 内に置いたままにしてある。
#[must_use]
pub fn stage_graph_not_readable_message(path: &str, cause: &str, env_override: bool) -> String {
    if env_override {
        format!(
            "Stage graph not readable at {path}: {cause}. AIDLC_STAGE_GRAPH points to {path}; unset it to use the default."
        )
    } else {
        format!(
            "Stage graph not readable at {path}: {cause}. Reinstall the framework or re-run setup to restore the data file."
        )
    }
}

/// `stage-graph.json` が不正 JSON のときの逐語文言 (12 §4 #2 — `NotReadable` とは別文言)。
///
/// ピン留めソース採取で逐語確認済み (`aidlc-lib.ts:8579-8581` @3c3146cf —
/// `docs/specs/research/golden-3c3146cf-lib.md` §8.2)。
#[must_use]
pub fn stage_graph_invalid_json_message(path: &str, cause: &str) -> String {
    format!("Stage graph at {path} is not valid JSON: {cause}")
}

/// 読取失敗を stderr へ出す 1 行に整形する (stdout は汚さない — 12 §4 #10)。
#[must_use]
pub fn graph_read_error_message(error: &GraphReadError) -> String {
    match error {
        GraphReadError::NotReadable {
            path,
            cause,
            env_override,
        } => stage_graph_not_readable_message(path, cause, *env_override),
        GraphReadError::InvalidJson { path, cause } => {
            stage_graph_invalid_json_message(path, cause)
        }
        GraphReadError::ScopeFile { message } | GraphReadError::Malformed { message } => {
            message.clone()
        }
        GraphReadError::NotFound { expected, actual } => {
            definition_not_found_message(expected, actual)
        }
        GraphReadError::HarnessIdentity { path, cause } => harness_identity_message(path, cause),
    }
}

/// 要求された定義 id がこのハーネスの定義 id と違う (ADR-008)。
///
/// upstream には定義 id の概念自体が無いため、対応する逐語文言は存在しない
/// (診断文言なので互換対象外 — `malformed` と同じ扱い)。
#[must_use]
fn definition_not_found_message(
    expected: &WorkflowDefinitionId,
    actual: &WorkflowDefinitionId,
) -> String {
    format!("Workflow definition {actual} not found: this harness provides {expected}")
}

/// ハーネス identity (`harness.json`) を読めない・`name` が無い (ADR-008)。upstream に
/// 対応する逐語文言は無い (診断文言)。
#[must_use]
fn harness_identity_message(path: &str, cause: &str) -> String {
    format!("Harness identity not readable at {path}: {cause}")
}

/// scope identity ファイルに frontmatter が無い。
///
/// ピン留めソース採取で逐語確認済み (`aidlc-lib.ts:8661` @3c3146cf —
/// `docs/specs/research/golden-3c3146cf-lib.md` §8.3)。
fn scope_missing_frontmatter_message(path: &Path) -> String {
    format!("Scope file missing frontmatter: {}", path.display())
}

/// scope identity ファイルに `name:` が無い。
///
/// ピン留めソース採取で逐語確認済み (`aidlc-lib.ts:8663` @3c3146cf —
/// `docs/specs/research/golden-3c3146cf-lib.md` §8.3)。upstream は `scalarField` が不在時に
/// 空文字列を返す実装なので `if (!name)` の 1 判定で「キー不在」と「空値」を同じ文言へ倒す。
fn scope_missing_name_message(path: &Path) -> String {
    format!(
        "Scope file {} missing required frontmatter: name",
        path.display()
    )
}

/// `skeleton:` の不正値。**[S] 逐語** (upstream 01 §355-357 に逐語あり) に加え、
/// ピン留めソース採取でも逐語確認済み (`aidlc-lib.ts:8698-8700` @3c3146cf —
/// `docs/specs/research/golden-3c3146cf-lib.md` §8.3)。
///
/// 文言カタログには置かず Gateway 内で組み立てる (カタログ移設の要否はレビュー時に判断)。
fn scope_invalid_skeleton_message(path: &Path, value: &str) -> String {
    format!(
        "Scope file {} has invalid skeleton value \"{value}\". Expected \"on\" or \"off\".",
        path.display()
    )
}

/// `review_cap:` の不正値。値域 (`adversarial` / `advisory` / `none`) は [S] 明記で、
/// 拒否文言もピン留めソース採取で逐語確認済み (`aidlc-lib.ts:8712-8714` @3c3146cf —
/// `docs/specs/research/golden-3c3146cf-lib.md` §8.3)。
fn scope_invalid_review_cap_message(path: &Path, value: &str) -> String {
    format!(
        "Scope file {} has invalid review_cap value \"{value}\". Expected \"adversarial\", \"advisory\", or \"none\".",
        path.display()
    )
}

/// 2 つの identity ファイルが同じ `name:` を宣言している (12 §3.3 — 致命)。
///
/// ピン留めソース採取で upstream 逐語を確定した (`aidlc-lib.ts:8666-8668` @3c3146cf —
/// `docs/specs/research/golden-3c3146cf-lib.md` §8.3):
///
/// ```text
/// Duplicate scope name "${name}" in ${filePath}: already declared in ${previousFile}. Rename one of them.
/// ```
///
/// 文言は上記 upstream 逐語に一致させる (D6 の既定 — 2026-08-22 裁定。当初実装の
/// `"<name>": <a> and <b>` 形は採取前の推定だったため廃止)。`filePath` = いま読んでいる
/// 重複側、`previousFile` = 先に宣言していた側。
fn scope_duplicate_name_message(name: &str, first: &Path, duplicate: &Path) -> String {
    format!(
        "Duplicate scope name \"{name}\" in {}: already declared in {}. Rename one of them.",
        duplicate.display(),
        first.display()
    )
}

/// グラフノードをドメイン型へ写せない。upstream はロード時に検証しないため対応する逐語文言は
/// 存在しない (12 §10 — 構造的パースはロード時検証の補強)。
///
/// TODO(golden: stage-0): 診断文言なので互換対象外だが、採取後に文言カタログへ載せるかを判断。
fn malformed(path: &Path, detail: &str) -> GraphReadError {
    GraphReadError::Malformed {
        message: format!("Stage graph at {}: {detail}", path.display()),
    }
}

// ---------------------------------------------------------------------------
// ワイヤ構造体 (serde — Gateway 内部部品。ドメインは serde 非依存)
// ---------------------------------------------------------------------------

/// `stage-graph.json` の 1 要素。**ルートは配列**なので `Vec<WireStageNode>` として読む。
///
/// `deny_unknown_fields` は**付けない** (F1)。`when` / `required_sections` / 予約 4 キーの
/// ようにグラフへ到達しないキーが混ざっても、単に無視される。
#[derive(Debug, Deserialize)]
struct WireStageNode {
    slug: String,
    number: String,
    name: String,
    phase: String,
    execution: String,
    mode: String,
    #[serde(default)]
    condition: String,
    #[serde(default)]
    lead_agent: String,
    #[serde(default)]
    support_agents: Vec<String>,
    #[serde(default)]
    for_each: Option<String>,
    #[serde(default)]
    workspace_requires: bool,
    #[serde(default)]
    produces: Vec<String>,
    #[serde(default)]
    optional_produces: Vec<String>,
    #[serde(default)]
    produces_kinds: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    consumes: Vec<WireConsume>,
    #[serde(default)]
    requires_stage: Vec<String>,
    #[serde(default)]
    sensors: Vec<String>,
    #[serde(default)]
    scopes: Vec<String>,
    #[serde(default)]
    reviewer: Option<String>,
    #[serde(default)]
    reviewer_max_iterations: Option<u32>,
    #[serde(default)]
    review_class: Option<String>,
    #[serde(default)]
    summary_confirmation: Option<String>,
    #[serde(default)]
    plugin: Option<String>,
    /// `None` = キー不在 = 有効 (12 §3.1)。
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    inputs: String,
    #[serde(default)]
    outputs: String,
    #[serde(default)]
    rules_in_context: Vec<WireRuleInContext>,
    #[serde(default)]
    sensors_applicable: Vec<WireSensorRef>,
}

#[derive(Debug, Deserialize)]
struct WireConsume {
    artifact: String,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    conditional_on: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WireRuleInContext {
    path: String,
    scope: String,
}

#[derive(Debug, Deserialize)]
struct WireSensorRef {
    id: String,
    path: String,
    #[serde(default)]
    matches: Option<String>,
}

/// `scope-grid.json` の 1 列。**中間の `"stages"` キーは省略できない** (F6 — レガシー
/// `mapping[scope].stages` 互換のための 2 段構造)。ドメイン側は 2 段写像だけを持つので、
/// この中間キーの知識はここに閉じる。
#[derive(Debug, Deserialize)]
struct WireScopeColumn {
    #[serde(default)]
    stages: BTreeMap<String, String>,
}

/// `harness.json` の読取に必要な部分。`harnessDir` / `rulesSubdir` は本 Repository の関心外
/// なので写さない (未知フィールドの許容は F1 と同じ方針)。
#[derive(Debug, Deserialize)]
struct WireHarness {
    #[serde(default)]
    name: String,
}

/// `DefinitionRevision` のハッシュ入力 (ADR-008)。**3 入力そのもの**を宣言順に束ねた
/// アダプタ層のワイヤ構造体で、ドメインには現れない。
///
/// フィールド順は canon-json の `to_value` が宣言順で写し、`hash_canonical` が再帰キーソートを
/// かけるため結果には効かない。順序を宣言順で固定してあるのは読み手のためである。
#[derive(Debug, Serialize)]
struct RevisionInput {
    /// `stage-graph.json` をそのまま読んだ値 (ドメイン型へ写す前)。
    stage_graph: serde_json::Value,
    /// `scope-grid.json` をそのまま読んだ値。欠損・不正時は転置導出グリッドを
    /// `{ <scope>: { stages: { <slug>: "EXECUTE"|"SKIP" } } }` 形へ直列化した値。
    scope_grid: serde_json::Value,
    /// scope identity の frontmatter を `name` 昇順に並べた配列。
    scopes: Vec<RevisionScope>,
}

/// `RevisionInput` の scope 要素 — frontmatter のうち読取モデルが保持する値。
///
/// 本 Repository が写さないキー (`description` など) はハッシュ入力にも入らない。revision は
/// 「この Repository が読んだ 3 入力」の内容版であって、ファイルの生バイトの版ではない。
#[derive(Debug, Serialize)]
struct RevisionScope {
    name: String,
    depth: Option<String>,
    keywords: Vec<String>,
    skeleton: Option<String>,
    review_cap: Option<String>,
    freeform_default: bool,
}

// ---------------------------------------------------------------------------
// Gateway
// ---------------------------------------------------------------------------

/// ファイルシステムを裏に持つ `WorkflowDefinitionRepository` の実装。
///
/// 呼出のたびに 3 入力を読み直す。キャッシュ戦略 (`OnceCell` / 注入 / 呼出ごとのロード) は
/// **観測不能なので実装の自由** (12 §10) — upstream のモジュールレベル可変シングルトンと
/// `_reset*ForTests()` は模倣しない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowDefinitionRepositoryImpl {
    data_dir: PathBuf,
    scopes_dir: PathBuf,
    stage_graph_override: Option<PathBuf>,
    scope_grid_override: Option<PathBuf>,
}

impl WorkflowDefinitionRepositoryImpl {
    /// `data_dir` は `stage-graph.json` / `scope-grid.json` の置き場
    /// (`<harnessRoot>/tools/data/`)、`scopes_dir` は identity ファイルの置き場
    /// (`<harnessRoot>/scopes/`)。
    #[must_use]
    pub const fn new(data_dir: PathBuf, scopes_dir: PathBuf) -> WorkflowDefinitionRepositoryImpl {
        WorkflowDefinitionRepositoryImpl {
            data_dir,
            scopes_dir,
            stage_graph_override: None,
            scope_grid_override: None,
        }
    }

    /// `AIDLC_STAGE_GRAPH` 相当のオーバライド。設定すると読取失敗時の逐語文言の hint 節が
    /// 「unset して既定に戻せ」形へ切り替わる (12 §4 #1)。
    #[must_use]
    pub fn with_stage_graph_override(mut self, path: PathBuf) -> WorkflowDefinitionRepositoryImpl {
        self.stage_graph_override = Some(path);
        self
    }

    /// `AIDLC_SCOPE_GRID` 相当のオーバライド。グリッドの欠損は fatal ではないため、
    /// こちらに hint 節の分岐は無い。
    #[must_use]
    pub fn with_scope_grid_override(mut self, path: PathBuf) -> WorkflowDefinitionRepositoryImpl {
        self.scope_grid_override = Some(path);
        self
    }

    /// 解決済みの `stage-graph.json` パス。
    #[must_use]
    pub fn stage_graph_path(&self) -> PathBuf {
        self.stage_graph_override
            .clone()
            .unwrap_or_else(|| self.data_dir.join(STAGE_GRAPH_FILE))
    }

    /// 解決済みの `scope-grid.json` パス。
    #[must_use]
    pub fn scope_grid_path(&self) -> PathBuf {
        self.scope_grid_override
            .clone()
            .unwrap_or_else(|| self.data_dir.join(SCOPE_GRID_FILE))
    }

    /// 解決済みの `harness.json` パス。env オーバライドは無い (upstream に対応する env が
    /// 無く、identity はハーネスの配置そのものだからである)。
    #[must_use]
    pub fn harness_path(&self) -> PathBuf {
        self.data_dir.join(HARNESS_FILE)
    }

    /// `harness.json` の `name` を定義 id として読む (ADR-008)。**fatal な入力**。
    ///
    /// 欠落・不正 JSON・`name` 欠落・id として不正、のいずれも `HarnessIdentity` に畳む。
    /// upstream には定義 id の概念が無いので、この 4 分岐に対応する逐語文言も無い。
    fn load_harness_identity(&self) -> Result<WorkflowDefinitionId, GraphReadError> {
        let path = self.harness_path();
        let identity = |cause: String| GraphReadError::HarnessIdentity {
            path: path.display().to_string(),
            cause,
        };
        let content = fs::read_to_string(&path).map_err(|e| identity(e.to_string()))?;
        let wire: WireHarness =
            serde_json::from_str(&content).map_err(|e| identity(e.to_string()))?;
        WorkflowDefinitionId::parse(&wire.name).map_err(|e| identity(e.to_string()))
    }

    /// グラフを読む。**唯一 fatal な入力** (12 §4 #1・#2)。
    ///
    /// ドメイン型の `StageGraph` と、**読んだままの生値**を返す。後者は `DefinitionRevision`
    /// のハッシュ入力で、ドメイン型へ写す過程で落ちる情報 (未知フィールド・キー順) まで
    /// 内容版に含めるために要る。
    fn load_graph(&self) -> Result<(StageGraph, serde_json::Value), GraphReadError> {
        let path = self.stage_graph_path();
        let content = fs::read_to_string(&path).map_err(|e| GraphReadError::NotReadable {
            path: path.display().to_string(),
            cause: e.to_string(),
            env_override: self.stage_graph_override.is_some(),
        })?;
        let invalid_json = |e: &serde_json::Error| GraphReadError::InvalidJson {
            path: path.display().to_string(),
            cause: e.to_string(),
        };
        let raw: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| invalid_json(&e))?;
        let wire: Vec<WireStageNode> =
            serde_json::from_str(&content).map_err(|e| invalid_json(&e))?;
        let mut nodes = Vec::with_capacity(wire.len());
        for node in wire {
            nodes.push(to_stage_node(node, &path)?);
        }
        // 文書順のまま渡す (F2 — 読込時に数値順へ正規化しない)。
        let graph = StageGraph::new(nodes).map_err(|e| malformed(&path, &format!("{e:?}")))?;
        Ok((graph, raw))
    }

    /// グリッドを読む。**読めない / 不正なら `None`** を返し、呼出側が転置導出へ倒す
    /// (12 §4 #3 — *"callers never see a hard ENOENT for a derivable artifact"*)。
    ///
    /// ドメイン型の `ScopeGrid` と読んだままの生値を返す (生値は revision のハッシュ入力)。
    fn load_grid(&self) -> Option<(ScopeGrid, serde_json::Value)> {
        let content = fs::read_to_string(self.scope_grid_path()).ok()?;
        let raw: serde_json::Value = serde_json::from_str(&content).ok()?;
        let wire: BTreeMap<String, WireScopeColumn> = serde_json::from_str(&content).ok()?;
        let mut columns: BTreeMap<String, BTreeMap<StageSlug, PlanAction>> = BTreeMap::new();
        for (scope, column) in wire {
            let mut cells: BTreeMap<StageSlug, PlanAction> = BTreeMap::new();
            for (slug, action) in column.stages {
                // 文法外 slug・`EXECUTE`/`SKIP` 以外の値はセルごと落とす。結果は 3 値契約の
                // `None` (=「このグリッドがコンパイルしていないステージ」) になり、
                // upstream の「列に slug が無い」と同じ観測になる (F8)。
                // 全体を転置導出へ倒さないのは、1 セルの異常でグリッド全体を捨てないため。
                if let (Ok(slug), Some(action)) =
                    (StageSlug::parse(&slug), PlanAction::parse(&action))
                {
                    cells.insert(slug, action);
                }
            }
            columns.insert(scope, cells);
        }
        Some((ScopeGrid::new(columns), raw))
    }

    /// identity ファイルを列挙して frontmatter を読む。**有効スコープの権威**はここ (F7)。
    ///
    /// ディレクトリ自体が無い場合は空カタログとして扱う (グラフと違い fatal にしない)。
    /// TODO(spec: 12 §11): scopes ディレクトリ欠損時の態度は upstream 側の裏取りが未了。
    fn load_scopes(&self) -> Result<BTreeMap<String, ScopeMetadata>, GraphReadError> {
        let paths = match scope_file_paths(&self.scopes_dir) {
            Ok(paths) => paths,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => {
                return Err(GraphReadError::ScopeFile {
                    message: format!("{}: {e}", self.scopes_dir.display()),
                });
            }
        };
        let mut scopes: BTreeMap<String, ScopeMetadata> = BTreeMap::new();
        let mut origins: BTreeMap<String, PathBuf> = BTreeMap::new();
        for path in paths {
            let content = fs::read_to_string(&path).map_err(|e| GraphReadError::ScopeFile {
                message: format!("{}: {e}", path.display()),
            })?;
            let metadata = parse_scope_metadata(&path, &content)?;
            let name = metadata.name().to_string();
            if let Some(first) = origins.get(&name) {
                return Err(GraphReadError::ScopeFile {
                    message: scope_duplicate_name_message(&name, first, &path),
                });
            }
            origins.insert(name.clone(), path);
            scopes.insert(name, metadata);
        }
        Ok(scopes)
    }
}

impl WorkflowDefinitionRepository for WorkflowDefinitionRepositoryImpl {
    fn find_by_id(&self, id: &WorkflowDefinitionId) -> Result<WorkflowDefinition, GraphReadError> {
        // 識別子の検査が先。1 ハーネス 1 定義なので、要求 id が違えば 3 入力を読む意味がない
        // (BR2.6 — expected = 提供できる id、actual = 要求された id)。
        let harness_id = self.load_harness_identity()?;
        if &harness_id != id {
            return Err(GraphReadError::NotFound {
                expected: harness_id,
                actual: id.clone(),
            });
        }

        let (graph, raw_graph) = self.load_graph()?;
        let (grid, raw_grid) = match self.load_grid() {
            Some(read) => read,
            // グリッド欠損は fatal にしない (12 §4 #3)。revision も導出グリッドから作る —
            // 「読めた 3 入力の内容版」であって「ディスクにあったバイトの版」ではない。
            None => {
                let derived = ScopeGrid::from_graph(&graph);
                let raw = serialize_grid(&derived);
                (derived, raw)
            }
        };
        let scopes = self.load_scopes()?;
        let revision = compute_revision(&raw_graph, &raw_grid, &scopes)?;

        Ok(WorkflowDefinition::from_artifacts(
            harness_id, revision, graph, grid, scopes,
        ))
    }
}

/// 転置導出グリッドを `scope-grid.json` と同じ 2 段構造へ直列化する
/// (`{ <scope>: { stages: { <slug>: "EXECUTE"|"SKIP" } } }` — F6 の中間キー込み)。
///
/// グリッドが読めなかったときの revision 入力。ファイルから読めたときの値と同じ形にして
/// おくことで、「導出グリッドと同じ内容の grid ファイルが置かれた」場合に同じ revision に
/// なる — 内容版が入力の**内容**だけで決まるという性質が保たれる。
fn serialize_grid(grid: &ScopeGrid) -> serde_json::Value {
    let mut columns = serde_json::Map::new();
    for scope in grid.scope_names() {
        let mut stages = serde_json::Map::new();
        if let Some(column) = grid.column(scope) {
            for (slug, action) in column {
                stages.insert(
                    slug.as_str().to_string(),
                    serde_json::Value::String(action.as_str().to_string()),
                );
            }
        }
        let mut wrapper = serde_json::Map::new();
        wrapper.insert("stages".to_string(), serde_json::Value::Object(stages));
        columns.insert(scope.to_string(), serde_json::Value::Object(wrapper));
    }
    serde_json::Value::Object(columns)
}

/// 3 入力の正準 JSON ダイジェストを `DefinitionRevision` にする (ADR-008)。
///
/// `hash_canonical` は再帰キーソート + `sha256:` 接頭辞 (正準族) なので、入力の**内容**だけで
/// 決まりキーの並び順には依存しない。scope は `BTreeMap` から取るため常に `name` 昇順。
fn compute_revision(
    raw_graph: &serde_json::Value,
    raw_grid: &serde_json::Value,
    scopes: &BTreeMap<String, ScopeMetadata>,
) -> Result<DefinitionRevision, GraphReadError> {
    let input = RevisionInput {
        stage_graph: raw_graph.clone(),
        scope_grid: raw_grid.clone(),
        scopes: scopes
            .values()
            .map(|metadata| RevisionScope {
                name: metadata.name().to_string(),
                depth: metadata.depth().map(str::to_string),
                keywords: metadata.keywords().to_vec(),
                skeleton: metadata.skeleton().map(|s| s.as_str().to_string()),
                review_cap: metadata.review_cap().map(|c| c.as_str().to_string()),
                freeform_default: metadata.freeform_default(),
            })
            .collect(),
    };
    let value = to_value(&input).map_err(|e| GraphReadError::Malformed {
        message: format!("definition revision input: {e}"),
    })?;
    DefinitionRevision::parse(&hash_canonical(&value).rendered()).map_err(|e| {
        GraphReadError::Malformed {
            message: format!("definition revision: {e}"),
        }
    })
}

// ---------------------------------------------------------------------------
// ワイヤ → ドメインの写像
// ---------------------------------------------------------------------------

fn to_stage_node(wire: WireStageNode, path: &Path) -> Result<StageNode, GraphReadError> {
    let slug = StageSlug::parse(&wire.slug)
        .map_err(|e| malformed(path, &format!("invalid slug {:?} ({e:?})", wire.slug)))?;
    let number = StageNumber::parse(&wire.number).map_err(|e| {
        malformed(
            path,
            &format!(
                "stage {:?} has invalid number {:?} ({e:?})",
                wire.slug, wire.number
            ),
        )
    })?;
    let phase = PhaseId::parse(&wire.phase).map_err(|e| {
        malformed(
            path,
            &format!("stage {:?} has unknown phase ({e:?})", wire.slug),
        )
    })?;
    let execution = ExecutionKind::parse(&wire.execution).map_err(|e| {
        malformed(
            path,
            &format!("stage {:?} has unknown execution ({e:?})", wire.slug),
        )
    })?;
    let mode = StageMode::parse(&wire.mode).map_err(|e| {
        malformed(
            path,
            &format!("stage {:?} has unknown mode ({e:?})", wire.slug),
        )
    })?;

    let mut consumes = Vec::with_capacity(wire.consumes.len());
    for decl in wire.consumes {
        let conditional_on = match decl.conditional_on {
            None => None,
            Some(raw) => Some(BrownfieldGreenfield::parse(&raw).map_err(|e| {
                malformed(
                    path,
                    &format!("stage {:?} has unknown conditional_on ({e:?})", wire.slug),
                )
            })?),
        };
        consumes.push(ConsumeDecl::new(
            decl.artifact,
            decl.required,
            conditional_on,
        ));
    }

    let mut requires_stage = Vec::with_capacity(wire.requires_stage.len());
    for dep in &wire.requires_stage {
        requires_stage.push(StageSlug::parse(dep).map_err(|e| {
            malformed(
                path,
                &format!(
                    "stage {:?} requires invalid slug {dep:?} ({e:?})",
                    wire.slug
                ),
            )
        })?);
    }

    let mut rules_in_context = Vec::with_capacity(wire.rules_in_context.len());
    for rule in wire.rules_in_context {
        let scope = RuleScope::parse(&rule.scope).map_err(|e| {
            malformed(
                path,
                &format!("stage {:?} has unknown rule scope ({e:?})", wire.slug),
            )
        })?;
        rules_in_context.push(RuleInContext::new(rule.path, scope));
    }

    let review_class = match wire.review_class {
        None => None,
        Some(ref raw) => Some(ReviewClass::parse(raw).map_err(|e| {
            malformed(
                path,
                &format!("stage {:?} has unknown review_class ({e:?})", wire.slug),
            )
        })?),
    };

    let sensors_applicable = wire
        .sensors_applicable
        .into_iter()
        .map(|s| SensorRef::new(s.id, s.path, s.matches))
        .collect();

    let mut builder = StageNodeBuilder::new(slug, number, wire.name, phase, execution, mode)
        .condition(wire.condition)
        .lead_agent(wire.lead_agent)
        .support_agents(wire.support_agents)
        .workspace_requires(wire.workspace_requires)
        .produces(wire.produces)
        .optional_produces(wire.optional_produces)
        .produces_kinds(wire.produces_kinds)
        .consumes(consumes)
        .requires_stage(requires_stage)
        .sensors(wire.sensors)
        .scopes(wire.scopes)
        .inputs(wire.inputs)
        .outputs(wire.outputs)
        .rules_in_context(rules_in_context)
        .sensors_applicable(sensors_applicable);
    if let Some(v) = wire.for_each {
        builder = builder.for_each(v);
    }
    if let Some(v) = wire.reviewer {
        builder = builder.reviewer(v);
    }
    if let Some(v) = wire.reviewer_max_iterations {
        builder = builder.reviewer_max_iterations(v);
    }
    if let Some(v) = review_class {
        builder = builder.review_class(v);
    }
    if let Some(v) = wire.summary_confirmation {
        builder = builder.summary_confirmation(v);
    }
    if let Some(v) = wire.plugin {
        builder = builder.plugin(v);
    }
    if let Some(v) = wire.enabled {
        builder = builder.enabled(v);
    }
    Ok(builder.build())
}

// ---------------------------------------------------------------------------
// scope identity ファイル (手書き frontmatter パーサ — 00-policy R9)
// ---------------------------------------------------------------------------

/// `<scopes_dir>/aidlc-*.md` をパス昇順で列挙する (重複 `name:` 検出を決定的にするため)。
fn scope_file_paths(scopes_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(scopes_dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if file_name.starts_with(SCOPE_FILE_PREFIX) && file_name.ends_with(SCOPE_FILE_SUFFIX) {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

/// frontmatter の最小 YAML サブセットを手書きで読む。
///
/// 受理する形は `---` で挟まれた `key: value` 行と `keywords: [a, b]` のフロー列だけで、
/// 未知キー (`description` / `testStrategy` / `runner` / `plugin` 等) は黙って無視する。
/// 汎用 YAML パーサへ置換すると寛容パースと逐語拒否文言の契約が静かに変わる (12 §3.3)。
fn parse_scope_metadata(path: &Path, content: &str) -> Result<ScopeMetadata, GraphReadError> {
    let body = frontmatter_body(content).ok_or_else(|| GraphReadError::ScopeFile {
        message: scope_missing_frontmatter_message(path),
    })?;

    let mut name: Option<String> = None;
    let mut depth: Option<String> = None;
    let mut keywords: Vec<String> = Vec::new();
    let mut skeleton: Option<SkeletonDefault> = None;
    let mut review_cap: Option<ReviewCapValue> = None;
    let mut freeform_default = false;

    for line in body.lines() {
        // インデントされた行は未知キーのブロックマッピング配下 (入れ子) の中身であり、
        // トップレベルキーとして解釈しない — trim してから split すると `plugin:` 配下の
        // `name: acme` が `name` を上書きし、寛容パース (「未知キーは黙って無視」) が壊れる。
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, raw)) = trimmed.split_once(':') else {
            continue;
        };
        let value = unquote(raw.trim());
        match key.trim() {
            "name" => name = Some(value.to_string()),
            "depth" => depth = Some(value.to_string()),
            "keywords" => keywords = parse_flow_sequence(value),
            "skeleton" => {
                skeleton =
                    Some(
                        SkeletonDefault::parse(value).map_err(|_| GraphReadError::ScopeFile {
                            message: scope_invalid_skeleton_message(path, value),
                        })?,
                    );
            }
            "review_cap" => {
                review_cap =
                    Some(
                        ReviewCapValue::parse(value).map_err(|_| GraphReadError::ScopeFile {
                            message: scope_invalid_review_cap_message(path, value),
                        })?,
                    );
            }
            // 有効スコープ中 1 つまでという集合レベルの一意性はスライス 1 の範囲外。
            // TODO(spec: 12 §3.3): `freeform_default` の集合一意性検証は compile 側と併せて実装する。
            "freeform_default" => freeform_default = value == "true",
            _ => {}
        }
    }

    let name = name.ok_or_else(|| GraphReadError::ScopeFile {
        message: scope_missing_name_message(path),
    })?;
    let mut metadata = ScopeMetadata::new(&name).map_err(|_| GraphReadError::ScopeFile {
        message: scope_missing_name_message(path),
    })?;
    if let Some(depth) = depth {
        metadata = metadata.with_depth(depth);
    }
    if !keywords.is_empty() {
        metadata = metadata.with_keywords(keywords);
    }
    if let Some(skeleton) = skeleton {
        metadata = metadata.with_skeleton(skeleton);
    }
    if let Some(review_cap) = review_cap {
        metadata = metadata.with_review_cap(review_cap);
    }
    Ok(metadata.with_freeform_default(freeform_default))
}

/// 先頭の `---` から次の `---` までを返す。どちらかが無ければ `None` (= frontmatter 無し)。
fn frontmatter_body(content: &str) -> Option<&str> {
    let rest = content.strip_prefix(FRONTMATTER_FENCE)?;
    // 開始フェンス行の残り (改行まで) は捨てる。
    let rest = rest
        .strip_prefix("\r\n")
        .or_else(|| rest.strip_prefix('\n'))?;
    let mut offset = 0usize;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == FRONTMATTER_FENCE {
            return Some(&rest[..offset]);
        }
        offset += line.len();
    }
    None
}

/// `[a, b]` のフロー列を読む。角括弧が無い形は「1 要素の列」として寛容に受ける。
fn parse_flow_sequence(value: &str) -> Vec<String> {
    let inner = match value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
        Some(inner) => inner,
        None => value,
    };
    inner
        .split(',')
        .map(|item| unquote(item.trim()).to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

/// 前後を囲う `"` / `'` を 1 組だけ剥がす。
fn unquote(value: &str) -> &str {
    for quote in ['"', '\''] {
        if value.len() >= 2 && value.starts_with(quote) && value.ends_with(quote) {
            return &value[1..value.len() - 1];
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn scope_path() -> &'static Path {
        Path::new("/scopes/aidlc-feature.md")
    }

    #[test]
    fn the_not_readable_hint_clause_switches_on_the_env_override() {
        let plain = stage_graph_not_readable_message("/d/stage-graph.json", "ENOENT", false);
        assert!(plain.starts_with("Stage graph not readable at /d/stage-graph.json: ENOENT."));
        assert!(
            plain.ends_with("Reinstall the framework or re-run setup to restore the data file.")
        );

        let overridden = stage_graph_not_readable_message("/x.json", "ENOENT", true);
        assert!(
            overridden
                .ends_with("AIDLC_STAGE_GRAPH points to /x.json; unset it to use the default.")
        );
    }

    #[test]
    fn invalid_json_has_its_own_wording() {
        assert_eq!(
            stage_graph_invalid_json_message("/d/stage-graph.json", "eof"),
            "Stage graph at /d/stage-graph.json is not valid JSON: eof"
        );
    }

    #[test]
    fn graph_read_error_message_renders_every_variant() {
        let rendered = graph_read_error_message(&GraphReadError::ScopeFile {
            message: "boom".to_string(),
        });
        assert_eq!(rendered, "boom");
        let rendered = graph_read_error_message(&GraphReadError::Malformed {
            message: "bang".to_string(),
        });
        assert_eq!(rendered, "bang");
        let rendered = graph_read_error_message(&GraphReadError::InvalidJson {
            path: "/p".to_string(),
            cause: "c".to_string(),
        });
        assert_eq!(rendered, "Stage graph at /p is not valid JSON: c");
        let rendered = graph_read_error_message(&GraphReadError::NotReadable {
            path: "/p".to_string(),
            cause: "c".to_string(),
            env_override: false,
        });
        assert!(rendered.starts_with("Stage graph not readable at /p: c."));
    }

    #[test]
    fn frontmatter_needs_both_fences() {
        assert_eq!(
            frontmatter_body("---\nname: a\n---\nbody\n"),
            Some("name: a\n")
        );
        assert_eq!(
            frontmatter_body("---\r\nname: a\r\n---\r\n"),
            Some("name: a\r\n")
        );
        assert_eq!(frontmatter_body("name: a\n"), None);
        assert_eq!(frontmatter_body("---\nname: a\n"), None);
    }

    #[test]
    fn name_is_the_only_required_key_and_unknown_keys_are_ignored() {
        let metadata = parse_scope_metadata(
            scope_path(),
            "---\nname: feature\ndescription: anything\nrunner: cargo\n---\n# body\n",
        )
        .unwrap();
        assert_eq!(metadata.name(), "feature");
        assert_eq!(metadata.depth(), None);
        assert!(metadata.keywords().is_empty());
    }

    #[test]
    fn missing_frontmatter_and_missing_name_have_distinct_wordings() {
        let err = parse_scope_metadata(scope_path(), "no frontmatter here\n").unwrap_err();
        assert_eq!(
            err,
            GraphReadError::ScopeFile {
                message: "Scope file missing frontmatter: /scopes/aidlc-feature.md".to_string()
            }
        );
        let err = parse_scope_metadata(scope_path(), "---\ndepth: standard\n---\n").unwrap_err();
        assert_eq!(
            err,
            GraphReadError::ScopeFile {
                message: "Scope file /scopes/aidlc-feature.md missing required frontmatter: name"
                    .to_string()
            }
        );
        let err = parse_scope_metadata(scope_path(), "---\nname: \"\"\n---\n").unwrap_err();
        assert!(matches!(err, GraphReadError::ScopeFile { .. }));
    }

    #[test]
    fn indented_lines_under_an_unknown_block_key_are_not_top_level_keys() {
        // 未知キー `plugin:` のブロック配下に name / skeleton が現れても、トップレベルの
        // name を上書きせず、skeleton の不正値検査にも掛からない (寛容パースの契約)。
        let metadata = parse_scope_metadata(
            scope_path(),
            "---\nname: feature\nplugin:\n  name: acme\n  skeleton: enabled\n---\n",
        )
        .unwrap();
        assert_eq!(metadata.name(), "feature");
        assert_eq!(metadata.skeleton(), None);
    }

    #[test]
    fn the_skeleton_rejection_is_verbatim() {
        let err = parse_scope_metadata(scope_path(), "---\nname: feature\nskeleton: yes\n---\n")
            .unwrap_err();
        assert_eq!(
            err,
            GraphReadError::ScopeFile {
                message: "Scope file /scopes/aidlc-feature.md has invalid skeleton value \"yes\". Expected \"on\" or \"off\"."
                    .to_string()
            }
        );
    }

    #[test]
    fn review_cap_accepts_the_three_declared_values_and_rejects_the_rest() {
        for (raw, expected) in [
            ("adversarial", ReviewCapValue::Adversarial),
            ("advisory", ReviewCapValue::Advisory),
            ("none", ReviewCapValue::None),
        ] {
            let metadata = parse_scope_metadata(
                scope_path(),
                &format!("---\nname: feature\nreview_cap: {raw}\n---\n"),
            )
            .unwrap();
            assert_eq!(metadata.review_cap(), Some(expected));
        }
        let err = parse_scope_metadata(
            scope_path(),
            "---\nname: feature\nreview_cap: strict\n---\n",
        )
        .unwrap_err();
        assert!(
            matches!(err, GraphReadError::ScopeFile { message } if message.contains("review_cap"))
        );
    }

    #[test]
    fn keywords_read_the_flow_sequence_and_tolerate_a_bare_scalar() {
        let metadata = parse_scope_metadata(
            scope_path(),
            "---\nname: feature\nkeywords: [\"api\", endpoint , ]\n---\n",
        )
        .unwrap();
        assert_eq!(
            metadata.keywords(),
            ["api".to_string(), "endpoint".to_string()]
        );

        let metadata =
            parse_scope_metadata(scope_path(), "---\nname: feature\nkeywords: api\n---\n").unwrap();
        assert_eq!(metadata.keywords(), ["api".to_string()]);

        let metadata =
            parse_scope_metadata(scope_path(), "---\nname: feature\nkeywords: []\n---\n").unwrap();
        assert!(metadata.keywords().is_empty());
    }

    #[test]
    fn freeform_default_is_true_only_for_the_literal_true() {
        for (raw, expected) in [("true", true), ("false", false), ("yes", false)] {
            let metadata = parse_scope_metadata(
                scope_path(),
                &format!("---\nname: feature\nfreeform_default: {raw}\n---\n"),
            )
            .unwrap();
            assert_eq!(metadata.freeform_default(), expected);
        }
    }

    #[test]
    fn path_resolution_prefers_the_overrides() {
        let reader =
            WorkflowDefinitionRepositoryImpl::new(PathBuf::from("/data"), PathBuf::from("/scopes"));
        assert_eq!(
            reader.stage_graph_path(),
            PathBuf::from("/data/stage-graph.json")
        );
        assert_eq!(
            reader.scope_grid_path(),
            PathBuf::from("/data/scope-grid.json")
        );

        let reader = reader
            .with_stage_graph_override(PathBuf::from("/pinned/graph.json"))
            .with_scope_grid_override(PathBuf::from("/pinned/grid.json"));
        assert_eq!(
            reader.stage_graph_path(),
            PathBuf::from("/pinned/graph.json")
        );
        assert_eq!(reader.scope_grid_path(), PathBuf::from("/pinned/grid.json"));
    }

    #[test]
    fn unquote_strips_only_one_matching_pair() {
        assert_eq!(unquote("\"a\""), "a");
        assert_eq!(unquote("'a'"), "a");
        assert_eq!(unquote("\"a"), "\"a");
        assert_eq!(unquote("''"), "");
        assert_eq!(unquote("a"), "a");
    }

    #[test]
    fn the_two_identity_failures_have_their_own_diagnostic_wordings() {
        // ADR-008 で足した 2 変種。upstream には定義 id の概念が無いので逐語の縛りは
        // 無いが、`graph_read_error_message` が 1 行の診断へ畳むところまでが契約である。
        let rendered = graph_read_error_message(&GraphReadError::NotFound {
            expected: WorkflowDefinitionId::parse("claude").unwrap(),
            actual: WorkflowDefinitionId::parse("codex").unwrap(),
        });
        assert_eq!(
            rendered,
            "Workflow definition codex not found: this harness provides claude"
        );

        let rendered = graph_read_error_message(&GraphReadError::HarnessIdentity {
            path: "/d/harness.json".to_string(),
            cause: "missing name".to_string(),
        });
        assert_eq!(
            rendered,
            "Harness identity not readable at /d/harness.json: missing name"
        );
    }

    #[test]
    fn blank_comment_and_keyless_lines_are_skipped_instead_of_breaking_the_parse() {
        // frontmatter の最小 YAML サブセットは、空行・`#` コメント・`:` を持たない行を
        // 黙って読み飛ばす (寛容パースの契約 — 汎用 YAML パーサへ置換しない理由の 1 つ)。
        let metadata = parse_scope_metadata(
            scope_path(),
            "---\n\nname: feature\n# コメント行\n  \nbare-line-without-a-colon\ndepth: standard\n---\n",
        )
        .unwrap();
        assert_eq!(metadata.name(), "feature");
        assert_eq!(metadata.depth(), Some("standard"));
    }
}
