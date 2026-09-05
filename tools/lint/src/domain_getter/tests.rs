use super::DomainIndex;

const DOMAIN: &str = "modules/core/command/domain/src/model.rs";
const USE_CASE: &str = "modules/core/command/use-case/src/execute.rs";
const MODEL: &str = r#"
pub struct Aggregate { id: Identifier, title: String, ready: bool }
pub struct Identifier(String);
impl Aggregate {
    pub fn id(&self) -> &Identifier { &self.id }
    pub fn title(&self) -> &str { self.title.as_str() }
    pub fn is_ready(&self) -> bool { self.title.len() > 0 }
    pub fn decide(&self) -> bool { self.title.len() > 3 }
    pub fn change(&mut self) { self.ready = true; }
}
impl Identifier { pub fn as_str(&self) -> &str { &self.0 } }
"#;

fn findings(path: &str, source: &str) -> Vec<crate::check::Finding> {
    let sources = vec![
        (DOMAIN.to_string(), MODEL.to_string()),
        (path.to_string(), source.to_string()),
    ];
    DomainIndex::build(&sources).check(path, source)
}

#[test]
fn rejects_domain_getters_from_typed_arguments_and_references() {
    let result = findings(
        USE_CASE,
        r#"
use core_command_domain::model::Aggregate;
fn execute(aggregate: &Aggregate) {
    let alias = &aggregate;
    alias.id().as_str();
}
"#,
    );
    assert_eq!(result.len(), 2);
    assert!(result.iter().all(|f| f.rule == "use-case-domain-getter"));
}

#[test]
fn rejects_import_alias_and_ufcs() {
    let result = findings(
        USE_CASE,
        r#"
use core_command_domain::model::Aggregate as Entity;
fn execute(entity: &Entity) { Entity::id(entity); }
"#,
    );
    assert_eq!(result.len(), 1);
}

#[test]
fn rejects_new_getter_without_adding_its_name_to_linter() {
    assert_eq!(
        findings(
            USE_CASE,
            r#"
use core_command_domain::model::Aggregate;
fn execute(entity: &Aggregate) { entity.title(); }
"#
        )
        .len(),
        1
    );
}

#[test]
fn accepts_predicate_decision_and_command() {
    assert!(
        findings(
            USE_CASE,
            r#"
use core_command_domain::model::Aggregate;
fn execute(entity: &mut Aggregate) { entity.is_ready(); entity.decide(); entity.change(); }
"#
        )
        .is_empty()
    );
}

#[test]
fn accepts_identical_calls_in_adapters_and_tests() {
    let source = r#"
use core_command_domain::model::Aggregate;
fn execute(entity: &Aggregate) { entity.id().as_str(); }
"#;
    for path in [
        "modules/core/command/interface-adapter/src/execute.rs",
        "modules/core/query/interface-adapter/src/execute.rs",
        "modules/core/command/use-case/tests/getters.rs",
    ] {
        assert!(findings(path, source).is_empty(), "{path}");
    }
    assert!(findings(USE_CASE, &format!("#[cfg(test)] mod tests {{ {source} }}")).is_empty());
}

#[test]
fn accepts_non_domain_homonyms_and_shadowed_local() {
    assert!(
        findings(
            USE_CASE,
            r#"
use core_command_domain::model::Aggregate;
struct View;
impl View { fn id(&self) -> &str { "id" } }
fn execute(entity: &Aggregate, view: &View, text: String, option: Option<Aggregate>) {
    let entity = view;
    entity.id(); view.id(); text.as_str(); option.as_ref();
}
"#
        )
        .is_empty()
    );
}

#[test]
fn rejects_repository_result_after_await_try_and_local_alias() {
    let sources = vec![
        (DOMAIN.to_string(), MODEL.to_string()),
        (
            "modules/core/command/use-case/src/port.rs".into(),
            r#"
use core_command_domain::model::Aggregate;
pub trait AggregateRepository { async fn find(&self) -> Result<Aggregate, ()>; }
"#
            .into(),
        ),
        (
            USE_CASE.into(),
            r#"
use crate::port::AggregateRepository;
struct Execute<R: AggregateRepository> { repository: R }
impl<R: AggregateRepository> Execute<R> {
    async fn execute(&self) -> Result<(), ()> {
        let aggregate = self.repository.find().await?;
        let alias = aggregate;
        alias.id();
        Ok(())
    }
}
"#
            .into(),
        ),
    ];
    let index = DomainIndex::build(&sources);
    assert_eq!(index.check(USE_CASE, &sources[2].1).len(), 1);
}

