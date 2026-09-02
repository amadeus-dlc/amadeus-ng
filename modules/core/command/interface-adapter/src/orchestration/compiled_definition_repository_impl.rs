//! `CompiledDefinitionRepository` の実装 — コンパイル済み定義 (配布束) を 3 入力
//! (`stage-graph.json` / `scope-grid.json` / `<harnessRoot>/scopes/aidlc-<name>.md`) から
//! 再構成する (12-workflow-definition §6)。
//!
//! # これは集約 [`CompiledDefinition`] の Repository である (オーナー裁定 2026-09-02、b36)
//!
//! 配布束は**同一システムのドメインモデル**であり、外部システムの成果物ではない
//! (「クライアントをリポジトリに、クライアントが扱うデータを集約に昇格」— #79 §1-4 / #80)。
//! 媒体が 3 ファイルであることは**この実装の内部詳細**で、ポート面には現れない
//! (`coding-rules/gateway-taxonomy.md` §2 — 媒体名を契約に漏らさない)。
//!
//! `WorkflowDefinition` (ジャーナルに住む定義) とは**別集約・同一系譜**である。b30 裁定
//! 「リポジトリの実装は EventStoreForSqlite を使う」は ES 集約 `WorkflowDefinition` の
//! Repository の話で、本 Repository の永続化面は配布束そのもの — compile コンテキストが
//! 実装されたら (slice 2)、compile が `store` の書き手としてここに現れる。読んだ内容から
//! 定義を確立・改訂して**イベントストアへ書く**のは `DefineWorkflowUseCase` の仕事で、
//! 以後のジャーナル側の読取は常にイベントからの再構成になる。
//!
//! **この実装が所有するもの** (12 §6):
//! - パス解決とテストシーム (`<data_dir>/{stage-graph,scope-grid}.json` / `<scopes_dir>`、
//!   および `AIDLC_STAGE_GRAPH` / `AIDLC_SCOPE_GRID` 相当のオーバライド)。
//!   **env の読取そのものは合成ルートの責務**で、ここは注入されたパスだけを見る
//!   (テストを hermetic に保つため)。
//! - JSON コーデック (serde ワイヤ構造体) と frontmatter パーサ (手書き — 00-policy R9)。
//! - (内容版 `DefinitionRevision` はここでは算出しない — 集約が内容から導出する。
//!   ADR-008 改訂 2026-09-02、b36)。
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
    BrownfieldGreenfield, Compiled, CompiledDefinition, CompiledDefinitionEvent,
    CompiledDefinitionEventId, CompiledDefinitionId, ConsumeDecl, ExecutionKind, PhaseId,
    PlanAction, ReviewCapValue, ReviewClass, RuleInContext, RuleScope, ScopeGrid, ScopeMetadata,
    SensorRef, SkeletonDefault, StageGraph, StageMode, StageNode, StageNodeBuilder, StageNumber,
    StageSlug,
};
use core_command_use_case::orchestration::{CompiledDefinitionRepository, RepositoryError};
use core_infrastructure::atomic::write_file_atomic;
use core_infrastructure::canon_json::{SerializationProfile, ToValueError, serialize, to_value};
use serde::ser::{SerializeMap, Serializer};
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
    fn into_repository_error(
        self,
        id: &CompiledDefinitionId,
    ) -> RepositoryError<CompiledDefinitionId> {
        match self {
            DefinitionReadFailure::Io { kind, path } => RepositoryError::Io {
                kind,
                path: Some(path),
            },
            DefinitionReadFailure::Corrupt(cause) => RepositoryError::Corrupt {
                id: id.clone(),
                seq_nr: None,
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

/// (イベント, 集約) の対が同じ内容を語っているか — `store` の書込契約。
///
/// 媒体はスナップショット (配布ファイル) なので書くのは集約のほうだが、イベントが集約の
/// 現在の状態を説明していない対は歴史と保存像の食い違いであり、拒む。誕生と再コンパイルは
/// 内容全量が集約と一致すること、scope 登記は登記した identity と列が集約に載っていること、
/// プラグイン選択は集約のグラフがその選択の不動点であること。
fn event_describes(event: &CompiledDefinitionEvent, aggregate: &CompiledDefinition) -> bool {
    match event {
        CompiledDefinitionEvent::Compiled(compiled) => {
            CompiledDefinition::from(compiled.clone()) == *aggregate
        }
        CompiledDefinitionEvent::Recompiled(recompiled) => {
            recompiled.aggregate_id() == aggregate.id()
                && recompiled.graph() == aggregate.graph()
                && recompiled.grid() == aggregate.grid()
                && recompiled.scopes() == aggregate.scopes()
        }
        CompiledDefinitionEvent::ScopeRegistered(registered) => {
            let name = registered.metadata().name();
            registered.aggregate_id() == aggregate.id()
                && aggregate.scopes().get(name) == Some(registered.metadata())
                && aggregate.grid().column(name) == Some(registered.column())
        }
        CompiledDefinitionEvent::PluginSelectionApplied(applied) => {
            applied.aggregate_id() == aggregate.id()
                && aggregate
                    .graph()
                    .with_plugin_selection(applied.enabled_plugins())
                    == *aggregate.graph()
        }
    }
}

/// `store` の書込契約違反・永続化表現への写像失敗を `Corrupt` に畳む (分類は契約に載せず、
/// 材料は原因連鎖で運ぶ — 読取側の `into_repository_error` と対)。
fn store_corrupt(
    id: &CompiledDefinitionId,
    message: String,
) -> RepositoryError<CompiledDefinitionId> {
    RepositoryError::Corrupt {
        id: id.clone(),
        seq_nr: None,
        source: Box::new(DefinitionCorruption::Malformed { message }),
    }
}

/// `store` の OS 由来の失敗を、対象パスつきの `Io` に畳む。
fn io_at(path: &Path, error: &io::Error) -> RepositoryError<CompiledDefinitionId> {
    RepositoryError::Io {
        kind: error.kind(),
        path: Some(path.to_path_buf()),
    }
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
    #[serde(default, with = "crate::orchestration::kinds_codec")]
    produces_kinds: Vec<(String, Vec<String>)>,
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

// ---------------------------------------------------------------------------
// Gateway
// ---------------------------------------------------------------------------

/// 配布 3 ファイルを媒体に持つ [`CompiledDefinitionRepository`] の実装。
///
/// 呼出のたびに 3 入力を読み直す。キャッシュ戦略 (`OnceCell` / 注入 / 呼出ごとのロード) は
/// **観測不能なので実装の自由** (12 §10) — upstream のモジュールレベル可変シングルトンと
/// `_reset*ForTests()` は模倣しない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledDefinitionRepositoryImpl {
    data_dir: PathBuf,
    scopes_dir: PathBuf,
    stage_graph_override: Option<PathBuf>,
    scope_grid_override: Option<PathBuf>,
}

impl CompiledDefinitionRepositoryImpl {
    /// `data_dir` は `stage-graph.json` / `scope-grid.json` の置き場
    /// (`<harnessRoot>/tools/data/`)、`scopes_dir` は identity ファイルの置き場
    /// (`<harnessRoot>/scopes/`)。
    #[must_use]
    pub const fn new(data_dir: PathBuf, scopes_dir: PathBuf) -> CompiledDefinitionRepositoryImpl {
        CompiledDefinitionRepositoryImpl {
            data_dir,
            scopes_dir,
            stage_graph_override: None,
            scope_grid_override: None,
        }
    }

    /// `AIDLC_STAGE_GRAPH` 相当のオーバライド。
    #[must_use]
    pub fn with_stage_graph_override(mut self, path: PathBuf) -> CompiledDefinitionRepositoryImpl {
        self.stage_graph_override = Some(path);
        self
    }

    /// `AIDLC_SCOPE_GRID` 相当のオーバライド。グリッドの欠損は fatal ではない。
    #[must_use]
    pub fn with_scope_grid_override(mut self, path: PathBuf) -> CompiledDefinitionRepositoryImpl {
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
    fn load_harness_identity(&self) -> Result<CompiledDefinitionId, DefinitionReadFailure> {
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
        CompiledDefinitionId::parse(&dto.name).map_err(|e| identity(e.to_string()))
    }

    /// グラフを読む。**唯一 fatal な入力** (12 §4 #1・#2)。
    fn load_graph(&self) -> Result<StageGraph, DefinitionReadFailure> {
        let path = self.stage_graph_path();
        let content =
            fs::read_to_string(&path).map_err(|e| DefinitionReadFailure::from_io(&path, &e))?;
        let invalid_json = |e: &serde_json::Error| {
            DefinitionReadFailure::Corrupt(DefinitionCorruption::InvalidJson {
                path: path.clone(),
                cause: e.to_string(),
            })
        };
        let dto: Vec<StageNodeDto> =
            serde_json::from_str(&content).map_err(|e| invalid_json(&e))?;
        let mut nodes = Vec::with_capacity(dto.len());
        for node in dto {
            nodes.push(to_stage_node(node, &path)?);
        }
        // 文書順のまま渡す (F2 — 読込時に数値順へ正規化しない)。
        StageGraph::new(nodes).map_err(|e| malformed(&path, &format!("{e:?}")))
    }

    /// グリッドを読む。**読めない / 不正なら `None`** を返し、呼出側が転置導出へ倒す
    /// (12 §4 #3 — *"callers never see a hard ENOENT for a derivable artifact"*)。
    fn load_grid(&self) -> Option<ScopeGrid> {
        let content = fs::read_to_string(self.scope_grid_path()).ok()?;
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
        Some(ScopeGrid::new(columns))
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

    /// `harness.json` の書込バイト。
    ///
    /// 集約の内容は識別子 (`name`) だけである。既存ファイルがあれば、集約の内容ではない
    /// 付随キー (`harnessDir` / `rulesSubdir` …) をキー順ごと保ったまま `name` だけを
    /// 差し替える (「書かない」= 壊さない)。無ければ `name` だけの identity を新設する。
    /// 既存ファイルが JSON オブジェクトとして読めないときは、読めない内容を黙って捨てず
    /// `Corrupt` で拒む。
    fn emit_harness_identity(
        &self,
        id: &CompiledDefinitionId,
    ) -> Result<String, RepositoryError<CompiledDefinitionId>> {
        let path = self.harness_path();
        let mut members = match fs::read_to_string(&path) {
            Ok(existing) => match serde_json::from_str::<serde_json::Value>(&existing) {
                Ok(serde_json::Value::Object(members)) => members,
                Ok(_) | Err(_) => {
                    return Err(store_corrupt(
                        id,
                        format!(
                            "harness identity at {} is not a JSON object; refusing to overwrite it",
                            path.display()
                        ),
                    ));
                }
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => serde_json::Map::new(),
            Err(e) => return Err(io_at(&path, &e)),
        };
        members.insert(
            "name".to_string(),
            serde_json::Value::String(id.as_str().to_string()),
        );
        contract_pretty(&serde_json::Value::Object(members))
            .map_err(|e| store_corrupt(id, format!("emit harness identity: {e}")))
    }

    /// 3 入力を読んで集約を再構成する本体。失敗はアダプタ私有の中間表現で返し、ポート契約への
    /// 写像は `find_by_id` が 1 箇所で行う。
    ///
    /// 復号した内容は誕生記録 [`Compiled`] に束ね、集約は `From<Compiled>` で導出する —
    /// ジャーナルを読む Repository が genesis イベントからスナップショット種を起こすのと
    /// 同じ経路である。再構成がイベントを**生成**しているのではなく、媒体に書かれている
    /// 誕生の内容を読み戻しているだけである。
    fn read(&self) -> Result<CompiledDefinition, DefinitionReadFailure> {
        // 定義 id は配布物自身が名乗る (ADR-008) — 呼出側が id を指定して照合するのではない。
        let id = self.load_harness_identity()?;
        let graph = self.load_graph()?;
        // グリッド欠損は fatal にしない (12 §4 #3) — 転置導出へ倒す。内容版は集約が内容から
        // 導出するので、導出グリッドと同じ内容の grid ファイルが置かれた場合と同じ値になる。
        let grid = self
            .load_grid()
            .unwrap_or_else(|| ScopeGrid::from_graph(&graph));
        let scopes = self.load_scopes()?;
        // 誕生記録の再構成にはイベント識別子が要るが、媒体 (配布ファイル) はイベントでは
        // ないので同一性を持たない。ここで採る値は `From<Compiled>` に捨てられる
        // (`coding-rules/aggregate-commands.md` — 再構成はイベントを歴史に足さない)。
        Ok(CompiledDefinition::from(Compiled::new(
            CompiledDefinitionEventId::generate(),
            id,
            graph,
            grid,
            scopes,
        )))
    }
}

impl CompiledDefinitionRepository for CompiledDefinitionRepositoryImpl {
    async fn find_by_id(
        &self,
        id: &CompiledDefinitionId,
    ) -> Result<CompiledDefinition, RepositoryError<CompiledDefinitionId>> {
        let compiled_definition = self
            .read()
            .map_err(|failure| failure.into_repository_error(id))?;
        // 配布束が別の系譜 ID を名乗っていれば、要求された ID の配布定義は「無い」。
        // 呼出側 (合成ルート) は同じ harness.json から ID を解決しているので、実際に
        // ここへ落ちるのは配布物が要求と食い違う異常系だけである。
        if compiled_definition.id() != id {
            return Err(RepositoryError::NotFound { id: id.clone() });
        }
        Ok(compiled_definition)
    }
    async fn store(
        &mut self,
        event: &CompiledDefinitionEvent,
        compiled_definition: &CompiledDefinition,
    ) -> Result<(), RepositoryError<CompiledDefinitionId>> {
        let id = compiled_definition.id();
        // イベントと集約の照合 — 対の取り違えは「歴史と保存像が別の内容を語る」書込契約違反
        // として拒む (`IntentRepositoryImpl` の写し — 対で受ける契約の意味を実装が守る)。
        if !event_describes(event, compiled_definition) {
            return Err(store_corrupt(
                id,
                "store pair mismatch: the event does not describe the aggregate".to_string(),
            ));
        }
        let graph_bytes = emit_graph(compiled_definition.graph())
            .map_err(|e| store_corrupt(id, format!("emit stage graph: {e}")))?;
        let grid_bytes = emit_grid(compiled_definition.graph(), compiled_definition.grid())
            .map_err(|e| store_corrupt(id, format!("emit scope grid: {e}")))?;
        let harness_bytes = self.emit_harness_identity(id)?;

        // 書込の原子性は**ファイル単位** (同一ディレクトリの一時ファイルへ書いて rename —
        // 途中で落ちても読み手が半端なファイルを見ることはない)。配布束は 2 ディレクトリ
        // (`tools/data` と `scopes`) に跨るので、**束全体の原子性** (途中で落ちたときに古い束と
        // 新しい束が混ざらないこと) はここでは与えない — 書き手 (compile コンテキスト、
        // slice 2) が「新しいハーネスディレクトリへ書いてから差し替える」等で担う。書き出す
        // バイトはディスクに触れる前にすべて用意してある (上の emit 3 本) ので、内容の
        // 直列化失敗では何も書かない。
        let write =
            |path: &Path, bytes: &str| -> Result<(), RepositoryError<CompiledDefinitionId>> {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).map_err(|e| io_at(parent, &e))?;
                }
                write_file_atomic(path, bytes.as_bytes()).map_err(|e| io_at(path, &e))
            };

        // 書込先は読取と同じ解決済みパス (override 込み) — 書いた面を `find_by_id` が読む。
        write(&self.harness_path(), &harness_bytes)?;
        write(&self.stage_graph_path(), &graph_bytes)?;
        write(&self.scope_grid_path(), &grid_bytes)?;

        // scope identity ファイル群: 集約が持つ集合と一致させる — 集合に無い既存の
        // `aidlc-*.md` は残すと次の find_by_id が余分なスコープとして読み戻すので消す。
        // 一覧が取れないのも失敗である (黙って続けると、消すべきものを消さないまま
        // 新しい識別ファイルだけが増える)。
        fs::create_dir_all(&self.scopes_dir).map_err(|e| io_at(&self.scopes_dir, &e))?;
        let wanted: std::collections::BTreeSet<String> = compiled_definition
            .scopes()
            .keys()
            .map(|name| format!("{SCOPE_FILE_PREFIX}{name}{SCOPE_FILE_SUFFIX}"))
            .collect();
        let existing =
            scope_file_paths(&self.scopes_dir).map_err(|e| io_at(&self.scopes_dir, &e))?;
        for path in existing {
            let keeps = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| wanted.contains(n));
            if !keeps {
                fs::remove_file(&path).map_err(|e| io_at(&path, &e))?;
            }
        }
        for metadata in compiled_definition.scopes().values() {
            let file = self.scopes_dir.join(format!(
                "{SCOPE_FILE_PREFIX}{}{SCOPE_FILE_SUFFIX}",
                metadata.name()
            ));
            // 散文本文は集約の内容ではない — 既存ファイルがあれば frontmatter だけを
            // 差し替え、本文はそのまま残す (「書かない」= 壊さない)。
            let mut bytes = emit_scope_markdown(metadata);
            match fs::read_to_string(&file) {
                Ok(existing) => bytes.push_str(scope_prose(&existing)),
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(io_at(&file, &e)),
            }
            write(&file, &bytes)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 書き手 (store) — contract-pretty のバイト契約 (12 §10 / golden-3c3146cf-graph-dist §1-§2)
// ---------------------------------------------------------------------------

/// `FIELD_ORDER` 28 キーを **struct 宣言順で符号化**した emit 用ワイヤ構造体
/// (ADR 0001 決定 3)。`undefined` 落としに相当するのは skip 属性だけで、
/// `null` / `[]` / `""` / `false` は落とさない — ただし dist 実測 (golden §2) で
/// 「キーごと不在」が常態のフィールド (`workspace_requires` は true の 1 件のみ・
/// `optional_produces` / `produces_kinds` は非空の数件のみ) は、その実測どおり
/// 既定値で省略する。
#[derive(Serialize)]
struct StageNodeEmitDto<'a> {
    slug: &'a str,
    number: &'a str,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    plugin: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    phase: &'a str,
    execution: &'a str,
    condition: &'a str,
    lead_agent: &'a str,
    support_agents: &'a [String],
    mode: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    for_each: Option<&'a str>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    workspace_requires: bool,
    produces: &'a [String],
    #[serde(skip_serializing_if = "<[String]>::is_empty")]
    optional_produces: &'a [String],
    #[serde(
        with = "crate::orchestration::kinds_codec",
        skip_serializing_if = "<[(String, Vec<String>)]>::is_empty"
    )]
    produces_kinds: &'a [(String, Vec<String>)],
    consumes: Vec<ConsumeEmitDto<'a>>,
    requires_stage: Vec<&'a str>,
    sensors: &'a [String],
    scopes: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    reviewer: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reviewer_max_iterations: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    review_class: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary_confirmation: Option<&'a str>,
    inputs: &'a str,
    outputs: &'a str,
    rules_in_context: Vec<RuleInContextEmitDto<'a>>,
    sensors_applicable: Vec<SensorRefEmitDto<'a>>,
}

/// consume 宣言の emit 形 (キー順は dist 実測: artifact, required, conditional_on)。
#[derive(Serialize)]
struct ConsumeEmitDto<'a> {
    artifact: &'a str,
    required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    conditional_on: Option<&'a str>,
}

/// rules_in_context 項の emit 形 (キー順: path, scope)。
#[derive(Serialize)]
struct RuleInContextEmitDto<'a> {
    path: &'a str,
    scope: &'a str,
}

/// sensors_applicable 項の emit 形 (キー順は dist 実測: id, path, matches)。
#[derive(Serialize)]
struct SensorRefEmitDto<'a> {
    id: &'a str,
    path: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    matches: Option<&'a str>,
}

