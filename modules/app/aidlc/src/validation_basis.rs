//! 完了時の実ファイル・配布グラフから検証根拠を採取する入力境界。
use crate::layout::Layout;
use core_command_domain::orchestration::StageValidation;
use core_infrastructure::{canon_json, hash::sha256_hex};
use serde_json::Value;
use std::{fs, path::Path};

pub(crate) fn read(layout: &Layout, requested: Option<&str>) -> Option<StageValidation> {
    let state = fs::read_to_string(layout.state_file()?).ok()?;
    let stage = requested.or_else(|| field(&state, "Current Stage"))?;
    let result = (|| {
        let graph: Vec<Value> = serde_json::from_slice(
            &fs::read(layout.definition_data_dir().join("stage-graph.json"))
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let node = graph
            .iter()
            .find(|n| text(n, "slug") == stage)
            .ok_or_else(|| format!("Unknown stage: {stage}"))?;
        let record = layout.record_dir().ok_or("No active intent record")?;
        let relative = record
            .strip_prefix(layout.project_dir())
            .map_err(|e| e.to_string())?;
        capture(
            layout.project_dir(),
            relative,
            layout.space(),
            node,
            &graph,
            &state,
        )
    })();
    Some(match result {
        Ok(value) => StageValidation::Basis(value),
        Err(error) => StageValidation::Warning(format!(
            "Validity receipt omitted for stage \"{stage}\": {}",
            error
                .split(['\r', '\n', '\u{2028}', '\u{2029}'])
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
        )),
    })
}

fn object<const N: usize>(fields: [(&str, Value); N]) -> Value {
    Value::Object(
        fields
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    )
}
fn text<'a>(value: &'a Value, name: &str) -> &'a str {
    value.get(name).and_then(Value::as_str).unwrap_or("")
}
fn field<'a>(state: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("- **{name}**:");
    state
        .lines()
        .find_map(|line| line.strip_prefix(&prefix).map(str::trim))
}
fn canonical(value: &Value) -> Result<String, String> {
    let value = canon_json::to_value(value).map_err(|e| e.to_string())?;
    Ok(canon_json::serialize(
        &value,
        canon_json::SerializationProfile::HashCanonical,
    ))
}
fn digest(value: &Value) -> Result<String, String> {
    Ok(format!(
        "sha256:{}",
        sha256_hex(canonical(value)?.as_bytes())
    ))
}
fn values(value: &Value, name: &str) -> Vec<Value> {
    value
        .get(name)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}
