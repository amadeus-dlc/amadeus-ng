//! Published Language 3 入力 (`stage-graph.json` / `scope-grid.json` /
//! `<harnessRoot>/scopes/aidlc-<name>.md`) の**純 parse** — 読み終えた生バイト
//! ([`DefinitionArtifacts`]) をクエリモデル [`DefinitionView`] へ写す
//! (12-workflow-definition §6)。
//!
//! これらのファイルは compile コンテキストのイベント投影 = **リードモデル**であり、読む・
//! パースする責務はクエリ側にある (オーナー裁定 2026-08-30 —
//! `coding-rules/cqrs-boundaries.md` 規則 7)。写す先は**自前のビュー型**であって、コマンド側
//! ドメインの集約ではない (同規則 6: クエリ側はドメインに絶対依存せず集約を再構成しない)。
//!
//! ファイルの読取・パス解決・オーバライドの解決・scopes ディレクトリの列挙は同クレートの
//! DAO 実装 ([`super::workflow_definition_dao_impl`]) が行い、本モジュールは
//! **fs 呼び出しゼロ**の変換だけを持つ。
//!
//! **このモジュールが所有するもの** (12 §6):
//! - JSON コーデック (serde ワイヤ構造体) と frontmatter パーサ (手書き — 00-policy R9)。
//! - scope identity の拒否文言 — 拒否理由がパスと値に依存するため、材料の分解形では組み直せず
//!   [`WorkflowDefinitionReadError::ScopeFile`] / `Malformed` が組み立て済みの文言を運ぶ。
//!
//! **I/O 失敗の逐語文言 (12 §4 #1 / #2 と identity 診断) はここには無い** — ポートが運ぶのは
//! 材料だけで、文言を組むのは出す側のユースケース (`core_query_use_case` の `next` ラダーの
//! `wording`) である (`coding-rules/error-handling.md`、オーナー裁定 2026-08-31 のポート化)。
//!
//! **serde の厳格度** (12 §10 の表):
//! 1. 未知フィールドは**許容**する (`deny_unknown_fields` を付けない — F1)。将来版や
//!    プラグインが `FIELD_ORDER` を増やしても読めなくなってはならない。
//! 2. 欠損 optional は `Option` ないし空 default (`#[serde(default)]`)。
//! 3. 未知の列挙値は**全列挙 (`phase` / `execution` / `review_class` / `mode`) を load 時に
//!    厳密 enum で落とす** (12 §10 表 #3 — 2026-08-22 裁定)。ビュー型に `Unknown` variant を
//!    持たせず、読めた値だけが型に存在する状態を保つ。upstream との観測差は手編集グラフの
//!    未知値に限られ、dist の正規データでは生じない — ピン留め `3c3146cf` の配布実バイト
//!    33 ノードが全数 load できることは `tests/golden_parity_test.rs` が固定した。
//!
//! **失敗態度** (12 §4): グラフは fatal、グリッドは転置導出フォールバック、identity と
//! グリッド列の不一致は双方向とも正当。

use core_infrastructure::canon_json::{hash_canonical, to_value};
use core_query_use_case::orchestration::WorkflowDefinitionReadError;
use core_query_use_case::workflow_view::{
    BrownfieldGreenfieldView, ConsumeDeclView, DefinitionIdView, DefinitionRevisionView,
    DefinitionView, ExecutionKindView, PhaseView, PlanActionView, ReviewCapValueView,
    ReviewClassView, RuleInContextView, RuleScopeView, ScopeGridView, ScopeMetadataView,
    SensorRefView, SkeletonDefaultView, StageGraphView, StageModeView, StageNumberView,
    StageSlugView, StageView, StageViewBuilder,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// frontmatter の区切り行。
const FRONTMATTER_FENCE: &str = "---";

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
/// 文言はこのモジュールが組み立てる (出す側が逐語文言を持つ)。
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

/// グラフノードをビュー型へ写せない。upstream はロード時に検証しないため対応する逐語文言は
/// 存在しない (12 §10 — 構造的パースはロード時検証の補強)。
///
/// TODO(golden: stage-0): 診断文言なので互換対象外だが、採取後に文言カタログへ載せるかを判断。
fn malformed(path: &Path, detail: &str) -> WorkflowDefinitionReadError {
    WorkflowDefinitionReadError::Malformed {
        message: format!("Stage graph at {}: {detail}", path.display()),
    }
}

// ---------------------------------------------------------------------------
// ワイヤ構造体 (serde — 本モジュールの内部部品。ビュー型は serde 非依存)
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

/// 内容版 (`DefinitionRevisionView`) のハッシュ入力 (ADR-008)。**3 入力そのもの**を宣言順に束ねた
/// アダプタ層のワイヤ構造体で、ビュー型には現れない。
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
/// 本モジュールが写さないキー (`description` など) はハッシュ入力にも入らない。revision は
/// 「この読取が見た 3 入力」の内容版であって、ファイルの生バイトの版ではない。
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
// 読み終えた素材 (loader が集める) と純 parse
// ---------------------------------------------------------------------------

/// 生テキスト 1 つと、その出所 (逐語文言の材料)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawArtifact {
    path: String,
    text: String,
}

