//! D3 の観測 — 配布シェルの配置と配布資産 (グラフ・スコープ表・スコープ定義・ステージ・
//! エージェント)。

use std::path::Path;

use core_query_use_case::orchestration::{
    ConsumeView, DefinitionAssetsView, GraphStageView, ObservationFailure, ScopeGridEntryView,
    StageArtifactsView, StageFileView, WorkspaceShellView,
};

use super::super::doctor_paths::DoctorPaths;

/// 本家 `PHASES` の順 (ステージ本体のディレクトリ名)。
const PHASES: [&str; 5] = [
    "initialization",
    "ideation",
    "inception",
    "construction",
    "operation",
];

/// D3.a の観測。
pub(super) fn shell(paths: &DoctorPaths) -> WorkspaceShellView {
    WorkspaceShellView::new(
        paths.harness_dir().exists(),
        paths.default_memory_dir().exists(),
    )
}

/// D3.b〜D3.d の観測。
pub(super) fn assets(paths: &DoctorPaths) -> DefinitionAssetsView {
    let data = paths.harness_dir().join("tools").join("data");
    DefinitionAssetsView::new(
        graph(&data.join("stage-graph.json")),
        scope_grid(&data.join("scope-grid.json")),
        scope_names(&paths.harness_dir().join("scopes")),
        stage_files(&paths.harness_dir().join("aidlc-common").join("stages")),
        agents(&paths.harness_dir().join("agents")),
    )
}

fn read_json(path: &Path, what: &str) -> Result<serde_json::Value, ObservationFailure> {
    let raw = std::fs::read_to_string(path).map_err(|error| {
        ObservationFailure::new(format!(
            "{what} not readable at {}: {error}. Reinstall the framework or re-run setup to restore the data file.",
            path.display()
        ))
    })?;
    serde_json::from_str(&raw).map_err(|error| {
        ObservationFailure::new(format!(
            "{what} at {} is not valid JSON: {error}",
            path.display()
        ))
    })
}

fn strings(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect()
}

fn text(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// `stage-graph.json` の全ステージ (無効なものも含む)。
fn graph(path: &Path) -> Result<Vec<GraphStageView>, ObservationFailure> {
    let parsed = read_json(path, "Stage graph")?;
    let stages = parsed.as_array().ok_or_else(|| {
        ObservationFailure::new(format!(
            "Stage graph at {} is not a JSON array",
            path.display()
        ))
    })?;
    Ok(stages
        .iter()
        .map(|stage| {
            let consumes = stage
                .get("consumes")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .map(|consume| {
                    ConsumeView::new(
                        text(consume, "artifact"),
                        consume
                            .get("required")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false),
                        consume
                            .get("conditional_on")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_string),
                    )
                })
                .collect();
            GraphStageView::new(
                text(stage, "slug"),
                text(stage, "phase"),
                text(stage, "number"),
                stage.get("enabled").and_then(serde_json::Value::as_bool) != Some(false),
                strings(stage.get("requires_stage")),
                StageArtifactsView::new(
                    strings(stage.get("produces")),
                    strings(stage.get("optional_produces")),
                    consumes,
                ),
            )
        })
        .collect())
}

/// `scope-grid.json` の全スコープ。
fn scope_grid(path: &Path) -> Result<Vec<ScopeGridEntryView>, ObservationFailure> {
    let parsed = read_json(path, "Scope grid")?;
    let scopes = parsed.as_object().ok_or_else(|| {
        ObservationFailure::new(format!(
            "Scope grid at {} is not a JSON object",
            path.display()
        ))
    })?;
    Ok(scopes
        .iter()
        .map(|(scope, entry)| {
            let stages = entry
                .get("stages")
                .and_then(serde_json::Value::as_object)
                .into_iter()
                .flatten()
                .map(|(slug, action)| {
                    (
                        slug.clone(),
                        action.as_str().unwrap_or_default().to_string(),
                    )
                })
                .collect();
            ScopeGridEntryView::new(scope.clone(), stages)
        })
        .collect())
}

/// `.md` ファイルを名前順に列挙する (ディレクトリが読めなければ原因)。
fn markdown_files(dir: &Path) -> Result<Vec<std::path::PathBuf>, ObservationFailure> {
    let entries = std::fs::read_dir(dir)
        .map_err(|error| ObservationFailure::new(format!("{}: {error}", dir.display())))?;
    let mut files: Vec<_> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect();
    files.sort();
    Ok(files)
}

