//! `DefinitionArtifactsClient` の実装 (Gateway) — ハーネス配布物 (Published Language 3 入力:
//! `stage-graph.json` / `scope-grid.json` / `<harnessRoot>/scopes/aidlc-<name>.md`) を
//! ディスクから読み、定義を確立するための材料へ写す (12-workflow-definition §6)。
//!
//! # ここは Repository ではない (2026-08-31 オーナー裁定)
//!
//! かつてこのコードは `WorkflowDefinitionRepositoryImpl` として、3 入力から集約を組み立てて
//! いた。その実装は破棄された (「NG 中の NG です。リポジトリの実装は EventStoreForSqlite を
//! 使わないといけない」) — 集約の最新状態をファイルから組み立てるのは
//! `coding-rules/cqrs-boundaries.md` 規則 4 への正面違反だからである。
//!
//! パースの中身はそのまま**取込境界**へ移した。相手は外部システム (upstream の compile が
//! 出力してハーネスと一緒に配ったバイト) であり、責務は Gateway 2 分類のうち**外部システム
//! クライアント**である (`coding-rules/gateway-taxonomy.md` §1)。読んだ材料から定義を確立・
//! 改訂して**イベントストアへ書く**のは `DefineWorkflowUseCase` の仕事で、以後の集約の読取は
//! 常にジャーナルからの再構成になる。
//!
//! **この実装が所有するもの** (12 §6):
//! - パス解決とテストシーム (`<data_dir>/{stage-graph,scope-grid}.json` / `<scopes_dir>`、
//!   および `AIDLC_STAGE_GRAPH` / `AIDLC_SCOPE_GRID` 相当のオーバライド)。
//!   **env の読取そのものは合成ルートの責務**で、ここは注入されたパスだけを見る
//!   (テストを hermetic に保つため)。
//! - JSON コーデック (serde ワイヤ構造体) と frontmatter パーサ (手書き — 00-policy R9)。
//! - 内容版 `DefinitionRevision` の算出 (正準 JSON ダイジェスト — ADR-008)。
//!
//! **serde の厳格度** (12 §10 の表):
//! 1. 未知フィールドは**許容**する (`deny_unknown_fields` を付けない — F1)。将来版や
//!    プラグインが `FIELD_ORDER` を増やしても読めなくなってはならない。
//! 2. 欠損 optional は `Option` ないし空 default (`#[serde(default)]`)。
//! 3. 未知の列挙値は**全列挙 (`phase` / `execution` / `review_class` / `mode`) を load 時に
//!    厳密 enum で落とす** (12 §10 表 #3 — 2026-08-22 裁定)。ドメイン型に `Unknown` variant を
//!    持たせず Always Valid を維持する。upstream との観測差は手編集グラフの未知値に限られ、
//!    dist の正規データでは生じない — ピン留め `3c3146cf` の配布実バイト 33 ノードが全数
//!    取り込めることは `tests/golden_parity_test.rs` が固定した。
//!
//! **失敗態度** (12 §4): グラフは fatal、グリッドは転置導出フォールバック、identity と
//! グリッド列の不一致は双方向とも正当。

