//! ステージ frontmatter の schema 検証 (本家 `aidlc-stage-schema.ts validateStageFrontmatter`
//! の写し — 規則 1〜9 を同じ順・同じ綴りで)。[`StageFrontmatter`](super::StageFrontmatter) の
//! 私有の検査手順であり、公開型は持たない。

use super::FrontmatterValue;

/// 発見順の項目 (値オブジェクトの表現の借用)。
type Frontmatter = [(String, FrontmatterValue)];

const VALID_PHASES: [&str; 5] = [
    "initialization",
    "ideation",
    "inception",
    "construction",
    "operation",
];
const VALID_EXECUTIONS: [&str; 2] = ["ALWAYS", "CONDITIONAL"];
const VALID_MODES: [&str; 5] = ["inline", "subagent", "pipeline", "mob", "agent-team"];
const ENSEMBLE_MODES: [&str; 2] = ["pipeline", "mob"];
const VALID_CONDITIONAL_ON: [&str; 2] = ["brownfield", "greenfield"];
const RESERVED_AGENT_SLUG: &str = "orchestrator";
const RESERVED_KEYS: [(&str, &str); 4] = [
    ("on_failure", "loop driver"),
    ("blocks_on", "construction worktrees"),
    ("timeout", "sensor binding"),
    ("retry", "loop driver"),
];
const WHEN_PREDICATE_KEYS: [&str; 1] = ["producer-in-plan"];
const UNIT_KINDS: [&str; 5] = ["service", "spec", "ui", "packaging", "library"];
const REQUIRED_FIELDS: [&str; 12] = [
    "slug",
    "phase",
    "execution",
    "condition",
    "lead_agent",
    "support_agents",
    "mode",
    "produces",
    "consumes",
    "requires_stage",
    "inputs",
    "outputs",
];
const OPTIONAL_FIELDS: [&str; 16] = [
    "number",
    "name",
    "plugin",
    "for_each",
    "workspace_requires",
    "optional_produces",
    "produces_kinds",
    "sensors",
    "scopes",
    "reviewer",
    "review_artifact",
    "reviewer_max_iterations",
    "review_class",
    "summary_confirmation",
    "when",
    "required_sections",
];

