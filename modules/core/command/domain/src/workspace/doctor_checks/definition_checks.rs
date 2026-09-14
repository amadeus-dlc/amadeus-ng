//! D3.a〜D3.d — 配布シェルの配置とグラフ・スコープ・ステージ schema・参照
//! (本家 2979–2998、4177–4257、4259–4289、4290–4377 の分岐)。

use std::collections::{BTreeMap, BTreeSet};

use crate::workspace::{
    DefinitionAssets, DoctorCheck, DoctorCheckId, GraphStage, ScopeGridEntry, StageFrontmatter,
    WorkspaceShell,
};

/// C7 D3.b が対象とするスコープ (本家の全 11 スコープではなく、今回使う 2 つ)。
const TARGET_SCOPES: [&str; 2] = ["bugfix", "feature"];

/// D3.a、D3.b、D3.c (2 行)、D3.d (2 行)。
pub(super) fn evaluate(shell: &WorkspaceShell, definition: &DefinitionAssets) -> Vec<DoctorCheck> {
    let mut checks = vec![shell_check(shell), scope_validation(definition)];
    checks.push(cycle_detection(definition));
    checks.push(orphan_stage_files(definition));
    checks.push(schema_validation(definition));
    checks.push(graph_references(definition));
    checks
}

fn shell_check(shell: &WorkspaceShell) -> DoctorCheck {
    let label = "workspace shell ready (.claude/ + aidlc/spaces/default/memory/)".to_string();
    if shell.harness_dir_exists() && shell.memory_dir_exists() {
        DoctorCheck::passed(DoctorCheckId::D3a, label)
    } else {
        DoctorCheck::failed(
            DoctorCheckId::D3a,
            label,
            "copy the workspace shell from `dist/claude/` into your project root".to_string(),
        )
    }
}

/// 本家 `validateScope` (lenient) — 1 スコープの errors / advisories。未知スコープは `Err`。
fn validate_scope(
    scope: &str,
    graph: &[GraphStage],
    grid: &[ScopeGridEntry],
    valid_scopes: &[String],
) -> Result<(Vec<String>, Vec<String>), String> {
    if !valid_scopes.iter().any(|name| name == scope) {
        return Err(format!(
            "Unknown scope: \"{scope}\". Valid scopes: {}",
            valid_scopes.join(", ")
        ));
    }
    let enabled: Vec<&GraphStage> = graph.iter().filter(|stage| stage.enabled()).collect();
    let execute: BTreeSet<&str> = grid
        .iter()
        .find(|entry| entry.scope() == scope)
        .map(|entry| entry.execute_slugs().collect())
        .unwrap_or_default();
    let mut on_path: Vec<&GraphStage> = enabled
        .iter()
        .copied()
        .filter(|stage| execute.contains(stage.slug()))
        .collect();
    on_path.sort_by(|a, b| a.numeric_order(b));
    let on_path_slugs: BTreeSet<&str> = on_path.iter().map(|stage| stage.slug()).collect();
    let mut errors = Vec::new();
    let mut advisories = Vec::new();
    for stage in &on_path {
        for consume in stage.artifacts().consumes() {
            if !consume.required() {
                continue;
            }
            let producers: Vec<&str> = enabled
                .iter()
                .filter(|candidate| {
                    candidate
                        .artifacts()
                        .produced_names()
                        .any(|name| name == consume.artifact())
                })
                .map(|candidate| candidate.slug())
                .collect();
            if producers.is_empty() {
                errors.push(format!(
                    "Stage \"{}\" requires artifact \"{}\" but no stage in the graph produces it.",
                    stage.slug(),
                    consume.artifact()
                ));
                continue;
            }
            if !producers.iter().any(|slug| on_path_slugs.contains(slug)) {
                advisories.push(format!(
                    "Stage \"{}\" requires artifact \"{}\" whose producer(s) [{}] are not on the \"{scope}\" path. Ensure existing artifact is current.",
                    stage.slug(),
                    consume.artifact(),
                    producers.join(", ")
                ));
            }
        }
    }
    Ok((errors, advisories))
}