fn capture(
    root: &Path,
    record: &Path,
    space: &str,
    stage: &Value,
    graph: &[Value],
    state: &str,
) -> Result<String, String> {
    let mut contract = serde_json::Map::new();
    for key in [
        "slug",
        "phase",
        "execution",
        "condition",
        "for_each",
        "workspace_requires",
    ] {
        if let Some(value) = stage.get(key) {
            contract.insert(key.into(), value.clone());
        }
    }
    for key in ["consumes", "produces", "optional_produces"] {
        contract.insert(key.into(), Value::Array(values(stage, key)));
    }
    contract.insert(
        "produces_kinds".into(),
        stage
            .get("produces_kinds")
            .cloned()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new())),
    );
    let project_type = field(state, "Project Type")
        .map(str::to_lowercase)
        .filter(|value| matches!(value.as_str(), "brownfield" | "greenfield"));
    let mut inputs = Vec::new();
    for consume in values(stage, "consumes") {
        let conditional = text(&consume, "conditional_on");
        if !conditional.is_empty()
            && project_type
                .as_deref()
                .is_some_and(|kind| kind != conditional)
        {
            continue;
        }
        let artifact = text(&consume, "artifact");
        let required = consume.get("required") != Some(&Value::Bool(false));
        let owners: Vec<_> = graph
            .iter()
            .filter(|node| {
                values(node, "produces")
                    .iter()
                    .chain(values(node, "optional_produces").iter())
                    .any(|value| value.as_str() == Some(artifact))
            })
            .collect();
        if !required && owners.is_empty() {
            continue;
        }
        let [owner] = owners.as_slice() else {
            return Err(format!(
                "Cannot capture validity for artifact \"{artifact}\": expected exactly one producer, found {}.",
                owners.len()
            ));
        };
        if let Some(basis) = artifact_basis(root, record, space, artifact, owner, required, state)?
        {
            inputs.push(basis);
        }
    }
    let mut outputs = Vec::new();
    let required = values(stage, "produces");
    let optional = values(stage, "optional_produces");
    let mut seen = std::collections::BTreeSet::new();
    for artifact in required
        .iter()
        .chain(optional.iter())
        .filter_map(Value::as_str)
    {
        if !seen.insert(artifact) {
            continue;
        }
        if let Some(basis) = artifact_basis(
            root,
            record,
            space,
            artifact,
            stage,
            required.iter().any(|v| v.as_str() == Some(artifact)),
            state,
        )? {
            outputs.push(basis);
        }
    }
    for bases in [&mut inputs, &mut outputs] {
        // 本家はartifact + NUL + producerをlocaleCompareする。契約識別子は
        // 小文字ASCIIのslugで、照合で無視されるNULを比較キーへ含めない。
        bases
            .sort_by_key(|basis| format!("{}{}", text(basis, "artifact"), text(basis, "producer")));
    }
    canonical(&object([
        ("schema", Value::from(3)),
        (
            "graphContract",
            Value::from(digest(&Value::Object(contract))?),
        ),
        ("projectType", project_type.map_or(Value::Null, Value::from)),
        ("inputs", Value::Array(inputs)),
        ("outputs", Value::Array(outputs)),
    ]))
}
fn artifact_basis(
    root: &Path,
    record: &Path,
    space: &str,
    artifact: &str,
    owner: &Value,
    required: bool,
    state: &str,
) -> Result<Option<Value>, String> {
    let filename = match artifact {
        "build-test-results" | "load-test-results" => "test-results.md".into(),
        "traceability" => "traceability.json".into(),
        name => format!("{name}.md"),
    };
    let slug = text(owner, "slug");
    let path = if slug == "reverse-engineering" {
        let repo = root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("workspace");
        Path::new("aidlc")
            .join("spaces")
            .join(space)
            .join("codekb")
            .join(repo)
            .join(filename)
    } else {
        if text(owner, "for_each") == "unit-of-work"
            && !matches!(
                field(state, "Scope"),
                Some("bugfix" | "security-patch" | "poc" | "refactor" | "express")
            )
        {
            return Err(format!(
                "Unit-scoped validity capture is not connected for stage \"{slug}\"."
            ));
        }
        record.join(text(owner, "phase")).join(slug).join(filename)
    };
    let absolute = root.join(&path);
    let (hash, present) = if !absolute.exists() {
        ("missing".to_string(), false)
    } else {
        match fs::metadata(&absolute).and_then(|metadata| {
            if metadata.is_file() {
                fs::read(&absolute).map(Some)
            } else {
                Ok(None)
            }
        }) {
            Ok(Some(bytes)) => (format!("sha256:{}", sha256_hex(&bytes)), true),
            Ok(None) => ("not-a-file".to_string(), false),
            Err(error) => (
                format!("unreadable:{}", sha256_hex(error.to_string().as_bytes())),
                false,
            ),
        }
    };
    if !required && !present {
        return Ok(None);
    }
    let path = path.to_string_lossy().replace('\\', "/");
    let structure = Value::Array(vec![object([
        ("path", Value::from(path.clone())),
        ("unit", Value::Null),
        ("unitKind", Value::Null),
    ])]);
    let content = Value::Array(vec![object([
        ("path", Value::from(path)),
        ("sha256", Value::from(hash)),
        ("present", Value::from(present)),
    ])]);
    Ok(Some(object([
        ("artifact", Value::from(artifact)),
        ("producer", Value::from(slug)),
        ("required", Value::from(required)),
        ("instanceCount", Value::from(1)),
        ("presentCount", Value::from(usize::from(present))),
        ("structureHash", Value::from(digest(&structure)?)),
        ("contentHash", Value::from(digest(&content)?)),
    ])))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_input_boundary_matches_fixed_upstream_validation_fields() {
        let corpus: Value = serde_json::from_str(include_str!(
            "../../../../tests/golden/selfhost-stage1/validation-basis.json"
        ))
        .unwrap();
        for case in corpus.get("observations").unwrap().as_array().unwrap() {
            let parent = tempfile::tempdir().unwrap();
            let root = parent.path().join("workspace");
            let input = case.get("input").unwrap();
            let record = input.get("record").unwrap().as_str().unwrap();
            fs::create_dir_all(root.join(record)).unwrap();
            for (path, value) in input.get("files").unwrap().as_object().unwrap() {
                let path = root.join(path);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, value.as_str().unwrap()).unwrap();
            }
            for path in input.get("directories").unwrap().as_array().unwrap() {
                fs::create_dir_all(root.join(path.as_str().unwrap())).unwrap();
            }
            let stage = input.get("stage").unwrap();
            let observed = capture(
                &root,
                Path::new(record),
                "default",
                stage,
                input.get("stages").unwrap().as_array().unwrap(),
                input.get("state").unwrap().as_str().unwrap(),
            );
            let actual = match observed {
                Ok(basis) => object([("Validation Basis", Value::from(basis))]),
                Err(message) => object([(
                    "Validation Warning",
                    Value::from(format!(
                        "Validity receipt omitted for stage \"{}\": {message}",
                        text(stage, "slug")
                    )),
                )]),
            };
            assert_eq!(
                &actual,
                case.get("fields").unwrap(),
                "{}",
                case.get("id").unwrap()
            );
        }
    }

    fn stage(json: &str) -> Value {
        serde_json::from_str(json).unwrap()
    }

    /// 同じ成果物名を `produces` と `optional_produces` の両方に挙げても 1 度しか採取しない。
    /// per-unit 段の採取は bugfix 系の scope だけが繋がっている。
    #[test]
    fn duplicate_artifacts_are_captured_once_and_unit_scoped_stages_are_gated_by_scope() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("workspace");
        let record = Path::new("aidlc/spaces/default/intents/260101-basis-aaaaaaaa");
        let stage_dir = root.join(record).join("construction/code-generation");
        fs::create_dir_all(&stage_dir).unwrap();
        fs::write(stage_dir.join("code.md"), "# code\n").unwrap();
        let node = stage(
            r#"{"slug":"code-generation","phase":"construction","for_each":"unit-of-work",
                "produces":["code"],"optional_produces":["code"]}"#,
        );
        let graph = vec![node.clone()];
        let basis = capture(
            &root,
            record,
            "default",
            &node,
            &graph,
            "- **Scope**: bugfix\n",
        )
        .unwrap();
        assert_eq!(basis.matches("\"artifact\":\"code\"").count(), 1, "{basis}");
        let warning = capture(
            &root,
            record,
            "default",
            &node,
            &graph,
            "- **Scope**: classic\n",
        )
        .unwrap_err();
        assert_eq!(
            warning,
            "Unit-scoped validity capture is not connected for stage \"code-generation\"."
        );
    }

    /// ファイルでない成果物は `not-a-file`、読めない成果物は `unreadable:` の要約で載る。
    #[cfg(unix)]
    #[test]
    fn irregular_and_unreadable_artifacts_are_summarised_without_their_bytes() {
        use std::os::unix::fs::PermissionsExt as _;
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("workspace");
        let record = Path::new("aidlc/spaces/default/intents/260101-basis-aaaaaaaa");
        let stage_dir = root.join(record).join("inception/domain-design");
        fs::create_dir_all(stage_dir.join("design.md")).unwrap();
        fs::write(stage_dir.join("notes.md"), "secret\n").unwrap();
        fs::set_permissions(
            stage_dir.join("notes.md"),
            fs::Permissions::from_mode(0o000),
        )
        .unwrap();
        let node =
            stage(r#"{"slug":"domain-design","phase":"inception","produces":["design","notes"]}"#);
        let graph = vec![node.clone()];
        let hidden = capture(
            &root,
            record,
            "default",
            &node,
            &graph,
            "- **Scope**: classic\n",
        )
        .unwrap();
        fs::set_permissions(
            stage_dir.join("notes.md"),
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        let readable = capture(
            &root,
            record,
            "default",
            &node,
            &graph,
            "- **Scope**: classic\n",
        )
        .unwrap();
        assert!(
            hidden.contains("\"artifact\":\"design\",\"contentHash\":\"sha256:"),
            "{hidden}"
        );
        assert_eq!(hidden.matches("\"presentCount\":0").count(), 2, "{hidden}");
        if nix_is_root() {
            return;
        }
        assert_ne!(hidden, readable, "読めない成果物の要約は読めるときと異なる");
        assert_eq!(
            readable.matches("\"presentCount\":1").count(),
            1,
            "{readable}"
        );
    }

    fn nix_is_root() -> bool {
        std::process::Command::new("id")
            .arg("-u")
            .output()
            .ok()
            .is_some_and(|output| String::from_utf8_lossy(&output.stdout).trim() == "0")
    }

    /// `read` は状態ファイルの `Current Stage` を既定にし、未知の stage は警告に畳む。
    #[test]
    fn read_defaults_to_the_current_stage_and_warns_on_unknown_stages() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join(".claude/tools/data");
        fs::create_dir_all(&data).unwrap();
        fs::write(
            data.join("stage-graph.json"),
            r#"[{"slug":"domain-design","phase":"inception","produces":["design"]}]"#,
        )
        .unwrap();
        let intents = root.path().join("aidlc/spaces/default/intents");
        let record = intents.join("260101-basis-aaaaaaaa");
        fs::create_dir_all(record.join("inception/domain-design")).unwrap();
        fs::write(
            record.join("aidlc-state.md"),
            "- **Scope**: classic\n- **Current Stage**: domain-design\n",
        )
        .unwrap();
        fs::write(intents.join("active-intent"), "260101-basis-aaaaaaaa\n").unwrap();
        let layout = Layout::resolve(root.path());
        assert!(matches!(
            read(&layout, None),
            Some(core_command_domain::orchestration::StageValidation::Basis(
                _
            ))
        ));
        assert!(matches!(
            read(&layout, Some("nowhere")),
            Some(core_command_domain::orchestration::StageValidation::Warning(warning))
                if warning == "Validity receipt omitted for stage \"nowhere\": Unknown stage: nowhere"
        ));
        assert!(
            read(
                &Layout::resolve(root.path().join("elsewhere").as_path()),
                None
            )
            .is_none()
        );
    }

    /// 定義グラフが無い・壊れている・記録が作業空間の外にあるなら、根拠は警告になる。
    #[test]
    fn an_unreadable_graph_or_a_foreign_record_yields_a_warning_not_a_basis() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("workspace");
        let record = root.join("aidlc/spaces/default/intents/260101-basis-aaaaaaaa");
        fs::create_dir_all(&record).unwrap();
        fs::write(
            record.join("aidlc-state.md"),
            "# state\n- **Current Stage**: intent-capture\n",
        )
        .unwrap();
        fs::write(
            root.join("aidlc/spaces/default/intents/active-intent"),
            "260101-basis-aaaaaaaa\n",
        )
        .unwrap();
        let layout = Layout::resolve(&root);
        // 失敗時だけ評価される行を作らないよう、Debug 表示の包含を 1 行で検査する。
        let o = format!("{:?}", read(&layout, None));
        assert!(o.contains("Warning(") && o.contains("No such file"), "{o}");
        let data = root.join(".claude/tools/data");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("stage-graph.json"), "{}").unwrap();
        let o = format!("{:?}", read(&layout, None));
        assert!(
            o.contains("Warning(") && o.contains("expected a sequence"),
            "{o}"
        );
        // 記録が作業空間の下に無い配置は根拠を採れない。
        fs::write(
            data.join("stage-graph.json"),
            r#"[{"slug":"intent-capture","phase":"ideation"}]"#,
        )
        .unwrap();
        let foreign = Layout::for_record(
            &root.join("elsewhere"),
            &core_command_domain::workspace::SpaceName::parse("default").unwrap(),
            &core_command_domain::workspace::IntentDirName::parse("260101-basis-aaaaaaaa").unwrap(),
        );
        assert!(
            read(&foreign, Some("intent-capture")).is_none(),
            "状態が無ければ何も返さない"
        );
    }
}
