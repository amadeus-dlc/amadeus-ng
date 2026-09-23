//! run-stage 指示の文脈 — 入力の解決・会話へ載せる知識・実効のレビュー形（2.8.2 追従）。
//!
//! 読み取りモデルの行は定義とスコープグリッドだけで決まる部分を持つ。ここで扱うのは、
//! 2.8.2 が指示を出す瞬間にディスクと状態を見て決める部分である。
//!
//! - `consumes` を成果物の語彙名から**生産元ステージの置き場**へ解決し、在るものと無い
//!   必須のもの（`consumes_absent`）に分ける（`aidlc-orchestrate.ts` `resolveConsumes` /
//!   `resolveConsumePaths`）
//! - 会話に載せるペルソナへ、配布の知識とスペースの知識を足す（`inlineContextEntries`）
//! - レビュー階級を scope の `review_cap` と状態の `Review Override` で下げ、advisory なら
//!   往復を 1 回に固定する（`resolveReviewClass`）
//!
//! どれも読み取りだけで、ファイルを書かない。読めない材料は「無い」と同じに扱う — 2.8.2 も
//! 指示の発行を止めずに警告へ回す。
use crate::layout::Layout;
use core_query_use_case::orchestration::AbsentConsume;
use serde_json::Value;
use std::fs;
use std::path::Path;

/// 会話へ載せるパス列の上限（2.8.2 `INLINE_CONTEXT_PATHS_MAX_BYTES`）。
const INLINE_CONTEXT_PATHS_MAX_BYTES: usize = 8 * 1024;

/// 配布の知識のうち、Minimal の深さで刈り込みの対象になるもの（2.8.2
/// `SHIPPED_INLINE_KNOWLEDGE`）。ここに無いファイル（チームが足したもの）は刈り込まない。
const SHIPPED_INLINE_KNOWLEDGE: [(&str, &[&str]); 3] = [
    (
        "aidlc-shared",
        &[
            "ai-dlc-principles.md",
            "audit-format.md",
            "brownfield.md",
            "knowledge-readme-template.md",
            "memory-template.md",
            "rules-reading.md",
            "state-template.md",
            "verification.md",
            "worktree-info-schema.md",
        ],
    ),
    (
        "aidlc-product-agent",
        &[
            "functional-design-guide.md",
            "market-research-methods.md",
            "prioritization-frameworks.md",
            "product-guide.md",
            "requirements-elicitation.md",
            "requirements-guide.md",
            "user-story-patterns.md",
        ],
    ),
    (
        "aidlc-architect-agent",
        &[
            "adr-template.md",
            "architecture-guide.md",
            "architecture-patterns.md",
            "ddd-patterns.md",
            "nfr-design-guide.md",
            "nfr-design-patterns.md",
        ],
    ),
];

/// ステージ名と、そのステージが役ごとに残す配布の知識。
type MinimalSelection = (
    &'static str,
    &'static [(&'static str, &'static [&'static str])],
);

/// Minimal の深さで各ステージが残す配布の知識（2.8.2 `MINIMAL_INLINE_KNOWLEDGE`）。
const MINIMAL_INLINE_KNOWLEDGE: [MinimalSelection; 2] = [
    (
        "intent-capture",
        &[
            (
                "aidlc-shared",
                &[
                    "ai-dlc-principles.md",
                    "rules-reading.md",
                    "verification.md",
                ],
            ),
            (
                "aidlc-product-agent",
                &["requirements-elicitation.md", "requirements-guide.md"],
            ),
            ("aidlc-architect-agent", &["architecture-guide.md"]),
        ],
    ),
    (
        "requirements-analysis",
        &[
            (
                "aidlc-shared",
                &[
                    "ai-dlc-principles.md",
                    "brownfield.md",
                    "rules-reading.md",
                    "verification.md",
                ],
            ),
            (
                "aidlc-product-agent",
                &["requirements-elicitation.md", "requirements-guide.md"],
            ),
        ],
    ),
];

/// 実効のレビュー形。
pub(crate) struct ReviewShape {
    /// レビューの対象成果物（定義が名乗っていれば）。
    pub(crate) artifact: Option<String>,
    /// 実効の階級（`none` なら指示にレビュー欄を載せない）。
    pub(crate) class: String,
    /// 往復の上限（advisory は 1）。
    pub(crate) max_iterations: u32,
}