fn scope_validation(definition: &DefinitionAssets) -> DoctorCheck {
    let id = DoctorCheckId::D3b;
    let check_failed = |cause: String| {
        DoctorCheck::failed(id, "Scope validation: check failed".to_string(), cause)
    };
    let (grid, names, graph) = match (
        definition.scope_grid(),
        definition.scope_names(),
        definition.graph(),
    ) {
        (Ok(grid), Ok(names), Ok(graph)) => (grid, names, graph),
        (Err(cause), _, _) | (_, Err(cause), _) | (_, _, Err(cause)) => {
            return check_failed(cause.cause().to_string());
        }
    };
    let mut total_errors = 0;
    let mut total_advisories = 0;
    let mut failing: Vec<String> = Vec::new();
    for scope in TARGET_SCOPES {
        match validate_scope(scope, graph, grid, names) {
            Ok((errors, advisories)) => {
                total_advisories += advisories.len();
                if !errors.is_empty() {
                    total_errors += errors.len();
                    failing.push(format!("{scope}: {}", errors.join("; ")));
                }
            }
            Err(cause) => return check_failed(cause),
        }
    }
    if total_errors == 0 {
        DoctorCheck::passed(
            id,
            format!(
                "Scope validation: {} scopes valid ({total_advisories} advisories)",
                TARGET_SCOPES.len()
            ),
        )
    } else {
        DoctorCheck::failed(
            id,
            format!(
                "Scope validation: {} of {} scopes have errors",
                failing.len(),
                TARGET_SCOPES.len()
            ),
            failing.join(" | "),
        )
    }
}

/// 本家 `findCycles` — Tarjan の強連結成分 (大きさ 2 以上、または自己ループ)。
fn find_cycles(stages: &[&GraphStage]) -> Vec<Vec<String>> {
    struct Tarjan<'a> {
        by_slug: BTreeMap<&'a str, &'a GraphStage>,
        index: BTreeMap<&'a str, usize>,
        lowlink: BTreeMap<&'a str, usize>,
        on_stack: BTreeSet<&'a str>,
        stack: Vec<&'a str>,
        next: usize,
        cycles: Vec<Vec<String>>,
    }
    impl<'a> Tarjan<'a> {
        fn strongconnect(&mut self, v: &'a str) {
            self.index.insert(v, self.next);
            self.lowlink.insert(v, self.next);
            self.next += 1;
            self.stack.push(v);
            self.on_stack.insert(v);
            let deps: Vec<&'a str> = self
                .by_slug
                .get(v)
                .map(|stage| {
                    stage
                        .requires_stage()
                        .iter()
                        .map(String::as_str)
                        .filter(|dep| self.by_slug.contains_key(dep))
                        .collect()
                })
                .unwrap_or_default();
            for w in deps {
                if !self.index.contains_key(w) {
                    self.strongconnect(w);
                    let low = self
                        .lowlink
                        .get(v)
                        .copied()
                        .unwrap_or(0)
                        .min(self.lowlink.get(w).copied().unwrap_or(0));
                    self.lowlink.insert(v, low);
                } else if self.on_stack.contains(w) {
                    let low = self
                        .lowlink
                        .get(v)
                        .copied()
                        .unwrap_or(0)
                        .min(self.index.get(w).copied().unwrap_or(0));
                    self.lowlink.insert(v, low);
                }
            }
            if self.lowlink.get(v) == self.index.get(v) {
                let mut component: Vec<&'a str> = Vec::new();
                while let Some(w) = self.stack.pop() {
                    self.on_stack.remove(w);
                    component.push(w);
                    if w == v {
                        break;
                    }
                }
                let self_loop = component.len() == 1
                    && self
                        .by_slug
                        .get(v)
                        .is_some_and(|stage| stage.requires_itself());
                if component.len() >= 2 || self_loop {
                    self.cycles
                        .push(component.iter().map(|slug| (*slug).to_string()).collect());
                }
            }
        }
    }
    let mut tarjan = Tarjan {
        by_slug: stages.iter().map(|stage| (stage.slug(), *stage)).collect(),
        index: BTreeMap::new(),
        lowlink: BTreeMap::new(),
        on_stack: BTreeSet::new(),
        stack: Vec::new(),
        next: 0,
        cycles: Vec::new(),
    };
    for stage in stages {
        if !tarjan.index.contains_key(stage.slug()) {
            tarjan.strongconnect(stage.slug());
        }
    }
    tarjan.cycles
}