use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, ConsumeDecl, DefinitionRevision, ExecutionKind, PhaseId, PlanAction,
    ReviewCapValue, ReviewClass, RuleInContext, RuleScope, ScopeGrid, ScopeMetadata, SensorRef,
    SkeletonDefault, StageGraph, StageMode, StageNode, StageNodeBuilder, StageNumber, StageSlug,
    WorkflowDefinitionId,
};
use core_command_use_case::orchestration::{
    DefinitionArtifacts, DefinitionArtifactsClient, DefinitionArtifactsError,
};
use core_infrastructure::canon_json::{hash_canonical, to_value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
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
// 失敗 (アダプタ私有 — ポート契約には分類を載せない)
// ---------------------------------------------------------------------------

/// 3 入力の読取失敗 (この実装の内部中間表現)。
#[derive(Debug)]
enum DefinitionReadFailure {
    /// OS 由来の読取失敗 (欠損・権限・種別違い)。**内容の破損ではない**。
    Io {
        /// OS 由来の分類。
        kind: io::ErrorKind,
        /// 読もうとしたパス。
        path: PathBuf,
    },
    /// 読めたが内容が壊れている。
    Corrupt(DefinitionCorruption),
}

impl DefinitionReadFailure {
    /// OS の失敗から起こす。
    fn from_io(path: &Path, error: &io::Error) -> DefinitionReadFailure {
        DefinitionReadFailure::Io {
            kind: error.kind(),
            path: path.to_path_buf(),
        }
    }

    /// ポート契約のエラーへ写す。
    ///
    /// `Corrupt` の分類は契約に載せない (裁定 6) — 原因は `Error::source` の連鎖で
    /// 診断表示だけを運ぶ。
    fn into_artifacts_error(self) -> DefinitionArtifactsError {
        match self {
            DefinitionReadFailure::Io { kind, path } => DefinitionArtifactsError::Io { kind, path },
            DefinitionReadFailure::Corrupt(cause) => DefinitionArtifactsError::Corrupt {
                source: Box::new(cause),
            },
        }
    }
}

/// 「読めたが内容が壊れている」の原因 (アダプタ私有)。
///
/// ポート契約は「壊れていた」としか約束しないので、本型は `Corrupt` の `source` として
/// **診断表示だけ**を運ぶ (`coding-rules/error-handling.md` — エラーは契約の一部であり、
/// 内部実装がバレる情報を含めない)。
///
/// **upstream 逐語文言はここには無い。** 12 §4 / §6 の利用者向け文言 (「Stage graph not
/// readable at ...」等) を所有するのは**クエリ側**の読取実装
/// (`core_query_interface_adapter::workflow_definition_parse`) である — 定義 3 入力は
/// リードモデルであり、それを読んで人に見せるのはクエリ側の仕事だからである
/// (`coding-rules/cqrs-boundaries.md` 規則 7、b26 段階 2)。ここに残るのは同じ材料を
/// 開発者向けに 1 行へ畳んだ**診断**であって、互換対象ではない。
#[derive(Debug)]
enum DefinitionCorruption {
    /// `stage-graph.json` が不正 JSON (12 §4 #2)。
    InvalidJson {
        /// パースに失敗したパス。
        path: PathBuf,
        /// JSON パーサ由来の理由。
        cause: String,
    },
    /// `harness.json` は読めたが定義 id を与えない (不正 JSON / `name` 欠落 / id として不正
    /// — ADR-008)。
    HarnessIdentity {
        /// 読んだパス。
        path: PathBuf,
        /// JSON パーサないし id の形式検証由来の理由。
        cause: String,
    },
    /// scope identity ファイルの frontmatter 検証の失敗 (`name` 欠落・`skeleton` の
    /// 不正値・名前の重複など — 12 §3.3)。
    ScopeFile {
        /// 失敗の詳細 (材料)。
        message: String,
    },
    /// JSON としては読めたがドメイン型へ写せない (未知 `phase`、文法外 `slug` など)。
    ///
    /// upstream はロード時に検証しないが、serde による構造的パースは「ロード時無検証」からの
    /// 逸脱ではなく補強として扱う (12 §10) — dist の正規データに対しては観測差が生じない。
    Malformed {
        /// 写像に失敗した箇所の詳細 (材料)。
        message: String,
    },
}

impl fmt::Display for DefinitionCorruption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DefinitionCorruption::InvalidJson { path, cause } => write!(
                f,
                "stage graph at {} is not valid JSON: {cause}",
                path.display()
            ),
            DefinitionCorruption::HarnessIdentity { path, cause } => write!(
                f,
                "harness identity at {} does not provide a definition id: {cause}",
                path.display()
            ),
            DefinitionCorruption::ScopeFile { message }
            | DefinitionCorruption::Malformed { message } => f.write_str(message),
        }
    }
}

impl Error for DefinitionCorruption {}

/// scope identity ファイルに frontmatter が無い。
fn scope_missing_frontmatter_message(path: &Path) -> String {
    format!("Scope file missing frontmatter: {}", path.display())
}