#[test]
fn rejects_direct_boolean_field_even_with_predicate_name() {
    let domain = MODEL.replace("self.title.len() > 0", "self.ready");
    let source =
        "use core_command_domain::model::Aggregate; fn execute(a: Aggregate) { a.is_ready(); }";
    let sources = vec![(DOMAIN.into(), domain), (USE_CASE.into(), source.into())];
    assert_eq!(
        DomainIndex::build(&sources).check(USE_CASE, source).len(),
        1
    );
}

#[test]
fn resolves_reexports_module_alias_and_type_alias() {
    let source = r#"
use core_command_domain::facade as model;
type Entity = model::Aggregate;
fn execute(a: &Entity) { <Entity>::id(a); }
"#;
    let sources = vec![
        (DOMAIN.into(), MODEL.into()),
        (
            "modules/core/command/domain/src/facade.rs".into(),
            "pub use crate::model::Aggregate;".into(),
        ),
        (USE_CASE.into(), source.into()),
    ];
    assert_eq!(
        DomainIndex::build(&sources).check(USE_CASE, source).len(),
        1
    );
}

#[test]
fn accepts_same_type_name_from_non_domain_module_and_unknown_receivers() {
    let source = r#"
use crate::view::Aggregate;
fn execute(a: &Aggregate, unknown: &External) { a.id(); unknown.id(); }
"#;
    let sources = vec![
        (DOMAIN.into(), MODEL.into()),
        (
            "modules/core/command/use-case/src/view.rs".into(),
            "pub struct Aggregate; impl Aggregate { fn id(&self) -> &str { \"id\" } }".into(),
        ),
        (USE_CASE.into(), source.into()),
    ];
    assert!(
        DomainIndex::build(&sources)
            .check(USE_CASE, source)
            .is_empty()
    );
}

#[test]
fn external_test_module_exemption_depends_on_cfg_not_filename() {
    let path = "modules/core/command/use-case/src/test_support.rs";
    let source = "use core_command_domain::model::Aggregate; fn execute(a: Aggregate) { a.id(); }";
    for (declaration, expected) in [
        ("#[cfg(test)] mod test_support;", 0),
        ("mod test_support;", 1),
    ] {
        let sources = vec![
            (DOMAIN.into(), MODEL.into()),
            (
                "modules/core/command/use-case/src/lib.rs".into(),
                declaration.into(),
            ),
            (path.into(), source.into()),
        ];
        assert_eq!(
            DomainIndex::build(&sources).check(path, source).len(),
            expected
        );
    }
}

#[test]
fn applies_to_query_use_case_and_keeps_non_test_cfg() {
    let path = "modules/core/query/use-case/src/execute.rs";
    let source = "use core_command_domain::model::Aggregate; #[cfg(not(test))] fn execute(a: Aggregate) { a.id(); }";
    assert_eq!(findings(path, source).len(), 1);
}

#[test]
fn reasoned_allow_suppresses_only_the_call_on_next_line() {
    let source = r#"
use core_command_domain::model::Aggregate;
fn execute(a: Aggregate) {
    // amadeus-lint: allow(use-case-domain-getter) — 移行中の明示的な例外
    a.id();
    // amadeus-lint: allow(use-case-domain-getter)
    a.id();
}
"#;
    let result = findings(USE_CASE, source);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].line, 7);
}

#[test]
fn rejects_delegated_child_getters_and_collection_projection() {
    let domain = format!(
        "{MODEL}\n pub struct Parent {{ child: Aggregate, children: Vec<Aggregate>, cursor: usize }} impl Parent {{ fn title(&self) -> &str {{ self.child.title() }} fn cursor_id(&self) -> Option<&Identifier> {{ self.children.get(self.cursor).map(Aggregate::id) }} }}"
    );
    let source = "use core_command_domain::model::Parent; fn execute(p: &Parent) { p.title(); p.cursor_id(); }";
    let sources = vec![(DOMAIN.into(), domain), (USE_CASE.into(), source.into())];
    assert_eq!(
        DomainIndex::build(&sources).check(USE_CASE, source).len(),
        2
    );
}

#[test]
fn clone_preserves_receiver_identity() {
    assert_eq!(findings(USE_CASE, "use core_command_domain::model::Aggregate; fn execute(a: Aggregate) { let b = a.clone(); b.id(); }").len(), 1);
}

#[test]
fn if_let_shadow_does_not_inherit_domain_type() {
    let source = r#"
use core_command_domain::model::Aggregate;
struct View;
impl View { fn id(&self) -> &str { "view" } }
fn execute(a: Aggregate, view: Option<View>) {
    if let Some(a) = view { a.id(); }
    a.id();
}
"#;
    assert_eq!(findings(USE_CASE, source).len(), 1);
}