impl RawArtifact {
    /// 解決済みパスと読み終えた全文から組む。
    #[must_use]
    pub const fn new(path: String, text: String) -> RawArtifact {
        RawArtifact { path, text }
    }
}

/// 読み終えた Published Language 3 入力 + scope identity ファイル群。
///
/// 集めるのは同クレートの reader である — パス解決 (`<data_dir>/{stage-graph,scope-grid}.json`
/// / `<scopes_dir>` と `AIDLC_STAGE_GRAPH` / `AIDLC_SCOPE_GRID` 相当のオーバライド) と
/// ファイル読取・列挙はそちらに閉じ、ここは値だけを見る。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionArtifacts {
    harness: RawArtifact,
    stage_graph: RawArtifact,
    scope_grid: Option<String>,
    scopes: Vec<RawArtifact>,
}

impl DefinitionArtifacts {
    /// loader が読み終えた素材から組む。
    ///
    /// `scope_grid` の `None` は「読めない / 無い」— fatal ではなく転置導出へ倒れる
    /// (12 §4 #3)。`scopes` はパス昇順 (重複 `name:` 検出を決定的にするため — 並べるのは
    /// reader の列挙)。`AIDLC_STAGE_GRAPH` オーバライドの hint 分岐は**読取失敗**
    /// (`NotReadable` — reader が組む) にだけ効くので、読めた素材はその区別を運ばない。
    #[must_use]
    pub const fn new(
        harness: RawArtifact,
        stage_graph: RawArtifact,
        scope_grid: Option<String>,
        scopes: Vec<RawArtifact>,
    ) -> DefinitionArtifacts {
        DefinitionArtifacts {
            harness,
            stage_graph,
            scope_grid,
            scopes,
        }
    }
}

/// 読み終えた素材をクエリモデル [`DefinitionView`] へ写す (fs 呼び出しゼロの純 parse)。
///
/// **失敗態度は 3 入力で意図的に非対称** (12 §4。この非対称そのものが観測可能な契約で、
/// 「より厳格にする」方向の改変も逸脱になる): ハーネス identity とグラフは fatal、
/// グリッドの不正は転置導出フォールバック、identity とグリッド列の不一致は双方向とも正当。
///
/// # Errors
///
/// ハーネス identity の検証失敗 (`HarnessIdentity`)、グラフの不正 JSON (`InvalidJson`)、
/// scope identity の検証失敗 (`ScopeFile`)、ビュー型への写像失敗 (`Malformed`)。
/// **グリッドの欠損・不正はエラーにしない** — 転置導出へフォールバックする (12 §4 #3)。
pub fn parse_workflow_definition(
    artifacts: &DefinitionArtifacts,
) -> Result<DefinitionView, WorkflowDefinitionReadError> {
    let harness_id = parse_harness_identity(&artifacts.harness)?;
    let (graph, raw_graph) = parse_graph(&artifacts.stage_graph)?;
    let (grid, raw_grid) = match artifacts.scope_grid.as_deref().and_then(parse_grid) {
        Some(read) => read,
        // グリッド欠損・不正は fatal にしない (12 §4 #3)。revision も導出グリッドから作る —
        // 「読めた 3 入力の内容版」であって「ディスクにあったバイトの版」ではない。
        None => {
            let derived = ScopeGridView::from_graph(&graph);
            let raw = serialize_grid(&derived);
            (derived, raw)
        }
    };
    let scopes = parse_scopes(&artifacts.scopes)?;
    let revision = compute_revision(&raw_graph, &raw_grid, &scopes)?;

    Ok(DefinitionView::new(
        harness_id, revision, graph, grid, scopes,
    ))
}