fn cycle_detection(definition: &DefinitionAssets) -> DoctorCheck {
    let id = DoctorCheckId::D3c;
    let graph = match definition.graph() {
        Ok(graph) => graph,
        Err(cause) => {
            return DoctorCheck::failed(
                id,
                "Cycle detection: graph load failed".to_string(),
                cause.cause().to_string(),
            );
        }
    };
    let enabled: Vec<&GraphStage> = graph.iter().filter(|stage| stage.enabled()).collect();
    let cycles = find_cycles(&enabled);
    if cycles.is_empty() {
        DoctorCheck::passed(id, "Cycle detection: 0 cycles".to_string())
    } else {
        DoctorCheck::failed(
            id,
            format!("Cycle detection: {} cycle(s) found", cycles.len()),
            format!(
                "cycles: {}",
                cycles
                    .iter()
                    .map(|cycle| cycle.join(" → "))
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
        )
    }
}

fn orphan_stage_files(definition: &DefinitionAssets) -> DoctorCheck {
    let id = DoctorCheckId::D3c;
    let (graph, files) = match (definition.graph(), definition.stage_files()) {
        (Ok(graph), Ok(files)) => (graph, files),
        (Err(cause), _) | (_, Err(cause)) => {
            return DoctorCheck::failed(
                id,
                "Orphan stage files: check failed".to_string(),
                cause.cause().to_string(),
            );
        }
    };
    let graph_slugs: BTreeSet<&str> = graph.iter().map(|stage| stage.slug()).collect();
    let disk_slugs: BTreeSet<&str> = files.iter().map(|file| file.slug()).collect();
    let missing: Vec<&str> = graph_slugs
        .iter()
        .copied()
        .filter(|slug| !disk_slugs.contains(slug))
        .collect();
    if missing.is_empty() {
        DoctorCheck::passed(
            id,
            format!(
                "Orphan stage files: {} graph entries all have files",
                graph_slugs.len()
            ),
        )
    } else {
        DoctorCheck::failed(
            id,
            format!(
                "Orphan stage files: {} graph entries have no file on disk",
                missing.len()
            ),
            format!("missing files: {}", missing.join(", ")),
        )
    }
}

fn schema_validation(definition: &DefinitionAssets) -> DoctorCheck {
    let id = DoctorCheckId::D3d;
    let check_failed = |cause: String| {
        DoctorCheck::failed(id, "Schema validation: check failed".to_string(), cause)
    };
    let (graph, agents, files) = match (
        definition.graph(),
        definition.agents(),
        definition.stage_files(),
    ) {
        (Ok(graph), Ok(agents), Ok(files)) => (graph, agents, files),
        (Err(cause), _, _) | (_, Err(cause), _) | (_, _, Err(cause)) => {
            return check_failed(cause.cause().to_string());
        }
    };
    let mut attempted = 0;
    let mut failures: Vec<String> = Vec::new();
    for stage in graph {
        let Some(file) = files
            .iter()
            .find(|file| file.phase() == stage.phase() && file.slug() == stage.slug())
        else {
            continue;
        };
        attempted += 1;
        let raw = match file.content() {
            Ok(raw) => raw,
            Err(cause) => return check_failed(cause.cause().to_string()),
        };
        let errors = match StageFrontmatter::parse(raw) {
            Ok(parsed) => {
                let context = (stage.phase() != "initialization").then_some(agents.as_slice());
                parsed.schema_errors(context)
            }
            Err(cause) => vec![cause],
        };
        if let Some(first) = errors.first() {
            failures.push(format!("{}: {first}", stage.slug()));
        }
    }
    if failures.is_empty() {
        DoctorCheck::passed(
            id,
            format!("Schema validation: {attempted}/{attempted} stages validated"),
        )
    } else {
        DoctorCheck::failed(
            id,
            format!(
                "Schema validation: {} of {attempted} stage(s) failed",
                failures.len()
            ),
            failures.join("; "),
        )
    }
}

fn graph_references(definition: &DefinitionAssets) -> DoctorCheck {
    let id = DoctorCheckId::D3d;
    let graph = match definition.graph() {
        Ok(graph) => graph,
        Err(cause) => {
            return DoctorCheck::failed(
                id,
                "Graph references: check failed".to_string(),
                cause.cause().to_string(),
            );
        }
    };
    let slugs: BTreeSet<&str> = graph.iter().map(|stage| stage.slug()).collect();
    let artifacts: BTreeSet<&str> = graph
        .iter()
        .flat_map(|stage| stage.artifacts().produced_names())
        .collect();
    let mut broken: Vec<String> = Vec::new();
    for stage in graph {
        for consume in stage.artifacts().consumes() {
            if !artifacts.contains(consume.artifact()) {
                broken.push(format!(
                    "{}: consumes unknown artifact \"{}\"",
                    stage.slug(),
                    consume.artifact()
                ));
            }
        }
        for required in stage.requires_stage() {
            if !slugs.contains(required.as_str()) {
                broken.push(format!(
                    "{}: requires_stage unknown slug \"{required}\"",
                    stage.slug()
                ));
            }
        }
    }
    if broken.is_empty() {
        DoctorCheck::passed(
            id,
            format!(
                "Graph references: {} artifacts + edges resolved",
                artifacts.len()
            ),
        )
    } else {
        DoctorCheck::failed(
            id,
            format!("Graph references: {} broken reference(s)", broken.len()),
            broken.join("; "),
        )
    }
}