/// scope identity ファイルに `name:` が無い。「キー不在」と「空値」は同じ材料へ倒す。
fn scope_missing_name_message(path: &Path) -> String {
    format!(
        "Scope file {} missing required frontmatter: name",
        path.display()
    )
}

/// `skeleton:` の不正値 (値域は `on` / `off`)。
fn scope_invalid_skeleton_message(path: &Path, value: &str) -> String {
    format!(
        "Scope file {} has invalid skeleton value \"{value}\". Expected \"on\" or \"off\".",
        path.display()
    )
}

/// `review_cap:` の不正値 (値域は `adversarial` / `advisory` / `none`)。
fn scope_invalid_review_cap_message(path: &Path, value: &str) -> String {
    format!(
        "Scope file {} has invalid review_cap value \"{value}\". Expected \"adversarial\", \"advisory\", or \"none\".",
        path.display()
    )
}

/// 2 つの identity ファイルが同じ `name:` を宣言している (12 §3.3 — 致命)。
///
/// `duplicate` = いま読んでいる重複側、`first` = 先に宣言していた側。どちらを直せばよいかが
/// 分かるよう両方を材料に載せる。
fn scope_duplicate_name_message(name: &str, first: &Path, duplicate: &Path) -> String {
    format!(
        "Duplicate scope name \"{name}\" in {}: already declared in {}. Rename one of them.",
        duplicate.display(),
        first.display()
    )
}

/// グラフノードをドメイン型へ写せない (12 §10 — 構造的パースはロード時検証の補強)。
fn malformed(path: &Path, detail: &str) -> DefinitionReadFailure {
    DefinitionReadFailure::Corrupt(DefinitionCorruption::Malformed {
        message: format!("Stage graph at {}: {detail}", path.display()),
    })
}

/// scope identity の検証失敗を畳む。
const fn scope_file(message: String) -> DefinitionReadFailure {
    DefinitionReadFailure::Corrupt(DefinitionCorruption::ScopeFile { message })
}

// ---------------------------------------------------------------------------
// ワイヤ構造体 (serde — Gateway 内部部品。ドメインは serde 非依存)
// ---------------------------------------------------------------------------

/// `stage-graph.json` の 1 要素。**ルートは配列**なので `Vec<StageNodeDto>` として読む。
///
/// `deny_unknown_fields` は**付けない** (F1)。`when` / `required_sections` / 予約 4 キーの
/// ようにグラフへ到達しないキーが混ざっても、単に無視される。
#[derive(Debug, Deserialize)]
struct StageNodeDto {
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
    consumes: Vec<ConsumeDto>,
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
    rules_in_context: Vec<RuleInContextDto>,
    #[serde(default)]
    sensors_applicable: Vec<SensorRefDto>,
}

#[derive(Debug, Deserialize)]
struct ConsumeDto {
    artifact: String,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    conditional_on: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RuleInContextDto {
    path: String,
    scope: String,
}

#[derive(Debug, Deserialize)]
struct SensorRefDto {
    id: String,
    path: String,
    #[serde(default)]
    matches: Option<String>,
}

/// `scope-grid.json` の 1 列。**中間の `"stages"` キーは省略できない** (F6 — レガシー
/// `mapping[scope].stages` 互換のための 2 段構造)。ドメイン側は 2 段写像だけを持つので、
/// この中間キーの知識はここに閉じる。
#[derive(Debug, Deserialize)]
struct ScopeColumnDto {
    #[serde(default)]
    stages: BTreeMap<String, String>,
}

/// `harness.json` の読取に必要な部分。`harnessDir` / `rulesSubdir` は本 Gateway の関心外
/// なので写さない (未知フィールドの許容は F1 と同じ方針)。
#[derive(Debug, Deserialize)]
struct HarnessDto {
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
/// 本 Gateway が写さないキー (`description` など) はハッシュ入力にも入らない。revision は
/// 「この Gateway が読んだ 3 入力」の内容版であって、ファイルの生バイトの版ではない。
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

/// ファイルシステムを裏に持つ `DefinitionArtifactsClient` の実装。
///
/// 呼出のたびに 3 入力を読み直す。キャッシュ戦略 (`OnceCell` / 注入 / 呼出ごとのロード) は
/// **観測不能なので実装の自由** (12 §10) — upstream のモジュールレベル可変シングルトンと
/// `_reset*ForTests()` は模倣しない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionArtifactsClientImpl {
    data_dir: PathBuf,
    scopes_dir: PathBuf,
    stage_graph_override: Option<PathBuf>,
    scope_grid_override: Option<PathBuf>,
}

impl DefinitionArtifactsClientImpl {
    /// `data_dir` は `stage-graph.json` / `scope-grid.json` の置き場
    /// (`<harnessRoot>/tools/data/`)、`scopes_dir` は identity ファイルの置き場
    /// (`<harnessRoot>/scopes/`)。
    #[must_use]
    pub const fn new(data_dir: PathBuf, scopes_dir: PathBuf) -> DefinitionArtifactsClientImpl {
        DefinitionArtifactsClientImpl {
            data_dir,
            scopes_dir,
            stage_graph_override: None,
            scope_grid_override: None,
        }
    }