/// `harness.json` の `name` を定義 id として読む (ADR-008)。**fatal な入力**。
///
/// 不正 JSON・`name` 欠落・id として不正、のいずれも `HarnessIdentity` に畳む (ファイルが
/// 読めない場合は reader が同じ変種を組む)。upstream には定義 id の概念が無いので、この
/// 分岐に対応する逐語文言も無い。
fn parse_harness_identity(
    harness: &RawArtifact,
) -> Result<DefinitionIdView, WorkflowDefinitionReadError> {
    let identity = |cause: String| WorkflowDefinitionReadError::HarnessIdentity {
        path: harness.path.clone(),
        cause,
    };
    let wire: WireHarness =
        serde_json::from_str(&harness.text).map_err(|e| identity(e.to_string()))?;
    DefinitionIdView::parse(&wire.name).map_err(|e| identity(e.to_string()))
}

/// グラフを写す。**唯一 fatal な入力** (12 §4 #1・#2 — #1 の読取失敗は reader が組む)。
///
/// ビュー型の `StageGraphView` と、**読んだままの生値**を返す。後者は内容版のハッシュ入力で、
/// ビュー型へ写す過程で落ちる情報 (未知フィールド・キー順) まで内容版に含めるために要る。
fn parse_graph(
    stage_graph: &RawArtifact,
) -> Result<(StageGraphView, serde_json::Value), WorkflowDefinitionReadError> {
    let path = Path::new(&stage_graph.path);
    let invalid_json = |e: &serde_json::Error| WorkflowDefinitionReadError::InvalidJson {
        path: stage_graph.path.clone(),
        cause: e.to_string(),
    };
    let raw: serde_json::Value =
        serde_json::from_str(&stage_graph.text).map_err(|e| invalid_json(&e))?;
    let wire: Vec<WireStageNode> =
        serde_json::from_str(&stage_graph.text).map_err(|e| invalid_json(&e))?;
    let mut nodes = Vec::with_capacity(wire.len());
    for node in wire {
        nodes.push(to_stage_view(node, path)?);
    }
    // 文書順のまま渡す (F2 — 読込時に数値順へ正規化しない)。
    let graph = StageGraphView::new(nodes).map_err(|e| malformed(path, &format!("{e:?}")))?;
    Ok((graph, raw))
}

/// グリッドを写す。**不正なら `None`** を返し、呼出側が転置導出へ倒す
/// (12 §4 #3 — *"callers never see a hard ENOENT for a derivable artifact"*)。
///
/// ビュー型の `ScopeGridView` と読んだままの生値を返す (生値は revision のハッシュ入力)。
fn parse_grid(content: &str) -> Option<(ScopeGridView, serde_json::Value)> {
    let raw: serde_json::Value = serde_json::from_str(content).ok()?;
    let wire: BTreeMap<String, WireScopeColumn> = serde_json::from_str(content).ok()?;
    let mut columns: BTreeMap<String, BTreeMap<StageSlugView, PlanActionView>> = BTreeMap::new();
    for (scope, column) in wire {
        let mut cells: BTreeMap<StageSlugView, PlanActionView> = BTreeMap::new();
        for (slug, action) in column.stages {
            // 文法外 slug・`EXECUTE`/`SKIP` 以外の値はセルごと落とす。結果は 3 値契約の
            // `None` (=「このグリッドがコンパイルしていないステージ」) になり、
            // upstream の「列に slug が無い」と同じ観測になる (F8)。
            // 全体を転置導出へ倒さないのは、1 セルの異常でグリッド全体を捨てないため。
            if let (Ok(slug), Some(action)) =
                (StageSlugView::parse(&slug), PlanActionView::parse(&action))
            {
                cells.insert(slug, action);
            }
        }
        columns.insert(scope, cells);
    }
    Some((ScopeGridView::new(columns), raw))
}

/// identity ファイル群の frontmatter を読む。**有効スコープの権威**はここ (F7)。
///
/// ディレクトリの列挙 (パス昇順・欠損時は空) は reader の責務で、ここは読み終えた列を
/// 検証する。
fn parse_scopes(
    files: &[RawArtifact],
) -> Result<BTreeMap<String, ScopeMetadataView>, WorkflowDefinitionReadError> {
    let mut scopes: BTreeMap<String, ScopeMetadataView> = BTreeMap::new();
    let mut origins: BTreeMap<String, String> = BTreeMap::new();
    for file in files {
        let path = Path::new(&file.path);
        let metadata = parse_scope_metadata(path, &file.text)?;
        let name = metadata.name().to_string();
        if let Some(first) = origins.get(&name) {
            return Err(WorkflowDefinitionReadError::ScopeFile {
                message: scope_duplicate_name_message(&name, Path::new(first), path),
            });
        }
        origins.insert(name.clone(), file.path.clone());
        scopes.insert(name, metadata);
    }
    Ok(scopes)
}