/// 先頭の `---` から次の `---` まで (本家 `frontmatterBlock`)。
fn frontmatter_block(body: &str) -> Option<&str> {
    let rest = body
        .strip_prefix("---\n")
        .or_else(|| body.strip_prefix("---\r\n"))?;
    let end = rest.find("\n---").filter(|index| {
        // `\r\n---` も閉じ fence として認める。
        rest.get(..*index).is_some()
    })?;
    let block = rest.get(..end)?;
    Some(block.strip_suffix('\r').unwrap_or(block))
}

/// `key: value` のスカラ (引用符は剥がす — 本家 `scalarField`)。
fn scalar_field(frontmatter: &str, key: &str) -> String {
    let prefix = format!("{key}:");
    frontmatter
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .map(str::trim)
        .filter(|raw| !matches!(*raw, ">" | "|" | ">-" | "|-"))
        .map(|raw| {
            if (raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2)
                || (raw.starts_with('\'') && raw.ends_with('\'') && raw.len() >= 2)
            {
                raw.get(1..raw.len() - 1).unwrap_or_default().to_string()
            } else {
                raw.to_string()
            }
        })
        .unwrap_or_default()
}

/// `.claude/scopes/*.md` の frontmatter `name` (名前順)。
fn scope_names(dir: &Path) -> Result<Vec<String>, ObservationFailure> {
    let mut names = Vec::new();
    for file in markdown_files(dir)? {
        let body = std::fs::read_to_string(&file)
            .map_err(|error| ObservationFailure::new(format!("{}: {error}", file.display())))?;
        let frontmatter = frontmatter_block(&body).ok_or_else(|| {
            ObservationFailure::new(format!(
                "Scope file missing frontmatter: {}",
                file.display()
            ))
        })?;
        let name = scalar_field(frontmatter, "name");
        if name.is_empty() {
            return Err(ObservationFailure::new(format!(
                "Scope file {} missing required frontmatter: name",
                file.display()
            )));
        }
        names.push(name);
    }
    names.sort();
    Ok(names)
}

/// `<stages>/<phase>/*.md` (フェーズ順・名前順)。フェーズディレクトリが無ければ飛ばす。
fn stage_files(root: &Path) -> Result<Vec<StageFileView>, ObservationFailure> {
    let mut files = Vec::new();
    for phase in PHASES {
        let dir = root.join(phase);
        if !dir.exists() {
            continue;
        }
        for file in markdown_files(&dir)? {
            let slug = file
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default();
            let content = std::fs::read_to_string(&file)
                .map_err(|error| ObservationFailure::new(format!("{}: {error}", file.display())));
            files.push(StageFileView::new(phase.to_string(), slug, content));
        }
    }
    Ok(files)
}

/// `.claude/agents/*.md` の frontmatter `name` (名前順、`aidlc.md` は除く — 本家 `loadAgents`)。
fn agents(dir: &Path) -> Result<Vec<String>, ObservationFailure> {
    let mut slugs: Vec<String> = Vec::new();
    for file in markdown_files(dir)? {
        if file.file_name().is_some_and(|name| name == "aidlc.md") {
            continue;
        }
        let body = std::fs::read_to_string(&file)
            .map_err(|error| ObservationFailure::new(format!("{}: {error}", file.display())))?;
        let frontmatter = frontmatter_block(&body).ok_or_else(|| {
            ObservationFailure::new(format!(
                "Agent file missing frontmatter: {}",
                file.display()
            ))
        })?;
        let slug = scalar_field(frontmatter, "name");
        let display_name = scalar_field(frontmatter, "display_name");
        let mut missing = Vec::new();
        if slug.is_empty() {
            missing.push("name");
        }
        if display_name.is_empty() {
            missing.push("display_name");
        }
        if !missing.is_empty() {
            return Err(ObservationFailure::new(format!(
                "Agent file {} missing required frontmatter: {}",
                file.display(),
                missing.join(", ")
            )));
        }
        if slugs.contains(&slug) {
            return Err(ObservationFailure::new(format!(
                "Duplicate agent slug \"{slug}\" in {}. Rename one of them.",
                file.display()
            )));
        }
        slugs.push(slug);
    }
    slugs.sort();
    Ok(slugs)
}
