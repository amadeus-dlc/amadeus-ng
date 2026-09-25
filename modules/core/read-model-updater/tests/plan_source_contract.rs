//! 計画承認の参照文書（計画・単体テスト指示・質問票・状態ファイル・memory 層）を読む境界の契約。
//! 無いものは空として扱い、在るのに読めないものは拒否する。
// テストコードでは unwrap / expect / panic を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::needless_pass_by_value,
    clippy::type_complexity
)]
use core_command_domain::orchestration::PlanTarget;
use core_read_model_updater::orchestration::{PlanSource, ReadModelUpdateError, SteeringSource};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

struct Fixture {
    root: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().unwrap(),
        }
    }
    fn project(&self) -> PathBuf {
        self.root.path().join("project")
    }
    fn record(&self) -> PathBuf {
        self.project()
            .join("aidlc/spaces/default/intents/260907-selfhost")
    }
    fn memory(&self) -> PathBuf {
        self.project().join("aidlc/spaces/default/memory")
    }
    fn source(&self, target: PlanTarget) -> PlanSource {
        PlanSource::new(self.project(), self.record(), self.memory(), target)
    }
    fn code_generation(&self, unit: Option<&str>) -> PathBuf {
        let mut directory = self.record().join("construction");
        if let Some(unit) = unit {
            directory.push(unit);
        }
        directory.join("code-generation")
    }
}
fn steering_read(error: ReadModelUpdateError) -> (String, ErrorKind) {
    match error {
        ReadModelUpdateError::SteeringRead { path, kind } => (path, kind),
        other => panic!("SteeringRead を期待した: {other:?}"),
    }
}

#[test]
fn a_unit_target_reads_the_unit_directory_and_missing_documents_are_empty() {
    let fixture = Fixture::new();
    let directory = fixture.code_generation(Some("u2-workflow-authority"));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("code-generation-plan.md"), "# 計画\n").unwrap();
    std::fs::write(directory.join("code-generation-questions.md"), "Q1\n").unwrap();
    std::fs::create_dir_all(fixture.memory()).unwrap();
    std::fs::write(
        fixture.memory().join("team.md"),
        "# Team\n\n## Testing Posture\n\n- **Methodology**: tdd\n",
    )
    .unwrap();
    std::fs::write(fixture.record().join("aidlc-state.md"), "# state\n").unwrap();
    let input = fixture
        .source(PlanTarget::for_unit("u2-workflow-authority").unwrap())
        .read(Some("f".repeat(64)))
        .unwrap();
    assert_eq!(input.documents().plan(), "# 計画\n");
    assert_eq!(input.documents().instructions(), "", "無い文書は空");
    assert_eq!(input.documents().questions(), "Q1\n");
    assert_eq!(
        input.documents().questions_file(),
        "aidlc/spaces/default/intents/260907-selfhost/construction/u2-workflow-authority/code-generation/code-generation-questions.md"
    );
    assert_eq!(input.target().unit(), Some("u2-workflow-authority"));
    assert_eq!(
        input.state_sha256().map(str::to_string),
        Some(core_infrastructure::hash::sha256_hex(b"# state\n"))
    );
    assert_eq!(input.source_sha256(), Some("f".repeat(64).as_str()));
}

#[test]
fn a_missing_state_file_yields_no_state_digest() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(fixture.record()).unwrap();
    let input = fixture
        .source(PlanTarget::stage_level())
        .read(None)
        .unwrap();
    assert_eq!(input.state_sha256(), None);
    assert_eq!(input.documents().plan(), "");
}

#[test]
fn a_document_that_exists_but_cannot_be_read_is_refused_with_its_path() {
    let fixture = Fixture::new();
    let directory = fixture.code_generation(None);
    std::fs::create_dir_all(directory.join("code-generation-plan.md")).unwrap();
    let (path, kind) = steering_read(
        fixture
            .source(PlanTarget::stage_level())
            .read(None)
            .unwrap_err(),
    );
    assert_eq!(Path::new(&path), directory.join("code-generation-plan.md"));
    assert_ne!(kind, ErrorKind::NotFound);
}

#[test]
fn a_state_file_that_cannot_be_read_is_refused_with_its_path() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(fixture.record().join("aidlc-state.md")).unwrap();
    let (path, kind) = steering_read(
        fixture
            .source(PlanTarget::stage_level())
            .read(None)
            .unwrap_err(),
    );
    assert_eq!(Path::new(&path), fixture.record().join("aidlc-state.md"));
    assert_ne!(kind, ErrorKind::NotFound);
}