/// 転置導出グリッドを `scope-grid.json` と同じ 2 段構造へ直列化する
/// (`{ <scope>: { stages: { <slug>: "EXECUTE"|"SKIP" } } }` — F6 の中間キー込み)。
///
/// グリッドが読めなかったときの revision 入力。ファイルから読めたときの値と同じ形にして
/// おくことで、「導出グリッドと同じ内容の grid ファイルが置かれた」場合に同じ revision に
/// なる — 内容版が入力の**内容**だけで決まるという性質が保たれる。
fn serialize_grid(grid: &ScopeGridView) -> serde_json::Value {
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

/// 3 入力の正準 JSON ダイジェストを `DefinitionRevisionView` にする (ADR-008)。
///
/// `hash_canonical` は再帰キーソート + `sha256:` 接頭辞 (正準族) なので、入力の**内容**だけで
/// 決まりキーの並び順には依存しない。scope は `BTreeMap` から取るため常に `name` 昇順。
fn compute_revision(
    raw_graph: &serde_json::Value,
    raw_grid: &serde_json::Value,
    scopes: &BTreeMap<String, ScopeMetadataView>,
) -> Result<DefinitionRevisionView, WorkflowDefinitionReadError> {
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
    let value = to_value(&input).map_err(|e| WorkflowDefinitionReadError::Malformed {
        message: format!("definition revision input: {e}"),
    })?;
    DefinitionRevisionView::parse(&hash_canonical(&value).rendered()).map_err(|e| {
        WorkflowDefinitionReadError::Malformed {
            message: format!("definition revision: {e}"),
        }
    })
}

// ---------------------------------------------------------------------------
// ワイヤ → ビュー型の写像
// ---------------------------------------------------------------------------

fn to_stage_view(
    wire: WireStageNode,
    path: &Path,
) -> Result<StageView, WorkflowDefinitionReadError> {
    let slug = StageSlugView::parse(&wire.slug)
        .map_err(|e| malformed(path, &format!("invalid slug {:?} ({e:?})", wire.slug)))?;
    let number = StageNumberView::parse(&wire.number).map_err(|e| {
        malformed(
            path,
            &format!(
                "stage {:?} has invalid number {:?} ({e:?})",
                wire.slug, wire.number
            ),
        )
    })?;
    let phase = PhaseView::parse(&wire.phase).map_err(|e| {
        malformed(
            path,
            &format!("stage {:?} has unknown phase ({e:?})", wire.slug),
        )
    })?;
    let execution = ExecutionKindView::parse(&wire.execution).map_err(|e| {
        malformed(
            path,
            &format!("stage {:?} has unknown execution ({e:?})", wire.slug),
        )
    })?;
    let mode = StageModeView::parse(&wire.mode).map_err(|e| {
        malformed(
            path,
            &format!("stage {:?} has unknown mode ({e:?})", wire.slug),
        )
    })?;

    let mut consumes = Vec::with_capacity(wire.consumes.len());
    for decl in wire.consumes {
        let conditional_on = match decl.conditional_on {
            None => None,
            Some(raw) => Some(BrownfieldGreenfieldView::parse(&raw).map_err(|e| {
                malformed(
                    path,
                    &format!("stage {:?} has unknown conditional_on ({e:?})", wire.slug),
                )
            })?),
        };
        consumes.push(ConsumeDeclView::new(
            decl.artifact,
            decl.required,
            conditional_on,
        ));
    }

    let mut requires_stage = Vec::with_capacity(wire.requires_stage.len());
    for dep in &wire.requires_stage {
        requires_stage.push(StageSlugView::parse(dep).map_err(|e| {
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
        let scope = RuleScopeView::parse(&rule.scope).map_err(|e| {
            malformed(
                path,
                &format!("stage {:?} has unknown rule scope ({e:?})", wire.slug),
            )
        })?;
        rules_in_context.push(RuleInContextView::new(rule.path, scope));
    }

    let review_class = match wire.review_class {
        None => None,
        Some(ref raw) => Some(ReviewClassView::parse(raw).map_err(|e| {
            malformed(
                path,
                &format!("stage {:?} has unknown review_class ({e:?})", wire.slug),
            )
        })?),
    };

    let sensors_applicable = wire
        .sensors_applicable
        .into_iter()
        .map(|s| SensorRefView::new(s.id, s.path, s.matches))
        .collect();

    let mut builder = StageViewBuilder::new(slug, number, wire.name, phase, execution, mode)
        .with_condition(wire.condition)
        .with_lead_agent(wire.lead_agent)
        .with_support_agents(wire.support_agents)
        .with_workspace_requires(wire.workspace_requires)
        .with_produces(wire.produces)
        .with_optional_produces(wire.optional_produces)
        .with_produces_kinds(wire.produces_kinds)
        .with_consumes(consumes)
        .with_requires_stage(requires_stage)
        .with_sensors(wire.sensors)
        .with_scopes(wire.scopes)
        .with_inputs(wire.inputs)
        .with_outputs(wire.outputs)
        .with_rules_in_context(rules_in_context)
        .with_sensors_applicable(sensors_applicable);
    if let Some(v) = wire.for_each {
        builder = builder.with_for_each(v);
    }
    if let Some(v) = wire.reviewer {
        builder = builder.with_reviewer(v);
    }
    if let Some(v) = wire.reviewer_max_iterations {
        builder = builder.with_reviewer_max_iterations(v);
    }
    if let Some(v) = review_class {
        builder = builder.with_review_class(v);
    }
    if let Some(v) = wire.summary_confirmation {
        builder = builder.with_summary_confirmation(v);
    }
    if let Some(v) = wire.plugin {
        builder = builder.with_plugin(v);
    }
    if let Some(v) = wire.enabled {
        builder = builder.with_enabled(v);
    }
    Ok(builder.build())
}

// ---------------------------------------------------------------------------
// scope identity ファイル (手書き frontmatter パーサ — 00-policy R9)
// ---------------------------------------------------------------------------

/// frontmatter の最小 YAML サブセットを手書きで読む。
///
/// 受理する形は `---` で挟まれた `key: value` 行と `keywords: [a, b]` のフロー列だけで、
/// 未知キー (`description` / `testStrategy` / `runner` / `plugin` 等) は黙って無視する。
/// 汎用 YAML パーサへ置換すると寛容パースと逐語拒否文言の契約が静かに変わる (12 §3.3)。
fn parse_scope_metadata(
    path: &Path,
    content: &str,
) -> Result<ScopeMetadataView, WorkflowDefinitionReadError> {
    let body = frontmatter_body(content).ok_or_else(|| WorkflowDefinitionReadError::ScopeFile {
        message: scope_missing_frontmatter_message(path),
    })?;

    let mut name: Option<String> = None;
    let mut depth: Option<String> = None;
    let mut keywords: Vec<String> = Vec::new();
    let mut skeleton: Option<SkeletonDefaultView> = None;
    let mut review_cap: Option<ReviewCapValueView> = None;
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
                skeleton = Some(SkeletonDefaultView::parse(value).map_err(|_| {
                    WorkflowDefinitionReadError::ScopeFile {
                        message: scope_invalid_skeleton_message(path, value),
                    }
                })?);
            }
            "review_cap" => {
                review_cap = Some(ReviewCapValueView::parse(value).map_err(|_| {
                    WorkflowDefinitionReadError::ScopeFile {
                        message: scope_invalid_review_cap_message(path, value),
                    }
                })?);
            }
            // 有効スコープ中 1 つまでという集合レベルの一意性はスライス 1 の範囲外。
            // TODO(spec: 12 §3.3): `freeform_default` の集合一意性検証は compile 側と併せて実装する。
            "freeform_default" => freeform_default = value == "true",
            _ => {}
        }
    }

    let name = name.ok_or_else(|| WorkflowDefinitionReadError::ScopeFile {
        message: scope_missing_name_message(path),
    })?;
    let mut metadata =
        ScopeMetadataView::new(&name).map_err(|_| WorkflowDefinitionReadError::ScopeFile {
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
            WorkflowDefinitionReadError::ScopeFile {
                message: "Scope file missing frontmatter: /scopes/aidlc-feature.md".to_string()
            }
        );
        let err = parse_scope_metadata(scope_path(), "---\ndepth: standard\n---\n").unwrap_err();
        assert_eq!(
            err,
            WorkflowDefinitionReadError::ScopeFile {
                message: "Scope file /scopes/aidlc-feature.md missing required frontmatter: name"
                    .to_string()
            }
        );
        let err = parse_scope_metadata(scope_path(), "---\nname: \"\"\n---\n").unwrap_err();
        assert!(matches!(err, WorkflowDefinitionReadError::ScopeFile { .. }));
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
            WorkflowDefinitionReadError::ScopeFile {
                message: "Scope file /scopes/aidlc-feature.md has invalid skeleton value \"yes\". Expected \"on\" or \"off\"."
                    .to_string()
            }
        );
    }

    #[test]
    fn review_cap_accepts_the_three_declared_values_and_rejects_the_rest() {
        for (raw, expected) in [
            ("adversarial", ReviewCapValueView::Adversarial),
            ("advisory", ReviewCapValueView::Advisory),
            ("none", ReviewCapValueView::None),
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
            matches!(err, WorkflowDefinitionReadError::ScopeFile { message } if message.contains("review_cap"))
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