/// `JSON.stringify(x, null, 2)` + 末尾改行 1 個の体裁 (contract-pretty)。
///
/// 契約 JSON の直列化は canon-json の 1 経路に固定する (BR1.7 / ADR 0001 決定 5 —
/// `serde_json` の直列化関数は `clippy.toml` が拒否する)。struct は宣言順、動的マップは
/// `Serialize` 実装が流した順で `JsonValue` になる (BR1.8) ので、emit 用 DTO の並びが
/// そのままバイトの並びになる。
fn contract_pretty<T: Serialize>(value: &T) -> Result<String, ToValueError> {
    Ok(serialize(
        &to_value(value)?,
        SerializationProfile::ContractPretty,
    ))
}

/// `stage-graph.json` のバイトを組む (ノード配列は文書順のまま)。
fn emit_graph(graph: &StageGraph) -> Result<String, ToValueError> {
    let nodes: Vec<StageNodeEmitDto<'_>> = graph
        .nodes()
        .iter()
        .map(|node| StageNodeEmitDto {
            slug: node.slug().as_str(),
            number: node.number().as_str(),
            name: node.name(),
            plugin: node.plugin(),
            enabled: node.enabled(),
            phase: node.phase().as_str(),
            execution: node.execution().as_str(),
            condition: node.condition(),
            lead_agent: node.lead_agent(),
            support_agents: node.support_agents(),
            mode: node.mode().as_str(),
            for_each: node.for_each(),
            workspace_requires: node.workspace_requires(),
            produces: node.produces(),
            optional_produces: node.optional_produces(),
            produces_kinds: node.produces_kinds(),
            consumes: node
                .consumes()
                .iter()
                .map(|consume| ConsumeEmitDto {
                    artifact: consume.artifact(),
                    required: consume.required(),
                    conditional_on: consume.conditional_on().map(BrownfieldGreenfield::as_str),
                })
                .collect(),
            requires_stage: node
                .requires_stage()
                .iter()
                .map(StageSlug::as_str)
                .collect(),
            sensors: node.sensors(),
            scopes: node.scopes(),
            reviewer: node.reviewer(),
            reviewer_max_iterations: node.reviewer_max_iterations(),
            review_class: node.review_class().map(ReviewClass::as_str),
            summary_confirmation: node.summary_confirmation(),
            inputs: node.inputs(),
            outputs: node.outputs(),
            rules_in_context: node
                .rules_in_context()
                .iter()
                .map(|rule| RuleInContextEmitDto {
                    path: rule.path(),
                    scope: rule.scope().as_str(),
                })
                .collect(),
            sensors_applicable: node
                .sensors_applicable()
                .iter()
                .map(|sensor| SensorRefEmitDto {
                    id: sensor.id(),
                    path: sensor.path(),
                    matches: sensor.matches(),
                })
                .collect(),
        })
        .collect();
    contract_pretty(&nodes)
}