    /// `AIDLC_STAGE_GRAPH` 相当のオーバライド。
    #[must_use]
    pub fn with_stage_graph_override(mut self, path: PathBuf) -> DefinitionArtifactsClientImpl {
        self.stage_graph_override = Some(path);
        self
    }

    /// `AIDLC_SCOPE_GRID` 相当のオーバライド。グリッドの欠損は fatal ではない。
    #[must_use]
    pub fn with_scope_grid_override(mut self, path: PathBuf) -> DefinitionArtifactsClientImpl {
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
    /// ファイルが読めないのは OS 由来の失敗 (`Io`)、読めたが定義 id を与えない (不正 JSON /
    /// `name` 欠落 / id として不正) のは内容の破損 (`Corrupt`) と読み分ける。
    fn load_harness_identity(&self) -> Result<WorkflowDefinitionId, DefinitionReadFailure> {
        let path = self.harness_path();
        let identity = |cause: String| {
            DefinitionReadFailure::Corrupt(DefinitionCorruption::HarnessIdentity {
                path: path.clone(),
                cause,
            })
        };
        let content =
            fs::read_to_string(&path).map_err(|e| DefinitionReadFailure::from_io(&path, &e))?;
        let dto: HarnessDto =
            serde_json::from_str(&content).map_err(|e| identity(e.to_string()))?;
        WorkflowDefinitionId::parse(&dto.name).map_err(|e| identity(e.to_string()))
    }

    /// グラフを読む。**唯一 fatal な入力** (12 §4 #1・#2)。
    ///
    /// ドメイン型の `StageGraph` と、**読んだままの生値**を返す。後者は `DefinitionRevision`
    /// のハッシュ入力で、ドメイン型へ写す過程で落ちる情報 (未知フィールド・キー順) まで
    /// 内容版に含めるために要る。
    fn load_graph(&self) -> Result<(StageGraph, serde_json::Value), DefinitionReadFailure> {
        let path = self.stage_graph_path();
        let content =
            fs::read_to_string(&path).map_err(|e| DefinitionReadFailure::from_io(&path, &e))?;
        let invalid_json = |e: &serde_json::Error| {
            DefinitionReadFailure::Corrupt(DefinitionCorruption::InvalidJson {
                path: path.clone(),
                cause: e.to_string(),
            })
        };
        let raw: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| invalid_json(&e))?;
        let dto: Vec<StageNodeDto> =
            serde_json::from_str(&content).map_err(|e| invalid_json(&e))?;
        let mut nodes = Vec::with_capacity(dto.len());
        for node in dto {
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
        let dto: BTreeMap<String, ScopeColumnDto> = serde_json::from_str(&content).ok()?;
        let mut columns: BTreeMap<String, BTreeMap<StageSlug, PlanAction>> = BTreeMap::new();
        for (scope, column) in dto {
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
    fn load_scopes(&self) -> Result<BTreeMap<String, ScopeMetadata>, DefinitionReadFailure> {
        let paths = match scope_file_paths(&self.scopes_dir) {
            Ok(paths) => paths,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(DefinitionReadFailure::from_io(&self.scopes_dir, &e)),
        };
        let mut scopes: BTreeMap<String, ScopeMetadata> = BTreeMap::new();
        let mut origins: BTreeMap<String, PathBuf> = BTreeMap::new();
        for path in paths {
            let content =
                fs::read_to_string(&path).map_err(|e| DefinitionReadFailure::from_io(&path, &e))?;
            let metadata = parse_scope_metadata(&path, &content)?;
            let name = metadata.name().to_string();
            if let Some(first) = origins.get(&name) {
                return Err(scope_file(scope_duplicate_name_message(
                    &name, first, &path,
                )));
            }
            origins.insert(name.clone(), path);
            scopes.insert(name, metadata);
        }
        Ok(scopes)
    }

    /// 3 入力を読んで材料を組み立てる本体。失敗はアダプタ私有の中間表現で返し、ポート契約への
    /// 写像は [`DefinitionArtifactsClient::fetch`] が 1 箇所で行う。
    fn read(&self) -> Result<DefinitionArtifacts, DefinitionReadFailure> {
        // 定義 id は配布物自身が名乗る (ADR-008) — 呼出側が id を指定して照合するのではない。
        let id = self.load_harness_identity()?;
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
        Ok(DefinitionArtifacts::new(id, revision, graph, grid, scopes))
    }
}

impl DefinitionArtifactsClient for DefinitionArtifactsClientImpl {
    fn load(&self) -> Result<DefinitionArtifacts, DefinitionArtifactsError> {
        self.read()
            .map_err(DefinitionReadFailure::into_artifacts_error)
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
) -> Result<DefinitionRevision, DefinitionReadFailure> {
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
    let value = to_value(&input).map_err(|e| {
        DefinitionReadFailure::Corrupt(DefinitionCorruption::Malformed {
            message: format!("definition revision input: {e}"),
        })
    })?;
    DefinitionRevision::parse(&hash_canonical(&value).rendered()).map_err(|e| {
        DefinitionReadFailure::Corrupt(DefinitionCorruption::Malformed {
            message: format!("definition revision: {e}"),
        })
    })
}

// ---------------------------------------------------------------------------
// ワイヤ → ドメインの写像
// ---------------------------------------------------------------------------

fn to_stage_node(dto: StageNodeDto, path: &Path) -> Result<StageNode, DefinitionReadFailure> {
    let slug = StageSlug::parse(&dto.slug)
        .map_err(|e| malformed(path, &format!("invalid slug {:?} ({e:?})", dto.slug)))?;
    let number = StageNumber::parse(&dto.number).map_err(|e| {
        malformed(
            path,
            &format!(
                "stage {:?} has invalid number {:?} ({e:?})",
                dto.slug, dto.number
            ),
        )
    })?;
    let phase = PhaseId::parse(&dto.phase).map_err(|e| {
        malformed(
            path,
            &format!("stage {:?} has unknown phase ({e:?})", dto.slug),
        )
    })?;
    let execution = ExecutionKind::parse(&dto.execution).map_err(|e| {
        malformed(
            path,
            &format!("stage {:?} has unknown execution ({e:?})", dto.slug),
        )
    })?;
    let mode = StageMode::parse(&dto.mode).map_err(|e| {
        malformed(
            path,
            &format!("stage {:?} has unknown mode ({e:?})", dto.slug),
        )
    })?;

    let mut consumes = Vec::with_capacity(dto.consumes.len());
    for decl in dto.consumes {
        let conditional_on = match decl.conditional_on {
            None => None,
            Some(raw) => Some(BrownfieldGreenfield::parse(&raw).map_err(|e| {
                malformed(
                    path,
                    &format!("stage {:?} has unknown conditional_on ({e:?})", dto.slug),
                )
            })?),
        };
        consumes.push(ConsumeDecl::new(
            decl.artifact,
            decl.required,
            conditional_on,
        ));
    }

    let mut requires_stage = Vec::with_capacity(dto.requires_stage.len());
    for dep in &dto.requires_stage {
        requires_stage.push(StageSlug::parse(dep).map_err(|e| {
            malformed(
                path,
                &format!("stage {:?} requires invalid slug {dep:?} ({e:?})", dto.slug),
            )
        })?);
    }

    let mut rules_in_context = Vec::with_capacity(dto.rules_in_context.len());
    for rule in dto.rules_in_context {
        let scope = RuleScope::parse(&rule.scope).map_err(|e| {
            malformed(
                path,
                &format!("stage {:?} has unknown rule scope ({e:?})", dto.slug),
            )
        })?;
        rules_in_context.push(RuleInContext::new(rule.path, scope));
    }

    let review_class = match dto.review_class {
        None => None,
        Some(ref raw) => Some(ReviewClass::parse(raw).map_err(|e| {
            malformed(
                path,
                &format!("stage {:?} has unknown review_class ({e:?})", dto.slug),
            )
        })?),
    };

    let sensors_applicable = dto
        .sensors_applicable
        .into_iter()
        .map(|s| SensorRef::new(s.id, s.path, s.matches))
        .collect();

    let mut builder = StageNodeBuilder::new(slug, number, dto.name, phase, execution, mode)
        .condition(dto.condition)
        .lead_agent(dto.lead_agent)
        .support_agents(dto.support_agents)
        .workspace_requires(dto.workspace_requires)
        .produces(dto.produces)
        .optional_produces(dto.optional_produces)
        .produces_kinds(dto.produces_kinds)
        .consumes(consumes)
        .requires_stage(requires_stage)
        .sensors(dto.sensors)
        .scopes(dto.scopes)
        .inputs(dto.inputs)
        .outputs(dto.outputs)
        .rules_in_context(rules_in_context)
        .sensors_applicable(sensors_applicable);
    if let Some(v) = dto.for_each {
        builder = builder.for_each(v);
    }
    if let Some(v) = dto.reviewer {
        builder = builder.reviewer(v);
    }
    if let Some(v) = dto.reviewer_max_iterations {
        builder = builder.reviewer_max_iterations(v);
    }
    if let Some(v) = review_class {
        builder = builder.review_class(v);
    }
    if let Some(v) = dto.summary_confirmation {
        builder = builder.summary_confirmation(v);
    }
    if let Some(v) = dto.plugin {
        builder = builder.plugin(v);
    }
    if let Some(v) = dto.enabled {
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
fn parse_scope_metadata(
    path: &Path,
    content: &str,
) -> Result<ScopeMetadata, DefinitionReadFailure> {
    let body = frontmatter_body(content)
        .ok_or_else(|| scope_file(scope_missing_frontmatter_message(path)))?;

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
                skeleton = Some(
                    SkeletonDefault::parse(value)
                        .map_err(|_| scope_file(scope_invalid_skeleton_message(path, value)))?,
                );
            }
            "review_cap" => {
                review_cap = Some(
                    ReviewCapValue::parse(value)
                        .map_err(|_| scope_file(scope_invalid_review_cap_message(path, value)))?,
                );
            }
            // 有効スコープ中 1 つまでという集合レベルの一意性はスライス 1 の範囲外。
            // TODO(spec: 12 §3.3): `freeform_default` の集合一意性検証は compile 側と併せて実装する。
            "freeform_default" => freeform_default = value == "true",
            _ => {}
        }
    }

    let name = name.ok_or_else(|| scope_file(scope_missing_name_message(path)))?;
    let mut metadata =
        ScopeMetadata::new(&name).map_err(|_| scope_file(scope_missing_name_message(path)))?;
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
    // panic! は想定外バリアントの即時失敗という検証用途で使っており、テスト失敗のシグナル
    // として妥当なため許容する。
    #![allow(clippy::panic)]

    use super::*;
    use std::path::Path;

    fn scope_path() -> &'static Path {
        Path::new("/scopes/aidlc-feature.md")
    }

    /// 破損の診断表示を取り出す (パーサは `Corrupt` 以外を返さない)。
    fn cause_of(failure: DefinitionReadFailure) -> String {
        let DefinitionReadFailure::Corrupt(cause) = failure else {
            panic!("Corrupt を期待した: {failure:?}");
        };
        cause.to_string()
    }

    /// 私有の破損原因を `Display` で 1 行に畳んだ診断表示。
    fn rendered(cause: &DefinitionCorruption) -> String {
        cause.to_string()
    }

    #[test]
    fn every_corruption_renders_its_material_in_one_line() {
        // 利用者向けの upstream 逐語文言はクエリ側が所有する (b26 段階 2)。ここに残るのは
        // `Error::source` の連鎖に載る開発者向け診断であり、材料だけを運ぶ。
        assert_eq!(
            rendered(&DefinitionCorruption::ScopeFile {
                message: "boom".to_string(),
            }),
            "boom"
        );
        assert_eq!(
            rendered(&DefinitionCorruption::Malformed {
                message: "bang".to_string(),
            }),
            "bang"
        );
        assert_eq!(
            rendered(&DefinitionCorruption::InvalidJson {
                path: PathBuf::from("/p"),
                cause: "c".to_string(),
            }),
            "stage graph at /p is not valid JSON: c"
        );
        assert_eq!(
            rendered(&DefinitionCorruption::HarnessIdentity {
                path: PathBuf::from("/d/harness.json"),
                cause: "missing name".to_string(),
            }),
            "harness identity at /d/harness.json does not provide a definition id: missing name"
        );
    }

    #[test]
    fn the_failure_maps_onto_the_port_contract() {
        // OS 由来は `Io` — 分類と対象パスを運び、原因連鎖は持たない。
        let error = DefinitionReadFailure::Io {
            kind: io::ErrorKind::NotFound,
            path: PathBuf::from("/d/stage-graph.json"),
        }
        .into_artifacts_error();
        let DefinitionArtifactsError::Io { kind, ref path } = error else {
            panic!("Io を期待した: {error:?}");
        };
        assert_eq!(kind, io::ErrorKind::NotFound);
        assert_eq!(path.as_path(), Path::new("/d/stage-graph.json"));
        assert!(Error::source(&error).is_none());

        // 破損は `Corrupt` — 分類は契約に載せず、原因連鎖だけが診断を運ぶ。
        let error = DefinitionReadFailure::Corrupt(DefinitionCorruption::Malformed {
            message: "bang".to_string(),
        })
        .into_artifacts_error();
        assert!(matches!(error, DefinitionArtifactsError::Corrupt { .. }));
        assert_eq!(
            Error::source(&error)
                .expect("Corrupt は原因を連鎖する")
                .to_string(),
            "bang"
        );
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
            cause_of(err),
            "Scope file missing frontmatter: /scopes/aidlc-feature.md"
        );
        let err = parse_scope_metadata(scope_path(), "---\ndepth: standard\n---\n").unwrap_err();
        assert_eq!(
            cause_of(err),
            "Scope file /scopes/aidlc-feature.md missing required frontmatter: name"
        );
        let err = parse_scope_metadata(scope_path(), "---\nname: \"\"\n---\n").unwrap_err();
        assert!(cause_of(err).ends_with("missing required frontmatter: name"));
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
            cause_of(err),
            "Scope file /scopes/aidlc-feature.md has invalid skeleton value \"yes\". Expected \"on\" or \"off\"."
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
        assert!(cause_of(err).contains("review_cap"));
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
        let definition_artifacts_client =
            DefinitionArtifactsClientImpl::new(PathBuf::from("/data"), PathBuf::from("/scopes"));
        assert_eq!(
            definition_artifacts_client.stage_graph_path(),
            PathBuf::from("/data/stage-graph.json")
        );
        assert_eq!(
            definition_artifacts_client.scope_grid_path(),
            PathBuf::from("/data/scope-grid.json")
        );
        assert_eq!(
            definition_artifacts_client.harness_path(),
            PathBuf::from("/data/harness.json")
        );

        let definition_artifacts_client = definition_artifacts_client
            .with_stage_graph_override(PathBuf::from("/pinned/graph.json"))
            .with_scope_grid_override(PathBuf::from("/pinned/grid.json"));
        assert_eq!(
            definition_artifacts_client.stage_graph_path(),
            PathBuf::from("/pinned/graph.json")
        );
        assert_eq!(
            definition_artifacts_client.scope_grid_path(),
            PathBuf::from("/pinned/grid.json")
        );
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
