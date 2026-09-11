//! 通常stage-levelレビューの内容結合を公開CLIで検証する。
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};
struct Workspace {
    temp: tempfile::TempDir,
}
impl Workspace {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        let data = root.join(".claude/tools/data");
        fs::create_dir_all(&data).unwrap();
        let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        for name in ["stage-graph.json", "scope-grid.json", "harness.json"] {
            fs::copy(
                repository
                    .join("tests/golden/upstream-a277af21/data")
                    .join(name),
                data.join(name),
            )
            .unwrap();
        }
        fs::create_dir_all(root.join(".claude/scopes")).unwrap();
        fs::copy(
            repository.join(".claude/scopes/aidlc-bugfix.md"),
            root.join(".claude/scopes/aidlc-bugfix.md"),
        )
        .unwrap();
        for tool in ["aidlc-utility", "aidlc-log"] {
            tool_link::link_tool(&temp.path().join(tool)).unwrap();
        }
        fs::create_dir_all(root.join("aidlc/spaces/default/intents")).unwrap();
        fs::write(root.join("source.rs"), "fn main() {}\n").unwrap();
        let workspace = Self { temp };
        let result = workspace.run(
            "aidlc-utility",
            &[
                "intent-create",
                "--scope",
                "bugfix",
                "--label",
                "review",
                "--arguments",
                "Fix review receipt",
            ],
        );
        assert!(result.status.success(), "{result:?}");
        workspace
    }
    fn root(&self) -> PathBuf {
        self.temp.path().join("workspace")
    }
    fn run(&self, tool: &str, args: &[&str]) -> Output {
        Command::new(self.temp.path().join(tool))
            .args(args)
            .current_dir(self.root())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.temp.path().join("home"))
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap()
    }
}
#[test]
fn a_review_cannot_start_without_the_declared_documents() {
    let workspace = Workspace::new();
    let output = workspace.run(
        "aidlc-log",
        &[
            "review",
            "--stage",
            "requirements-analysis",
            "--reviewer",
            "aidlc-product-lead-agent",
            "--iteration",
            "1",
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{output:?}");
}

impl Workspace {
    fn record(&self) -> PathBuf {
        let intents = self.root().join("aidlc/spaces/default/intents");
        intents.join(
            fs::read_to_string(intents.join("active-intent"))
                .unwrap()
                .trim(),
        )
    }
    fn audit(&self) -> String {
        let mut paths = fs::read_dir(self.record().join("audit"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        paths.sort();
        paths
            .into_iter()
            .map(|path| fs::read_to_string(path).unwrap())
            .collect()
    }
    fn requirements(&self, completed: bool) {
        use base64::Engine as _;
        let corpus: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/golden/upstream-a277af21/stage1/cases.json"
        ))
        .unwrap();
        let id = if completed {
            "review/completed"
        } else {
            "review/request"
        };
        let case = corpus
            .get("observations")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c.get("id").and_then(serde_json::Value::as_str) == Some(id))
            .unwrap();
        for (path, encoded) in case.get("initial_files").unwrap().as_object().unwrap() {
            if let Some((_, relative)) = path.split_once("/inception/requirements-analysis/") {
                let destination = self
                    .record()
                    .join("inception/requirements-analysis")
                    .join(relative);
                fs::create_dir_all(destination.parent().unwrap()).unwrap();
                fs::write(
                    destination,
                    base64::engine::general_purpose::STANDARD
                        .decode(encoded.as_str().unwrap())
                        .unwrap(),
                )
                .unwrap();
            }
        }
    }
    fn review(&self, stage: &str, completed: bool) -> Output {
        let reviewer = if stage == "code-generation" {
            "aidlc-architecture-reviewer-agent"
        } else {
            "aidlc-product-lead-agent"
        };
        let mut args = vec![
            "review",
            "--stage",
            stage,
            "--reviewer",
            reviewer,
            "--iteration",
            "1",
        ];
        if completed {
            args.extend(["--verdict", "READY"]);
        }
        self.run("aidlc-log", &args)
    }
}
#[test]
fn fixed_upstream_request_and_completion_bind_the_same_document_bytes() {
    let workspace = Workspace::new();
    workspace.requirements(false);
    let requested = workspace.review("requirements-analysis", false);
    assert!(requested.status.success(), "{requested:?}");
    let audit = workspace.audit();
    assert!(audit.contains("**Artifact Fingerprint**: sha256:6d7abc84613dde3fdd397bb5324e9aff18f22b868a057a81434c883f80d14211"),"{audit}");
    assert!(audit.contains("**Review Appendix Offset**: 163"));
    workspace.requirements(true);
    let completed = workspace.review("requirements-analysis", true);
    assert!(completed.status.success(), "{completed:?}");
    let audit = workspace.audit();
    assert!(audit.contains("**Request Fingerprint**: sha256:6d7abc84613dde3fdd397bb5324e9aff18f22b868a057a81434c883f80d14211"));
    assert!(audit.contains("**Artifact Fingerprint**: sha256:5fd9c73ce23a4726762079bc915a5c544a249f431a4a49f448435fb9f282223d"));
}
#[test]
fn a_changed_document_is_refused_without_consuming_the_pending_review() {
    let workspace = Workspace::new();
    workspace.requirements(false);
    assert!(
        workspace
            .review("requirements-analysis", false)
            .status
            .success()
    );
    workspace.requirements(true);
    let path = workspace
        .record()
        .join("inception/requirements-analysis/requirements.md");
    let approved = fs::read_to_string(&path).unwrap();
    fs::write(&path, approved.replace("FR1:", "FR2:")).unwrap();
    let refused = workspace.review("requirements-analysis", true);
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    assert!(!workspace.audit().contains("**Event**: REVIEW_COMPLETED"));
    fs::write(path, approved).unwrap();
    assert!(
        workspace
            .review("requirements-analysis", true)
            .status
            .success()
    );
}

#[test]
fn code_generation_completion_requires_the_requested_source_and_a_matching_appendix() {
    let workspace = Workspace::new();
    let dir = workspace.record().join("construction/code-generation");
    fs::create_dir_all(&dir).unwrap();
    for (name, body) in [
        ("code-generation-plan.md", "# Plan\n"),
        ("unit-test-instructions.md", "# Test\n"),
        ("code-summary.md", "# Code\n"),
        ("traceability.json", "{}\n"),
    ] {
        fs::write(dir.join(name), body).unwrap();
    }
    let request = workspace.review("code-generation", false);
    assert!(request.status.success(), "{request:?}");
    fs::write(dir.join("code-generation-plan.md"),"# Plan\n\n## Review\n\n**Reviewer:** aidlc-architecture-reviewer-agent\n**Verdict:** READY\n**Iteration:** 1\n").unwrap();
    fs::write(
        workspace.root().join("source.rs"),
        "fn main() { changed(); }\n",
    )
    .unwrap();
    let rejected = workspace.review("code-generation", true);
    assert_eq!(rejected.status.code(), Some(1), "{rejected:?}");
    assert!(
        String::from_utf8(rejected.stderr)
            .unwrap()
            .contains("workspace source changed")
    );
    fs::write(workspace.root().join("source.rs"), "fn main() {}\n").unwrap();
    let completed = workspace.review("code-generation", true);
    assert!(completed.status.success(), "{completed:?}");
    let audit = workspace.audit();
    assert!(audit.contains("**Request Source Fingerprint**:"));
}

#[test]
fn an_absent_or_inconsistent_review_appendix_cannot_complete_the_request() {
    let workspace = Workspace::new();
    workspace.requirements(false);
    assert!(
        workspace
            .review("requirements-analysis", false)
            .status
            .success()
    );
    let absent = workspace.review("requirements-analysis", true);
    assert_eq!(absent.status.code(), Some(1));
    workspace.requirements(true);
    let path = workspace
        .record()
        .join("inception/requirements-analysis/requirements.md");
    let proper = fs::read_to_string(&path).unwrap();
    for invalid in [
        proper.replace(
            "**Reviewer:** aidlc-product-lead-agent",
            "**Reviewer:** someone-else",
        ),
        proper.replace("**Iteration:** 1", "**Iteration:** 2"),
        proper.replace("**Verdict:** READY", "**Verdict:** NOT-READY"),
    ] {
        fs::write(&path, invalid).unwrap();
        assert_eq!(
            workspace
                .review("requirements-analysis", true)
                .status
                .code(),
            Some(1)
        );
    }
    fs::write(path, proper).unwrap();
    assert!(
        workspace
            .review("requirements-analysis", true)
            .status
            .success()
    );
}

#[test]
fn a_pending_review_has_exactly_one_retry() {
    let workspace = Workspace::new();
    workspace.requirements(false);
    assert!(
        workspace
            .review("requirements-analysis", false)
            .status
            .success()
    );
    let args = [
        "review",
        "--stage",
        "requirements-analysis",
        "--reviewer",
        "aidlc-product-lead-agent",
        "--iteration",
        "1",
        "--retry-pending",
    ];
    let first = workspace.run("aidlc-log", &args);
    assert!(first.status.success(), "{first:?}");
    let second = workspace.run("aidlc-log", &args);
    assert_eq!(second.status.code(), Some(1), "{second:?}");
    let error = String::from_utf8(second.stderr).unwrap();
    assert!(
        error.contains("already used its one pending-request retry"),
        "{error}"
    );
    assert_eq!(
        workspace
            .audit()
            .matches("**Event**: REVIEW_REQUESTED\n")
            .count(),
        2
    );
    workspace.requirements(true);
    assert!(
        workspace
            .review("requirements-analysis", true)
            .status
            .success()
    );
}

#[test]
fn request_and_bounded_retry_match_the_fixed_upstream_audit_and_output() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/review-receipts.json"
    ))
    .unwrap();
    assert_eq!(
        corpus
            .get("source")
            .unwrap()
            .get("commit")
            .unwrap()
            .as_str(),
        Some("a277af218f0df7f325d3b8be7b6d90fce2c5bd40")
    );
    let workspace = Workspace::new();
    workspace.requirements(false);
    let normalize = |text: &str| {
        text.lines()
            .map(|line| {
                if line.starts_with("**Timestamp**: ") {
                    "**Timestamp**: <TS>"
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + if text.ends_with('\n') { "\n" } else { "" }
    };
    for case in corpus.get("observations").unwrap().as_array().unwrap() {
        let args = case
            .get("args")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|arg| arg.as_str().unwrap())
            .collect::<Vec<_>>();
        let before = workspace.audit();
        let state = fs::read(workspace.record().join("aidlc-state.md")).unwrap();
        let actual = workspace.run("aidlc-log", &args);
        assert_eq!(
            actual.status.code(),
            case.get("exit")
                .unwrap()
                .as_i64()
                .map(|n| i32::try_from(n).unwrap())
        );
        assert_eq!(
            String::from_utf8(actual.stdout).unwrap(),
            case.get("stdout").unwrap().as_str().unwrap()
        );
        assert_eq!(
            String::from_utf8(actual.stderr).unwrap(),
            case.get("stderr").unwrap().as_str().unwrap()
        );
        assert_eq!(
            normalize(workspace.audit().strip_prefix(&before).unwrap()),
            case.get("audit").unwrap().as_str().unwrap()
        );
        assert_eq!(
            fs::read(workspace.record().join("aidlc-state.md")).unwrap() != state,
            case.get("state_changed").unwrap().as_bool().unwrap()
        );
    }
}

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;

#[path = "../../../../tests/support/tool_link.rs"]
mod tool_link;