/// 実効のレビューの解決結果。
///
/// 「実効の階級が `none` なのでレビュー欄を省く」と「定義グラフが読めないので行の宣言へ
/// 戻る」を取り違えないために分ける — 取り違えると、`none` に下げたステージの指示へ
/// 行が宣言したレビュー欄が戻ってしまう（2.8.2 `aidlc-orchestrate.ts` は `none` なら
/// reviewer / review_artifact / review_class / 往復上限と `reviewer` の
/// `protocol_modules` をまとめて省く）。
pub(crate) enum ReviewResolution {
    /// 定義グラフが読めない、またはステージが載っていない — 行の宣言を使う。
    Unresolved,
    /// レビューを載せない（宣言が無い、または実効の階級が `none`）。
    Omitted,
    /// 実効のレビュー形。
    Present(ReviewShape),
}

/// 定義グラフ・スコープグリッド・状態ファイルの読み取り（指示 1 回ぶん）。
pub(crate) struct StageContext<'a> {
    layout: &'a Layout,
    graph: Vec<Value>,
    grid: Value,
    state: String,
}

impl<'a> StageContext<'a> {
    /// 指示を描く瞬間の材料を読む。読めないものは空として扱う。
    pub(crate) fn read(layout: &'a Layout) -> StageContext<'a> {
        let data = layout.definition_data_dir();
        let json = |name: &str| {
            fs::read(data.join(name))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        };
        let graph = json("stage-graph.json")
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default();
        let grid = json("scope-grid.json").unwrap_or(Value::Null);
        let state = layout
            .state_file()
            .and_then(|path| fs::read_to_string(path).ok())
            .unwrap_or_default();
        StageContext {
            layout,
            graph,
            grid,
            state,
        }
    }

    fn node(&self, slug: &str) -> Option<&Value> {
        self.graph
            .iter()
            .find(|node| node.get("slug").and_then(Value::as_str) == Some(slug))
    }

    /// 状態ファイルの `- **<name>**: <value>` 欄。
    fn field(&self, name: &str) -> Option<&str> {
        let prefix = format!("- **{name}**:");
        self.state
            .lines()
            .find_map(|line| line.strip_prefix(prefix.as_str()))
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    fn scope(&self) -> Option<&str> {
        self.field("Scope")
    }

    /// そのステージを実効計画が走らせるか（状態の EXECUTE/SKIP を優先し、無ければグリッド）。
    fn runs(&self, slug: &str) -> bool {
        let suffix = self.state.lines().find_map(|line| {
            let rest = line.strip_prefix("- [")?;
            let rest = rest.get(3..)?;
            let (name, action) = rest.split_once(" — ")?;
            (name.trim() == slug).then(|| action.trim().starts_with("EXECUTE"))
        });
        suffix.unwrap_or_else(|| {
            self.scope()
                .and_then(|scope| self.grid.get(scope))
                .and_then(|scope| scope.get("stages"))
                .and_then(|stages| stages.get(slug))
                .and_then(Value::as_str)
                == Some("EXECUTE")
        })
    }

    /// `consumes` を生産元の置き場へ解決し、在るものと無い必須のものに分ける。
    ///
    /// 在るものは `consumes`、無い必須のものは `consumes_absent`（生産元が実効計画に無ければ
    /// 想定内）、無い任意のものは入力ではないので落とす（2.8.2 の存在分割）。
    ///
    /// 定義グラフにそのステージが無ければ `None`（呼出側は読み取りモデルの行へ戻る）。
    pub(crate) fn consumes(
        &self,
        slug: &str,
        project_kind: Option<&str>,
        unit: Option<&str>,
    ) -> Option<(Vec<String>, Vec<AbsentConsume>)> {
        let mut present = Vec::new();
        let mut absent = Vec::new();
        let node = self.node(slug)?;
        for consume in node
            .get("consumes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(artifact) = consume.get("artifact").and_then(Value::as_str) else {
                continue;
            };
            if let (Some(condition), Some(kind)) = (
                consume.get("conditional_on").and_then(Value::as_str),
                project_kind,
            ) && condition != kind
            {
                continue;
            }
            let required = consume.get("required").and_then(Value::as_bool) == Some(true);
            let producer = self.producer_of(artifact);
            let path = self.artifact_path(artifact, producer.unwrap_or(node), unit);
            if self.layout.project_dir().join(&path).exists() {
                present.push(path);
            } else if required {
                let expected = producer
                    .and_then(|producer| producer.get("slug").and_then(Value::as_str))
                    .is_none_or(|producer| !self.runs(producer));
                absent.push(AbsentConsume::new(path, expected));
            }
        }
        Some((present, absent))
    }

    /// その成果物を生産する最初のステージ（2.8.2 `producersOf(name)[0]`）。
    fn producer_of(&self, artifact: &str) -> Option<&Value> {
        self.graph.iter().find(|node| {
            node.get("produces")
                .and_then(Value::as_array)
                .is_some_and(|produces| produces.iter().any(|p| p.as_str() == Some(artifact)))
        })
    }

    /// 成果物の置き場（ワークスペース相対）。reverse-engineering の出力は codekb、それ以外は
    /// 生産元ステージの記録の下である（Unit ごとのステージは Unit 名の下）。
    fn artifact_path(&self, artifact: &str, producer: &Value, unit: Option<&str>) -> String {
        let file = artifact_filename(artifact);
        let slug = producer.get("slug").and_then(Value::as_str).unwrap_or("");
        let phase = producer.get("phase").and_then(Value::as_str).unwrap_or("");
        if slug == "reverse-engineering" {
            let repo = self
                .layout
                .project_dir()
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("repo");
            return format!("aidlc/spaces/{}/codekb/{repo}/{file}", self.layout.space());
        }
        let record = self
            .layout
            .record_dir()
            .and_then(|dir| dir.strip_prefix(self.layout.project_dir()).ok())
            .map(posix)
            .unwrap_or_default();
        let per_unit = producer.get("for_each").and_then(Value::as_str) == Some("unit-of-work");
        match unit {
            Some(unit) if per_unit => format!("{record}/{phase}/{unit}/{slug}/{file}"),
            _ => format!("{record}/{phase}/{slug}/{file}"),
        }
    }

    /// 会話へ載せるパス（ペルソナ・配布の知識・スペースの知識）。
    ///
    /// `personas` は行が持つペルソナのパス（`.claude/agents/<agent>.md`）である。そこから
    /// 役の名前を取り、2.8.2 と同じ順で知識を足す — 共有の配布知識、役ごとの配布知識、
    /// スペースの共有知識、役ごとのスペース知識。同じパスは最初の 1 つだけを残し、全体を
    /// 8 KiB の JSON に収まるところで切る。
    ///
    /// 定義グラフにそのステージが無ければ、行のペルソナをそのまま返す。
    pub(crate) fn inline_context_paths(&self, slug: &str, personas: &[String]) -> Vec<String> {
        if self.node(slug).is_none() {
            return personas.to_vec();
        }
        let agents: Vec<String> = personas
            .iter()
            .filter_map(|path| {
                Path::new(path)
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(str::to_string)
            })
            .collect();
        if agents.is_empty() {
            return personas.to_vec();
        }
        let minimal = self
            .field("Depth")
            .is_some_and(|depth| depth.eq_ignore_ascii_case("minimal"));
        let project = self.layout.project_dir();
        let mut paths: Vec<String> = personas.to_vec();
        let shipped = |owner: &str| {
            let files = markdown_files(
                &project.join(".claude/knowledge").join(owner),
                &format!(".claude/knowledge/{owner}"),
            );
            prune_minimal(files, slug, owner, minimal)
        };
        paths.extend(shipped("aidlc-shared"));
        for agent in &agents {
            paths.extend(shipped(agent));
        }
        let space = format!("aidlc/spaces/{}/knowledge", self.layout.space());
        paths.extend(markdown_files(
            &project.join(&space).join("aidlc-shared"),
            &format!("{space}/aidlc-shared"),
        ));
        for agent in &agents {
            paths.extend(markdown_files(
                &project.join(&space).join(agent),
                &format!("{space}/{agent}"),
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        paths.retain(|path| seen.insert(path.clone()));
        let mut kept: Vec<String> = Vec::new();
        let mut bytes = 2; // `[` と `]`
        for path in paths {
            let added = path.len() + 2 + usize::from(!kept.is_empty());
            if bytes + added > INLINE_CONTEXT_PATHS_MAX_BYTES {
                break;
            }
            bytes += added;
            kept.push(path);
        }
        kept
    }

    /// 内容確認（Consolidated Summary Confirmation）を要するステージ（定義の
    /// `summary_confirmation` — 2.8.2 `checkSummaryConfirmationEvidence`）。
    pub(crate) fn summary_confirmation_stages(&self) -> Vec<String> {
        self.graph
            .iter()
            .filter(|node| {
                node.get("summary_confirmation")
                    .is_some_and(|v| !v.is_null())
            })
            .filter(|node| node.get("phase").and_then(Value::as_str) != Some("initialization"))
            .filter_map(|node| node.get("slug").and_then(Value::as_str).map(str::to_string))
            .collect()
    }

    /// 実効のレビュー形（2.8.2 `resolveReviewClass`）。
    pub(crate) fn review(&self, slug: &str) -> ReviewResolution {
        let Some(node) = self.node(slug) else {
            return ReviewResolution::Unresolved;
        };
        if node.get("reviewer").and_then(Value::as_str).is_none() {
            return ReviewResolution::Omitted;
        }
        let declared = node
            .get("review_class")
            .and_then(Value::as_str)
            .unwrap_or("adversarial");
        let Some(mut class) = rank(declared) else {
            return ReviewResolution::Unresolved;
        };
        if let Some(cap) = self
            .scope()
            .and_then(|scope| self.review_cap(scope))
            .and_then(|cap| rank(&cap))
        {
            class = class.min(cap);
        }
        if let Some(override_) = self.field("Review Override").and_then(rank) {
            class = class.min(override_);
        }
        let class = ["none", "advisory", "adversarial"]
            .get(class)
            .copied()
            .unwrap_or("none");
        if class == "none" {
            return ReviewResolution::Omitted;
        }
        let max_iterations = if class == "advisory" {
            1
        } else {
            node.get("reviewer_max_iterations")
                .and_then(Value::as_u64)
                .and_then(|n| u32::try_from(n).ok())
                .unwrap_or(2)
        };
        ReviewResolution::Present(ReviewShape {
            artifact: node
                .get("review_artifact")
                .and_then(Value::as_str)
                .map(str::to_string),
            class: class.to_string(),
            max_iterations,
        })
    }

    /// scope ファイルの `review_cap`（`.claude/scopes/aidlc-<scope>.md` の前付け）。
    fn review_cap(&self, scope: &str) -> Option<String> {
        let text = fs::read_to_string(
            self.layout
                .project_dir()
                .join(".claude/scopes")
                .join(format!("aidlc-{scope}.md")),
        )
        .ok()?;
        let front = text.strip_prefix("---\n")?.split("\n---").next()?;
        front
            .lines()
            .find_map(|line| line.strip_prefix("review_cap:"))
            .map(|value| value.trim().trim_matches('"').to_string())
    }
}

/// 階級の順位（低いほど弱い — 2.8.2 `REVIEW_RANK`）。
fn rank(class: &str) -> Option<usize> {
    match class {
        "none" => Some(0),
        "advisory" => Some(1),
        "adversarial" => Some(2),
        _ => None,
    }
}

/// Minimal の深さでは、配布の知識のうちそのステージが選んだものだけを残す。
fn prune_minimal(files: Vec<String>, slug: &str, owner: &str, minimal: bool) -> Vec<String> {
    if !minimal {
        return files;
    }
    let Some(selected) = MINIMAL_INLINE_KNOWLEDGE
        .iter()
        .find(|(stage, _)| *stage == slug)
        .and_then(|(_, owners)| owners.iter().find(|(name, _)| *name == owner))
        .map(|(_, names)| *names)
    else {
        return files;
    };
    let shipped = SHIPPED_INLINE_KNOWLEDGE
        .iter()
        .find(|(name, _)| *name == owner)
        .map_or(&[][..], |(_, names)| *names);
    files
        .into_iter()
        .filter(|path| {
            let name = path.rsplit('/').next().unwrap_or(path);
            !shipped.contains(&name) || selected.contains(&name)
        })
        .collect()
}

/// ディレクトリの下の Markdown を再帰的に集める（名前順は大小文字を無視 — 2.8.2 の
/// `localeCompare` に合わせる）。
fn markdown_files(dir: &Path, shown: &str) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
    entries.sort_by_key(|entry| {
        let name = entry.file_name().to_string_lossy().into_owned();
        (name.to_lowercase(), name)
    });
    let mut found = Vec::new();
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let shown = format!("{shown}/{name}");
        if path.is_dir() {
            found.extend(markdown_files(&path, &shown));
        } else if name.ends_with(".md") && path.is_file() {
            found.push(shown);
        }
    }
    found
}

/// 成果物の語彙名からファイル名への写像（2.8.2 `aidlc-artifact-vocabulary.ts`）。
fn artifact_filename(name: &str) -> String {
    if name.ends_with(".md") || name.ends_with(".json") {
        return name.to_string();
    }
    match name {
        "build-test-results" | "load-test-results" => "test-results.md".to_string(),
        "traceability" => "traceability.json".to_string(),
        _ => format!("{name}.md"),
    }
}

fn posix(path: &Path) -> String {
    path.components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::indexing_slicing)]
    use super::*;

