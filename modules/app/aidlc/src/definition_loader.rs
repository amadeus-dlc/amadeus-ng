//! ワークフロー定義の loader — パス解決とファイル読取 (I/O はここに閉じる)。
//!
//! 3 入力 (`harness.json` / `stage-graph.json` / `scope-grid.json`) と scope identity
//! ファイル群 (`<scopes_dir>/aidlc-*.md`) を読み、生バイトをアダプタの純 parse
//! ([`parse_workflow_definition`]) へ渡す。**失敗態度は 12 §4 の表のとおり**で、I/O 由来の
//! 変種 (`NotReadable` / `HarnessIdentity` の読取失敗・`ScopeFile` の列挙失敗) はここが組み、
//! parse 由来の変種はアダプタが組む — 型は [`GraphReadError`] 1 本である。
//!
//! `AIDLC_STAGE_GRAPH` / `AIDLC_SCOPE_GRID` 相当のオーバライドは**パスとして注入**する —
//! env の読取そのものはバイナリの main が行う (テストを hermetic に保つため)。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use core_command_domain::workflow_definition::WorkflowDefinition;
use core_command_interface_adapter::orchestration::{
    DefinitionArtifacts, GraphReadError, RawArtifact, parse_workflow_definition,
};

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

/// 3 入力の置き場 — パス解決とテストシーム。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionPaths {
    data_dir: PathBuf,
    scopes_dir: PathBuf,
    stage_graph_override: Option<PathBuf>,
    scope_grid_override: Option<PathBuf>,
}

impl DefinitionPaths {
    /// `data_dir` は `stage-graph.json` / `scope-grid.json` / `harness.json` の置き場
    /// (`<harnessRoot>/tools/data/`)、`scopes_dir` は identity ファイルの置き場
    /// (`<harnessRoot>/scopes/`)。
    #[must_use]
    pub const fn new(data_dir: PathBuf, scopes_dir: PathBuf) -> DefinitionPaths {
        DefinitionPaths {
            data_dir,
            scopes_dir,
            stage_graph_override: None,
            scope_grid_override: None,
        }
    }

    /// `AIDLC_STAGE_GRAPH` 相当のオーバライド。設定すると読取失敗時の逐語文言の hint 節が
    /// 「unset して既定に戻せ」形へ切り替わる (12 §4 #1)。
    #[must_use]
    pub fn with_stage_graph_override(mut self, path: PathBuf) -> DefinitionPaths {
        self.stage_graph_override = Some(path);
        self
    }

    /// `AIDLC_SCOPE_GRID` 相当のオーバライド。グリッドの欠損は fatal ではないため、
    /// こちらに hint 節の分岐は無い。
    #[must_use]
    pub fn with_scope_grid_override(mut self, path: PathBuf) -> DefinitionPaths {
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
}

/// 3 入力 + scope identity 群を読み、集約 `WorkflowDefinition` を組み立てて返す。
///
/// 呼出のたびに読み直す。キャッシュ戦略は**観測不能なので実装の自由** (12 §10) — upstream の
/// モジュールレベル可変シングルトンと `_reset*ForTests()` は模倣しない。いずれの失敗でも
/// **stdout に何も書かない** (12 §4 #10 — half-emitted directive を出さない)。
///
/// # Errors
///
/// ハーネス identity の読取・検証失敗 (`HarnessIdentity`)、グラフの読取失敗 (`NotReadable`)、
/// 不正 JSON (`InvalidJson`)、scope identity の列挙・読取・検証失敗 (`ScopeFile`)、ドメイン
/// 型への写像失敗 (`Malformed`)。**グリッドの欠損・不正はエラーにしない** — 転置導出へ
/// フォールバックする (12 §4 #3)。
pub fn load_workflow_definition(
    paths: &DefinitionPaths,
) -> Result<WorkflowDefinition, GraphReadError> {
    let harness_path = paths.harness_path();
    let harness_text =
        fs::read_to_string(&harness_path).map_err(|e| GraphReadError::HarnessIdentity {
            path: harness_path.display().to_string(),
            cause: e.to_string(),
        })?;

    let graph_path = paths.stage_graph_path();
    let graph_text = fs::read_to_string(&graph_path).map_err(|e| GraphReadError::NotReadable {
        path: graph_path.display().to_string(),
        cause: e.to_string(),
        env_override: paths.stage_graph_override.is_some(),
    })?;

    // グリッドは読めなければ `None` — 転置導出は parse 側が行う (12 §4 #3)。
    let grid_text = fs::read_to_string(paths.scope_grid_path()).ok();

    let mut scopes = Vec::new();
    for path in scope_file_paths(&paths.scopes_dir)? {
        let text = fs::read_to_string(&path).map_err(|e| GraphReadError::ScopeFile {
            message: format!("{}: {e}", path.display()),
        })?;
        scopes.push(RawArtifact::new(path.display().to_string(), text));
    }

    parse_workflow_definition(&DefinitionArtifacts::new(
        RawArtifact::new(harness_path.display().to_string(), harness_text),
        RawArtifact::new(graph_path.display().to_string(), graph_text),
        grid_text,
        scopes,
    ))
}

/// `<scopes_dir>/aidlc-*.md` をパス昇順で列挙する (重複 `name:` 検出を決定的にするため)。
///
/// ディレクトリ自体が無い場合は空カタログとして扱う (グラフと違い fatal にしない)。
/// TODO(spec: 12 §11): scopes ディレクトリ欠損時の態度は upstream 側の裏取りが未了。
fn scope_file_paths(scopes_dir: &Path) -> Result<Vec<PathBuf>, GraphReadError> {
    let entries = match fs::read_dir(scopes_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(GraphReadError::ScopeFile {
                message: format!("{}: {e}", scopes_dir.display()),
            });
        }
    };
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| GraphReadError::ScopeFile {
            message: format!("{}: {e}", scopes_dir.display()),
        })?;
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