/// 検証 (エラーが無ければ空)。`agents` を渡したときだけ規則 9 (役割の登録照合) を行う。
pub(super) fn validate(object: &Frontmatter, agents: Option<&[String]>) -> Vec<String> {
    let mut errors = Vec::new();
    for (key, _) in object {
        if let Some((_, reason)) = RESERVED_KEYS.iter().find(|(reserved, _)| reserved == key) {
            errors.push(format!("{key} is reserved ({reason}); not active yet"));
        }
    }
    for (key, _) in object {
        if key == "bundle" {
            errors.push("bundle: was renamed; write plugin: for ownership".to_string());
            continue;
        }
        let known = REQUIRED_FIELDS.contains(&key.as_str())
            || OPTIONAL_FIELDS.contains(&key.as_str())
            || RESERVED_KEYS.iter().any(|(reserved, _)| reserved == key);
        if !known {
            errors.push(format!("unknown key: {key}"));
        }
    }
    for field in REQUIRED_FIELDS {
        if get(object, field).is_none() {
            errors.push(format!("missing required field: {field}"));
        }
    }
    check_string(object, "slug", &mut errors);
    check_pattern(object, "slug", is_kebab, "kebab-case", &mut errors);
    check_string(object, "number", &mut errors);
    check_pattern(
        object,
        "number",
        is_stage_number,
        "<phase-prefix>.<index>",
        &mut errors,
    );
    check_string(object, "name", &mut errors);
    check_string(object, "plugin", &mut errors);
    check_string(object, "phase", &mut errors);
    check_enum(object, "phase", &VALID_PHASES, &mut errors);
    check_string(object, "execution", &mut errors);
    check_enum(object, "execution", &VALID_EXECUTIONS, &mut errors);
    check_string(object, "condition", &mut errors);
    check_string(object, "lead_agent", &mut errors);
    check_string_array(object, "support_agents", &mut errors);
    check_string(object, "mode", &mut errors);
    check_enum(object, "mode", &VALID_MODES, &mut errors);
    if let Some(mode) = text(object, "mode")
        && ENSEMBLE_MODES.contains(&mode)
    {
        let empty = match get(object, "support_agents") {
            Some(FrontmatterValue::Sequence(items)) => items.is_empty(),
            _ => true,
        };
        if empty {
            errors.push(format!(
                "mode \"{mode}\" requires a non-empty support_agents"
            ));
        }
    }
    if text(object, "mode") == Some("pipeline")
        && let Some(lead) = text(object, "lead_agent")
        && let Some(FrontmatterValue::Sequence(support)) = get(object, "support_agents")
    {
        let mut seen = vec![lead.to_string()];
        for agent in support {
            let FrontmatterValue::Text(agent) = agent else {
                continue;
            };
            if seen.contains(agent) {
                errors.push(format!(
                    "mode \"pipeline\" requires unique lead/support chain entries; duplicate agent \"{agent}\""
                ));
                break;
            }
            seen.push(agent.clone());
        }
    }
    if let Some(value) = get(object, "for_each")
        && !matches!(value, FrontmatterValue::Text(_))
    {
        errors.push(format!("for_each must be string, got {}", describe(value)));
    }
    if let Some(value) = get(object, "workspace_requires")
        && !matches!(value, FrontmatterValue::Boolean(_))
    {
        errors.push(format!(
            "workspace_requires must be boolean, got {}",
            describe(value)
        ));
    }
    check_string(object, "reviewer", &mut errors);
    check_string(object, "review_artifact", &mut errors);
    if let Some(value) = get(object, "reviewer_max_iterations")
        && !matches!(value, FrontmatterValue::Integer(n) if *n >= 1)
    {
        errors.push(format!(
            "reviewer_max_iterations must be a positive integer, got {}",
            describe(value)
        ));
    }
    if let Some(value) = get(object, "summary_confirmation")
        && !matches!(value, FrontmatterValue::Text(t) if t == "required" || t == "if-present")
    {
        errors.push(format!(
            "summary_confirmation must be one of required, if-present, got {}",
            describe(value)
        ));
    }
    let reviewer_declared = get(object, "reviewer").is_some();
    if get(object, "reviewer_max_iterations").is_some() && !reviewer_declared {
        errors.push("reviewer_max_iterations requires a reviewer".to_string());
    }
    let review_artifact_declared = get(object, "review_artifact").is_some();
    if reviewer_declared && !review_artifact_declared {
        errors.push("reviewer requires review_artifact".to_string());
    }
    if review_artifact_declared && !reviewer_declared {
        errors.push("review_artifact requires a reviewer".to_string());
    }
    if let Some(value) = get(object, "review_class") {
        if !matches!(value, FrontmatterValue::Text(t) if t == "adversarial" || t == "advisory") {
            errors.push("review_class must be \"adversarial\" or \"advisory\"".to_string());
        }
        if !reviewer_declared {
            errors.push("review_class requires a reviewer".to_string());
        }
    }
    if let Some(value) = get(object, "required_sections") {
        check_string_array(object, "required_sections", &mut errors);
        if let FrontmatterValue::Sequence(items) = value {
            for (index, item) in items.iter().enumerate() {
                if let FrontmatterValue::Text(section) = item
                    && section.trim().is_empty()
                {
                    errors.push(format!("required_sections[{index}] must be non-empty"));
                }
            }
        }
    }
    if let Some(value) = get(object, "when") {
        match value {
            FrontmatterValue::Mapping(entries) => {
                if entries.len() != 1 {
                    errors.push(format!(
                        "when must have exactly one predicate key, got {}",
                        entries.len()
                    ));
                }
                for (key, predicate) in entries {
                    if !WHEN_PREDICATE_KEYS.contains(&key.as_str()) {
                        errors.push(format!(
                            "when has unknown predicate \"{key}\"; allowed: {}",
                            WHEN_PREDICATE_KEYS.join(" | ")
                        ));
                    } else if !matches!(predicate, FrontmatterValue::Text(t) if !t.trim().is_empty())
                    {
                        errors.push(format!("when.{key} must be a non-empty artifact slug"));
                    }
                }
            }
            other => errors.push(format!("when must be object, got {}", describe(other))),
        }
    }
    check_string_array(object, "produces", &mut errors);
    if let Some(value) = get(object, "optional_produces") {
        check_string_array(object, "optional_produces", &mut errors);
        if let FrontmatterValue::Sequence(items) = value {
            for (index, item) in items.iter().enumerate() {
                if let FrontmatterValue::Text(name) = item
                    && !is_kebab(name)
                {
                    errors.push(format!(
                        "optional_produces[{index}] must be kebab-case, got \"{name}\""
                    ));
                }
            }
        }
    }
    if let Some(value) = get(object, "produces_kinds") {
        match value {
            FrontmatterValue::Mapping(entries) => {
                let mut declared: Vec<&str> = Vec::new();
                for key in ["produces", "optional_produces"] {
                    if let Some(FrontmatterValue::Sequence(items)) = get(object, key) {
                        for item in items {
                            if let FrontmatterValue::Text(name) = item {
                                declared.push(name);
                            }
                        }
                    }
                }
                for (name, kinds) in entries {
                    if !is_kebab(name) {
                        errors.push(format!("produces_kinds key \"{name}\" is not kebab-case"));
                    } else if !declared.contains(&name.as_str()) {
                        errors.push(format!("produces_kinds key \"{name}\" is not in produces"));
                    }
                    match kinds {
                        FrontmatterValue::Sequence(items) if !items.is_empty() => {
                            for kind in items {
                                let known = matches!(kind, FrontmatterValue::Text(k) if UNIT_KINDS.contains(&k.as_str()));
                                if !known {
                                    let shown = match kind {
                                        FrontmatterValue::Text(k) => k.clone(),
                                        other => describe(other).to_string(),
                                    };
                                    errors.push(format!(
                                        "produces_kinds.{name} lists unknown kind \"{shown}\""
                                    ));
                                }
                            }
                        }
                        _ => errors.push(format!(
                            "produces_kinds.{name} must be a non-empty list of unit kinds"
                        )),
                    }
                }
            }
            other => errors.push(format!(
                "produces_kinds must be object, got {}",
                describe(other)
            )),
        }
    }
    if let Some(review_artifact) = text(object, "review_artifact") {
        let required_produces: Vec<&str> = match get(object, "produces") {
            Some(FrontmatterValue::Sequence(items)) => items
                .iter()
                .filter_map(|item| match item {
                    FrontmatterValue::Text(name) => Some(name.as_str()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        if !required_produces.contains(&review_artifact) {
            errors.push(format!(
                "review_artifact \"{review_artifact}\" must name a required produces entry"
            ));
        }
        if !artifact_filename(review_artifact).ends_with(".md") {
            errors.push(format!(
                "review_artifact \"{review_artifact}\" must resolve to a Markdown artifact"
            ));
        }
        if text(object, "for_each") == Some("unit-of-work")
            && let Some(FrontmatterValue::Mapping(kinds_map)) = get(object, "produces_kinds")
        {
            let kinds_of = |name: &str| -> Option<Vec<String>> {
                kinds_map
                    .iter()
                    .find(|(key, _)| key == name)
                    .and_then(|(_, kinds)| match kinds {
                        FrontmatterValue::Sequence(items) => Some(
                            items
                                .iter()
                                .filter_map(|kind| match kind {
                                    FrontmatterValue::Text(k) => Some(k.clone()),
                                    _ => None,
                                })
                                .collect(),
                        ),
                        _ => None,
                    })
            };
            let mut applicable: Vec<String> = Vec::new();
            for name in &required_produces {
                let kinds = kinds_of(name)
                    .unwrap_or_else(|| UNIT_KINDS.iter().map(|kind| (*kind).to_string()).collect());
                for kind in kinds {
                    if !applicable.contains(&kind) {
                        applicable.push(kind);
                    }
                }
            }
            let target = kinds_of(review_artifact)
                .unwrap_or_else(|| UNIT_KINDS.iter().map(|kind| (*kind).to_string()).collect());
            let missing: Vec<&str> = applicable
                .iter()
                .filter(|kind| !target.contains(kind))
                .map(String::as_str)
                .collect();
            if !missing.is_empty() {
                errors.push(format!(
                    "review_artifact \"{review_artifact}\" is pruned for applicable unit kinds: {}",
                    missing.join(", ")
                ));
            }
        }
    }
    if let Some(value) = get(object, "consumes") {
        match value {
            FrontmatterValue::Sequence(items) => {
                for (index, entry) in items.iter().enumerate() {
                    let FrontmatterValue::Mapping(fields) = entry else {
                        errors.push(format!(
                            "consumes[{index}] must be object, got {}",
                            describe(entry)
                        ));
                        continue;
                    };
                    match fields.iter().find(|(key, _)| key == "artifact") {
                        None => errors.push(format!("consumes[{index}].artifact missing")),
                        Some((_, FrontmatterValue::Text(artifact))) => {
                            if !is_kebab(artifact) {
                                errors.push(format!(
                                    "consumes[{index}].artifact must be kebab-case, got \"{artifact}\""
                                ));
                            }
                        }
                        Some((_, other)) => errors.push(format!(
                            "consumes[{index}].artifact must be string, got {}",
                            describe(other)
                        )),
                    }
                    match fields.iter().find(|(key, _)| key == "required") {
                        None => errors.push(format!("consumes[{index}].required missing")),
                        Some((_, FrontmatterValue::Boolean(_))) => {}
                        Some((_, other)) => errors.push(format!(
                            "consumes[{index}].required must be boolean, got {}",
                            describe(other)
                        )),
                    }
                    if let Some((_, conditional)) =
                        fields.iter().find(|(key, _)| key == "conditional_on")
                    {
                        match conditional {
                            FrontmatterValue::Text(value)
                                if !VALID_CONDITIONAL_ON.contains(&value.as_str()) =>
                            {
                                errors.push(format!(
                                    "consumes[{index}].conditional_on must be one of {}, got \"{value}\"",
                                    VALID_CONDITIONAL_ON.join(" | ")
                                ));
                            }
                            FrontmatterValue::Text(_) => {}
                            other => errors.push(format!(
                                "consumes[{index}].conditional_on must be string, got {}",
                                describe(other)
                            )),
                        }
                    }
                }
            }
            other => errors.push(format!("consumes must be array, got {}", describe(other))),
        }
    }
    check_string_array(object, "requires_stage", &mut errors);
    for key in ["sensors", "scopes"] {
        if let Some(value) = get(object, key) {
            check_string_array(object, key, &mut errors);
            if let FrontmatterValue::Sequence(items) = value {
                for (index, item) in items.iter().enumerate() {
                    if let FrontmatterValue::Text(id) = item
                        && id.is_empty()
                    {
                        errors.push(format!("{key}[{index}] must be non-empty"));
                    }
                }
            }
        }
    }
    check_string(object, "inputs", &mut errors);
    check_string(object, "outputs", &mut errors);
    if let Some(known) = agents {
        let registered =
            |slug: &str| slug == RESERVED_AGENT_SLUG || known.iter().any(|a| a == slug);
        if let Some(lead) = text(object, "lead_agent")
            && !registered(lead)
        {
            errors.push(format!(
                "lead_agent \"{lead}\" has no matching .claude/agents/*.md"
            ));
        }
        if let Some(FrontmatterValue::Sequence(support)) = get(object, "support_agents") {
            for (index, agent) in support.iter().enumerate() {
                if let FrontmatterValue::Text(agent) = agent
                    && !registered(agent)
                {
                    errors.push(format!(
                        "support_agents[{index}] \"{agent}\" has no matching .claude/agents/*.md"
                    ));
                }
            }
        }
        if let Some(reviewer) = text(object, "reviewer")
            && !registered(reviewer)
        {
            errors.push(format!(
                "reviewer \"{reviewer}\" has no matching .claude/agents/*.md"
            ));
        }
    }
    errors
}

fn get<'a>(object: &'a Frontmatter, key: &str) -> Option<&'a FrontmatterValue> {
    super::get(object, key)
}

fn text<'a>(object: &'a Frontmatter, key: &str) -> Option<&'a str> {
    match get(object, key) {
        Some(FrontmatterValue::Text(value)) => Some(value.as_str()),
        _ => None,
    }
}