#[test]
fn a_record_outside_the_project_cannot_name_its_questions_file() {
    let fixture = Fixture::new();
    let elsewhere = fixture.root.path().join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    let source = PlanSource::new(
        fixture.project(),
        elsewhere.clone(),
        fixture.memory(),
        PlanTarget::stage_level(),
    );
    let (path, kind) = steering_read(source.read(None).unwrap_err());
    assert_eq!(
        Path::new(&path),
        elsewhere.join("construction/code-generation/code-generation-questions.md")
    );
    assert_eq!(kind, ErrorKind::InvalidData);
}

#[test]
fn an_unreadable_memory_file_stops_the_testing_sections() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(fixture.record()).unwrap();
    std::fs::create_dir_all(fixture.memory().join("project.md")).unwrap();
    let (path, _) = steering_read(
        fixture
            .source(PlanTarget::stage_level())
            .read(None)
            .unwrap_err(),
    );
    assert_eq!(Path::new(&path), fixture.memory().join("project.md"));
    let (path, _) = steering_read(
        SteeringSource::new(fixture.memory())
            .read_testing_sections()
            .unwrap_err(),
    );
    assert_eq!(Path::new(&path), fixture.memory().join("project.md"));
}

#[test]
fn memory_rules_are_named_relative_to_the_workspace_root_when_one_is_given() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(fixture.memory().join("phases")).unwrap();
    std::fs::write(fixture.memory().join("org.md"), "# Org\n\nWe ship.\n").unwrap();
    std::fs::write(
        fixture.memory().join("phases/construction.md"),
        "# Construction\n\nTest first.\n",
    )
    .unwrap();
    let rules = SteeringSource::new(fixture.memory())
        .relative_to(fixture.project())
        .read()
        .unwrap();
    let names: Vec<String> = rules
        .files_for(core_command_domain::workflow_definition::PhaseId::Construction)
        .iter()
        .map(|rule| rule.path().to_string())
        .collect();
    assert_eq!(
        names,
        vec![
            "aidlc/spaces/default/memory/org.md".to_string(),
            "aidlc/spaces/default/memory/phases/construction.md".to_string()
        ]
    );
}

/// 閉じない HTML コメントは、その位置から先を規則本文として数える（本家 `isSubstantiveRuleText`）。
#[test]
fn an_unterminated_comment_leaves_the_rest_of_the_text_as_rules() {
    assert!(SteeringSource::text_is_substantive(
        "# Team\n\n<!-- open comment that never closes\nALWAYS run the suite.\n"
    ));
    assert!(!SteeringSource::text_is_substantive(
        "# Team\n\n<!-- closed --> \n<!-- also closed -->\n---\n"
    ));
}

/// 監査シャードの綴りが `spaces/<space>/intents/<record>` の形でなければ、record 相対の
/// 監査対象は導かれず、成果物・セッション監査の行はそのシャードへ積まれない。
#[test]
fn projection_targets_outside_the_record_layout_carry_no_project_dir() {
    use core_read_model_updater::orchestration::ProjectionTargets;
    let odd = ProjectionTargets::new(
        "/tmp/x/aidlc/spaces/default/other/260907/aidlc-state.md",
        "/tmp/x/aidlc/spaces/default/other/260907/audit/host.md",
        "/tmp/x/aidlc/spaces/default/memory",
    );
    assert_eq!(odd.project_dir(), None, "intents でない親は記録ではない");
    let not_spaces = ProjectionTargets::new(
        "/tmp/x/aidlc/rooms/default/intents/260907/aidlc-state.md",
        "/tmp/x/aidlc/rooms/default/intents/260907/audit/host.md",
        "/tmp/x/aidlc/rooms/default/memory",
    );
    assert_eq!(
        not_spaces.project_dir(),
        None,
        "spaces でない祖父は記録ではない"
    );
    let not_aidlc = ProjectionTargets::new(
        "/tmp/x/other/spaces/default/intents/260907/aidlc-state.md",
        "/tmp/x/other/spaces/default/intents/260907/audit/host.md",
        "/tmp/x/other/spaces/default/memory",
    );
    assert_eq!(
        not_aidlc.project_dir(),
        None,
        "aidlc でない曾祖父は記録ではない"
    );
    let regular = ProjectionTargets::new(
        "/tmp/x/aidlc/spaces/default/intents/260907/aidlc-state.md",
        "/tmp/x/aidlc/spaces/default/intents/260907/audit/host.md",
        "/tmp/x/aidlc/spaces/default/memory",
    );
    assert_eq!(
        regular.project_dir(),
        Some(std::path::Path::new("/tmp/x")),
        "正規の配置からはワークスペース根が導かれる"
    );
}