    const RECORD: &str = "260904-demo-abcd1234";

    /// 2 ステージの合成定義（requirements-analysis が reverse-engineering と units-generation の
    /// 出力を読む）と bugfix の状態を持つワークスペース。
    fn workspace(state: &str) -> (tempfile::TempDir, Layout) {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let layout = Layout::resolve(root.path());
        layout.point_at(RECORD).expect("カーソル");
        let record = layout.intents_dir().join(RECORD);
        fs::create_dir_all(&record).expect("record");
        fs::write(record.join("aidlc-state.md"), state).expect("state");
        let data = root.path().join(".claude/tools/data");
        fs::create_dir_all(&data).expect("data");
        fs::write(
            data.join("stage-graph.json"),
            r#"[
              {"slug":"reverse-engineering","phase":"inception","produces":["business-overview"]},
              {"slug":"units-generation","phase":"inception","produces":["unit-of-work"]},
              {"slug":"requirements-analysis","phase":"inception","produces":["requirements"],
               "consumes":[{"artifact":"business-overview","required":false,"conditional_on":"brownfield"},
                           {"artifact":"unit-of-work","required":true},
                           {"artifact":"requirements","required":true},
                           {"artifact":"intent-statement","required":false}],
               "reviewer":"aidlc-product-lead-agent","review_class":"adversarial",
               "reviewer_max_iterations":3,"review_artifact":"requirements",
               "summary_confirmation":"required"},
              {"slug":"state-init","phase":"initialization","summary_confirmation":"required"}
            ]"#,
        )
        .expect("graph");
        fs::write(
            data.join("scope-grid.json"),
            r#"{"bugfix":{"stages":{"reverse-engineering":"EXECUTE","units-generation":"SKIP","requirements-analysis":"EXECUTE"}}}"#,
        )
        .expect("grid");
        let layout = Layout::resolve(root.path());
        (root, layout)
    }

    #[test]
    fn consumes_resolve_under_their_producer_and_split_by_presence() {
        let (root, layout) = workspace("- **Scope**: bugfix\n");
        let repo = root
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .expect("名前")
            .to_string();
        let codekb = format!("aidlc/spaces/default/codekb/{repo}/business-overview.md");
        fs::create_dir_all(root.path().join(&codekb).parent().expect("親")).expect("codekb");
        fs::write(root.path().join(&codekb), "# BO\n").expect("codekb");
        let context = StageContext::read(&layout);

        let (present, absent) = context
            .consumes("requirements-analysis", Some("brownfield"), None)
            .expect("定義にある");
        assert_eq!(present, vec![codekb]);
        let record = format!("aidlc/spaces/default/intents/{RECORD}");
        assert_eq!(
            absent,
            vec![
                // 生産元が実効計画に無い（SKIP）ので想定内。
                AbsentConsume::new(
                    format!("{record}/inception/units-generation/unit-of-work.md"),
                    true
                ),
                // 生産元が走る計画なのに無いので本物の欠落。
                AbsentConsume::new(
                    format!("{record}/inception/requirements-analysis/requirements.md"),
                    false
                ),
            ],
            "任意の欠落（intent-statement）は入力ではないので落ちる"
        );
        // Greenfield では brownfield 条件の入力を読まない。
        let (present, _) = context
            .consumes("requirements-analysis", Some("greenfield"), None)
            .expect("定義にある");
        assert!(present.is_empty());
        assert!(context.consumes("unknown", None, None).is_none());
    }

    #[test]
    fn the_review_shape_is_lowered_by_the_scope_cap_and_the_override() {
        let (root, layout) = workspace("- **Scope**: bugfix\n");
        let scopes = root.path().join(".claude/scopes");
        fs::create_dir_all(&scopes).expect("scopes");
        fs::write(
            scopes.join("aidlc-bugfix.md"),
            "---\nname: bugfix\nreview_cap: advisory\n---\n",
        )
        .expect("scope");
        let ReviewResolution::Present(shape) =
            StageContext::read(&layout).review("requirements-analysis")
        else {
            panic!("advisory へ下がる");
        };
        assert_eq!(shape.class, "advisory");
        assert_eq!(shape.max_iterations, 1, "advisory は 1 回に固定する");
        assert_eq!(shape.artifact.as_deref(), Some("requirements"));

        fs::write(scopes.join("aidlc-bugfix.md"), "---\nname: bugfix\n---\n").expect("scope");
        let ReviewResolution::Present(shape) =
            StageContext::read(&layout).review("requirements-analysis")
        else {
            panic!("宣言どおり");
        };
        assert_eq!(
            (shape.class.as_str(), shape.max_iterations),
            ("adversarial", 3)
        );

        let (_root, layout) = workspace("- **Scope**: bugfix\n- **Review Override**: none\n");
        assert!(
            matches!(
                StageContext::read(&layout).review("requirements-analysis"),
                ReviewResolution::Omitted
            ),
            "none に下がればレビュー欄を載せない"
        );
        assert!(
            matches!(
                StageContext::read(&layout).review("unknown"),
                ReviewResolution::Unresolved
            ),
            "載っていないステージは行の宣言へ戻す"
        );
    }

    #[test]
    fn inline_context_adds_knowledge_and_prunes_shipped_files_at_minimal_depth() {
        let (root, layout) = workspace("- **Scope**: bugfix\n- **Depth**: Minimal\n");
        let knowledge = root.path().join(".claude/knowledge");
        for (owner, name) in [
            ("aidlc-shared", "brownfield.md"),
            ("aidlc-shared", "audit-format.md"),
            ("aidlc-shared", "team-extra.md"),
            ("aidlc-product-agent", "requirements-guide.md"),
        ] {
            fs::create_dir_all(knowledge.join(owner)).expect("dir");
            fs::write(knowledge.join(owner).join(name), "# k\n").expect("file");
        }
        let space = root
            .path()
            .join("aidlc/spaces/default/knowledge/aidlc-shared/rules");
        fs::create_dir_all(&space).expect("space");
        fs::write(space.join("A.md"), "# a\n").expect("a");
        fs::write(space.join("b.md"), "# b\n").expect("b");

        let personas = vec![".claude/agents/aidlc-product-agent.md".to_string()];
        let paths =
            StageContext::read(&layout).inline_context_paths("requirements-analysis", &personas);
        assert_eq!(
            paths,
            vec![
                ".claude/agents/aidlc-product-agent.md",
                ".claude/knowledge/aidlc-shared/brownfield.md",
                ".claude/knowledge/aidlc-shared/team-extra.md",
                ".claude/knowledge/aidlc-product-agent/requirements-guide.md",
                "aidlc/spaces/default/knowledge/aidlc-shared/rules/A.md",
                "aidlc/spaces/default/knowledge/aidlc-shared/rules/b.md",
            ],
            "Minimal では配布の知識のうちステージが選んだものとチームの追加だけを残す"
        );
        assert_eq!(
            StageContext::read(&layout).inline_context_paths("unknown", &personas),
            personas,
            "定義に無いステージは行のペルソナのまま"
        );
    }

    #[test]
    fn summary_confirmation_stages_come_from_the_definition_outside_initialization() {
        let (_root, layout) = workspace("- **Scope**: bugfix\n");
        assert_eq!(
            StageContext::read(&layout).summary_confirmation_stages(),
            vec!["requirements-analysis".to_string()]
        );
    }

    #[test]
    fn artifact_filenames_follow_the_vocabulary() {
        assert_eq!(artifact_filename("build-test-results"), "test-results.md");
        assert_eq!(artifact_filename("traceability"), "traceability.json");
        assert_eq!(artifact_filename("x.json"), "x.json");
        assert_eq!(artifact_filename("requirements"), "requirements.md");
    }
}