/// JS の `typeof` 相当 (本家 `describe`)。
const fn describe(value: &FrontmatterValue) -> &'static str {
    match value {
        FrontmatterValue::Text(_) => "string",
        FrontmatterValue::Boolean(_) => "boolean",
        FrontmatterValue::Integer(_) => "number",
        FrontmatterValue::Sequence(_) => "array",
        FrontmatterValue::Mapping(_) => "object",
    }
}

fn check_string(object: &Frontmatter, field: &str, errors: &mut Vec<String>) {
    if let Some(value) = get(object, field)
        && !matches!(value, FrontmatterValue::Text(_))
    {
        errors.push(format!("{field} must be string, got {}", describe(value)));
    }
}

fn check_string_array(object: &Frontmatter, field: &str, errors: &mut Vec<String>) {
    let Some(value) = get(object, field) else {
        return;
    };
    let FrontmatterValue::Sequence(items) = value else {
        errors.push(format!("{field} must be array, got {}", describe(value)));
        return;
    };
    for (index, item) in items.iter().enumerate() {
        if !matches!(item, FrontmatterValue::Text(_)) {
            errors.push(format!(
                "{field}[{index}] must be string, got {}",
                describe(item)
            ));
        }
    }
}

fn check_enum(object: &Frontmatter, field: &str, allowed: &[&str], errors: &mut Vec<String>) {
    if let Some(value) = text(object, field)
        && !allowed.contains(&value)
    {
        errors.push(format!(
            "{field} must be one of {}, got \"{value}\"",
            allowed.join(" | ")
        ));
    }
}

fn check_pattern(
    object: &Frontmatter,
    field: &str,
    accepts: fn(&str) -> bool,
    shape: &str,
    errors: &mut Vec<String>,
) {
    if let Some(value) = text(object, field)
        && !accepts(value)
    {
        errors.push(format!("{field} must be {shape}, got \"{value}\""));
    }
}

/// `^[a-z][a-z0-9-]*$`。
fn is_kebab(value: &str) -> bool {
    let mut chars = value.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// `^\d+\.\d+$`。
fn is_stage_number(value: &str) -> bool {
    value.split_once('.').is_some_and(|(phase, index)| {
        !phase.is_empty()
            && !index.is_empty()
            && phase.bytes().all(|b| b.is_ascii_digit())
            && index.bytes().all(|b| b.is_ascii_digit())
    })
}

/// 本家 `artifactFilename` — 例外表以外は `<name>.md`。
fn artifact_filename(name: &str) -> String {
    match name {
        "build-test-results" | "load-test-results" => "test-results.md".to_string(),
        "traceability" => "traceability.json".to_string(),
        _ => format!("{name}.md"),
    }
}
