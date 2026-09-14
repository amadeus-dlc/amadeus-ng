use super::DomainIndex;

const USE_CASE: &str = "modules/core/command/use-case/src/execute.rs";
const MODEL: &str = r#"
pub struct Intent { definition_id: WorkflowDefinitionId, title: String }
pub struct WorkflowDefinitionId(String);
pub struct WorkflowDefinition;
impl Intent {
    pub fn definition_id(&self) -> &WorkflowDefinitionId { &self.definition_id }
    pub fn title(&self) -> &str { &self.title }
}
impl WorkflowDefinitionId { pub fn as_str(&self) -> &str { &self.0 } }
"#;
const PORT: &str = r#"
use core_command_domain::model::{WorkflowDefinition, WorkflowDefinitionId};
pub trait WorkflowDefinitionRepository {
    async fn find_by_id(&self, id: &WorkflowDefinitionId) -> Result<WorkflowDefinition, ()>;
}
"#;

fn findings(body: &str, port: &str) -> Vec<crate::check::Finding> {
    check(body, port, "R: Definitions")
}

fn check(body: &str, port: &str, struct_generic: &str) -> Vec<crate::check::Finding> {
    let source = format!(
        r#"
use core_command_domain::model::Intent;
use crate::port::WorkflowDefinitionRepository as Definitions;
struct Execute<{struct_generic}> {{ repository: R }}
impl<R: Definitions> Execute<R> {{
    async fn execute(&self, intent: &Intent) -> Result<(), ()> {{
        {body}
        Ok(())
    }}
}}
"#
    );
    let sources = vec![
        (
            "modules/core/command/domain/src/model.rs".into(),
            MODEL.into(),
        ),
        (
            "modules/core/command/use-case/src/port.rs".into(),
            port.into(),
        ),
        (USE_CASE.into(), source.clone()),
    ];
    DomainIndex::build(&sources).check(USE_CASE, &source)
}

#[test]
fn resolves_repository_bounds_declared_on_the_impl_instead_of_the_struct() {
    let body = "self.repository.find_by_id(intent.definition_id()).await?;";
    assert!(check(body, PORT, "R").is_empty());
}

#[test]
fn accepts_only_the_identifier_getter_passed_directly_to_a_repository_lookup() {
    let body = "self.repository.find_by_id(intent.definition_id()).await?;";
    assert!(findings(body, PORT).is_empty());
}

#[test]
fn accepts_borrow_clone_and_ufcs_without_inspecting_identifier_contents() {
    for body in [
        "self.repository.find_by_id(&(intent.definition_id().clone())).await?;",
        "self.repository.find_by_id(Intent::definition_id(intent)).await?;",
        "Definitions::find_by_id(&self.repository, intent.definition_id()).await?;",
        "R::find_by_id(&self.repository, Intent::definition_id(intent)).await?;",
        "<R as Definitions>::find_by_id(&self.repository, intent.definition_id()).await?;",
    ] {
        assert!(findings(body, PORT).is_empty(), "{body}");
    }
}

#[test]
fn rejects_identifier_getters_used_for_business_decisions_or_helpers() {
    for (body, expected) in [
        ("if intent.definition_id() == intent.definition_id() {}", 1),
        (
            "let id = intent.definition_id(); self.repository.find_by_id(id).await?;",
            1,
        ),
        ("consume(intent.definition_id());", 1),
        (
            "self.repository.find_by_id({ consume(intent.title()); intent.definition_id() }).await?;",
            2,
        ),
        (
            "self.repository.find_by_id(if intent.title() == \"a\" { intent.definition_id() } else { intent.definition_id() }).await?;",
            2,
        ),
        (
            "self.repository.find_by_id(intent.definition_id()).await?; consume(intent.title());",
            1,
        ),
    ] {
        assert_eq!(findings(body, PORT).len(), expected, "{body}");
    }
}

#[test]
fn rejects_other_data_and_homonymous_methods_even_in_lookup_arguments() {
    let id_call = "self.repository.find_by_id(intent.definition_id()).await?;";
    // 同名の通常型はRepositoryポートではない。
    let ordinary = PORT
        .replace(
            "pub trait WorkflowDefinitionRepository",
            "pub struct WorkflowDefinitionRepository; impl WorkflowDefinitionRepository",
        )
        .replace(
            "-> Result<WorkflowDefinition, ()>;",
            "-> Result<WorkflowDefinition, ()> { todo!() }",
        );
    assert_eq!(findings(id_call, &ordinary).len(), 1);
    let text_port = PORT.replace("id: &WorkflowDefinitionId", "id: &str");
    assert_eq!(
        findings(
            "self.repository.find_by_id(intent.title()).await?;",
            &text_port
        )
        .len(),
        1
    );
    assert_eq!(
        findings(
            "self.repository.find_by_id(intent.definition_id().as_str()).await?;",
            &text_port
        )
        .len(),
        2
    );
    assert_eq!(findings(id_call, &text_port).len(), 1);
}