/// `scope-grid.json` のバイトを組む。
///
/// スコープ列は名前の辞書順 (dist 実測 — golden §2)、各列の stage キーは**グラフ文書順**
/// (dist 実測: ソートではなく `numericStageOrder` = 文書順の部分列)。動的キーの順序は
/// [`ScopeGridEmitDto`] / [`ScopeStagesEmitDto`] の `Serialize` 実装が流す順で保たれる
/// (canon-json の `to_value` は挿入順を保つ — BR1.8)。
fn emit_grid(graph: &StageGraph, grid: &ScopeGrid) -> Result<String, ToValueError> {
    contract_pretty(&ScopeGridEmitDto { graph, grid })
}

/// `scope-grid.json` の emit 形 — スコープ名 → 列。キー順は `ScopeGrid` の列順 (辞書順)。
struct ScopeGridEmitDto<'a> {
    graph: &'a StageGraph,
    grid: &'a ScopeGrid,
}

impl Serialize for ScopeGridEmitDto<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let scope_names = self.grid.scope_names();
        let mut columns = serializer.serialize_map(Some(scope_names.len()))?;
        for scope in scope_names {
            columns.serialize_entry(
                scope,
                &ScopeColumnEmitDto {
                    stages: ScopeStagesEmitDto {
                        graph: self.graph,
                        grid: self.grid,
                        scope,
                    },
                },
            )?;
        }
        columns.end()
    }
}