#[test]
fn follows_fallible_associated_constructor_tuple_return() {
    let domain =
        format!("{MODEL} impl Aggregate {{ fn create() -> Result<(Self, ()), ()> {{ todo!() }} }}");
    let source = "use core_command_domain::model::Aggregate; fn execute() -> Result<(), ()> { let (a, event) = Aggregate::create()?; a.id(); Ok(()) }";
    let sources = vec![(DOMAIN.into(), domain), (USE_CASE.into(), source.into())];
    assert_eq!(
        DomainIndex::build(&sources).check(USE_CASE, source).len(),
        1
    );
}

#[test]
fn rejects_getter_with_local_binding() {
    let domain = MODEL.replace("&self.id", "let id = &self.id; id");
    let source = "use core_command_domain::model::Aggregate; fn execute(a: &Aggregate) { a.id(); }";
    let sources = vec![(DOMAIN.into(), domain), (USE_CASE.into(), source.into())];
    assert_eq!(
        DomainIndex::build(&sources).check(USE_CASE, source).len(),
        1
    );
}

#[test]
fn rejects_enum_projection_getter() {
    let domain = format!(
        "{MODEL} pub enum Event {{ First(Aggregate), Second(Aggregate) }} impl Event {{ fn id(&self) -> &Identifier {{ match self {{ Self::First(a) => a.id(), Self::Second(a) => a.id() }} }} }}"
    );
    let source = "use core_command_domain::model::Event; fn execute(e: &Event) { e.id(); }";
    let sources = vec![(DOMAIN.into(), domain), (USE_CASE.into(), source.into())];
    assert_eq!(
        DomainIndex::build(&sources).check(USE_CASE, source).len(),
        1
    );
}

#[test]
fn rejects_iterator_closure_getter_without_flagging_view_closure() {
    let source = r#"
use core_command_domain::model::Aggregate;
struct View;
impl View { fn id(&self) -> &str { "view" } }
fn execute(items: Vec<Aggregate>, views: Vec<View>) {
    items.iter().map(|item| item.id());
    views.iter().map(|item| item.id());
}
"#;
    assert_eq!(findings(USE_CASE, source).len(), 1);
}

#[test]
fn rejects_getters_in_trait_default_method_implementation() {
    let source = r#"
use core_command_domain::model::Aggregate;
trait Execute { fn helper(&self, a: &Aggregate) { a.id(); } }
"#;
    assert_eq!(findings(USE_CASE, source).len(), 1);
}

#[test]
fn generic_parameter_shadows_import_even_when_bound_is_external() {
    let source = r#"
use core_command_domain::model::Aggregate;
fn execute<Aggregate: external::HasId>(a: &Aggregate) { a.id(); }
"#;
    assert!(findings(USE_CASE, source).is_empty());
}

#[test]
fn external_test_modules_follow_rust_module_paths_and_include_descendants() {
    let source = "use core_command_domain::model::Aggregate; fn execute(a: Aggregate) { a.id(); }";
    let parent = "modules/core/command/use-case/src/execute.rs";
    let nested = "modules/core/command/use-case/src/execute/helper.rs";
    let descendant = "modules/core/command/use-case/src/execute/helper/child.rs";
    let sibling = "modules/core/command/use-case/src/helper.rs";
    let sources = vec![
        (DOMAIN.into(), MODEL.into()),
        (parent.into(), "#[cfg(test)] mod helper;".into()),
        (nested.into(), format!("mod child; {source}")),
        (descendant.into(), source.into()),
        (sibling.into(), source.into()),
    ];
    let index = DomainIndex::build(&sources);
    assert_eq!(
        index.check(sibling, source).len(),
        1,
        "本番の同名兄弟は除外しない"
    );
    assert!(index.check(nested, source).is_empty());
    assert!(index.check(descendant, source).is_empty());
}

#[test]
fn tracks_if_let_and_for_bindings_without_leaking_them_into_else() {
    let source = r#"
use core_command_domain::model::Aggregate;
struct View;
impl View { fn id(&self) -> &str { "view" } }
fn execute(maybe: Option<Aggregate>, items: Vec<Aggregate>, entity: View) {
    if let Some(entity) = maybe { entity.id(); } else { entity.id(); }
    for entity in items { entity.id(); }
    entity.id();
}
"#;
    assert_eq!(findings(USE_CASE, source).len(), 2);
}