/// 1 列の emit 形 — 中間キー `stages` (F6) の 2 段構造。
#[derive(Serialize)]
struct ScopeColumnEmitDto<'a> {
    stages: ScopeStagesEmitDto<'a>,
}

/// 列の中身 — グラフ文書順で、このグリッドがコンパイルした slug だけを EXECUTE / SKIP で流す。
struct ScopeStagesEmitDto<'a> {
    graph: &'a StageGraph,
    grid: &'a ScopeGrid,
    scope: &'a str,
}

impl Serialize for ScopeStagesEmitDto<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut stages = serializer.serialize_map(None)?;
        for node in self.graph.nodes() {
            if let Some(action) = self.grid.action(self.scope, node.slug()) {
                stages.serialize_entry(node.slug().as_str(), action.as_str())?;
            }
        }
        stages.end()
    }
}

/// scope identity ファイル (`aidlc-<name>.md`) のバイトを組む。
///
/// frontmatter は [`parse_scope_metadata`] が読む最小サブセットと対称 — 集約が持たない
/// 散文本文は書かない (本文は集約の内容ではなく、内容版の入力にも入らない)。
fn emit_scope_markdown(metadata: &ScopeMetadata) -> String {
    let mut out = String::from("---\n");
    out.push_str(&format!("name: {}\n", metadata.name()));
    if let Some(depth) = metadata.depth() {
        out.push_str(&format!("depth: {depth}\n"));
    }
    if !metadata.keywords().is_empty() {
        out.push_str(&format!("keywords: [{}]\n", metadata.keywords().join(", ")));
    }
    if let Some(skeleton) = metadata.skeleton() {
        out.push_str(&format!("skeleton: {}\n", skeleton.as_str()));
    }
    if let Some(review_cap) = metadata.review_cap() {
        out.push_str(&format!("review_cap: {}\n", review_cap.as_str()));
    }
    if metadata.freeform_default() {
        out.push_str("freeform_default: true\n");
    }
    out.push_str("---\n");
    out
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

/// frontmatter の直後から末尾まで (散文本文)。frontmatter が無い内容は全体を本文とみなす —
/// `store` が既存ファイルの本文を保つための切り出しで、読取の検証 (`frontmatter_body`) とは
/// 役目が違う。
fn scope_prose(content: &str) -> &str {
    let Some(rest) = content.strip_prefix(FRONTMATTER_FENCE).and_then(|rest| {
        rest.strip_prefix("\r\n")
            .or_else(|| rest.strip_prefix('\n'))
    }) else {
        return content;
    };
    let head_len = content.len() - rest.len();
    let mut offset = 0usize;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == FRONTMATTER_FENCE {
            return content
                .get(head_len + offset + line.len()..)
                .unwrap_or_default();
        }
        offset += line.len();
    }
    content
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
        let id = CompiledDefinitionId::parse("claude").expect("定義 id");
        // OS 由来は `Io` — 分類と対象パスを運び、原因連鎖は持たない。
        let error = DefinitionReadFailure::Io {
            kind: io::ErrorKind::NotFound,
            path: PathBuf::from("/d/stage-graph.json"),
        }
        .into_repository_error(&id);
        let RepositoryError::Io { kind, ref path } = error else {
            panic!("Io を期待した: {error:?}");
        };
        assert_eq!(kind, io::ErrorKind::NotFound);
        assert_eq!(path.as_deref(), Some(Path::new("/d/stage-graph.json")));
        assert!(Error::source(&error).is_none());

        // 破損は `Corrupt` — 分類は契約に載せず、原因連鎖だけが診断を運ぶ。
        let error = DefinitionReadFailure::Corrupt(DefinitionCorruption::Malformed {
            message: "bang".to_string(),
        })
        .into_repository_error(&id);
        assert!(matches!(error, RepositoryError::Corrupt { .. }));
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
        let compiled_definition_repository =
            CompiledDefinitionRepositoryImpl::new(PathBuf::from("/data"), PathBuf::from("/scopes"));
        assert_eq!(
            compiled_definition_repository.stage_graph_path(),
            PathBuf::from("/data/stage-graph.json")
        );
        assert_eq!(
            compiled_definition_repository.scope_grid_path(),
            PathBuf::from("/data/scope-grid.json")
        );
        assert_eq!(
            compiled_definition_repository.harness_path(),
            PathBuf::from("/data/harness.json")
        );

        let compiled_definition_repository = compiled_definition_repository
            .with_stage_graph_override(PathBuf::from("/pinned/graph.json"))
            .with_scope_grid_override(PathBuf::from("/pinned/grid.json"));
        assert_eq!(
            compiled_definition_repository.stage_graph_path(),
            PathBuf::from("/pinned/graph.json")
        );
        assert_eq!(
            compiled_definition_repository.scope_grid_path(),
            PathBuf::from("/pinned/grid.json")
        );
    }

    #[test]
    fn scope_prose_is_everything_after_the_closing_fence() {
        assert_eq!(scope_prose("---\nname: a\n---\n\n# body\n"), "\n# body\n");
        assert_eq!(scope_prose("---\r\nname: a\r\n---\r\nbody"), "body");
        assert_eq!(scope_prose("---\nname: a\n---"), "");
        // frontmatter が無い (壊れた) 内容は全体が本文 — 捨てずに frontmatter を前置する。
        assert_eq!(scope_prose("no frontmatter\n"), "no frontmatter\n");
        assert_eq!(scope_prose("---\nname: a\n"), "---\nname: a\n");
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
