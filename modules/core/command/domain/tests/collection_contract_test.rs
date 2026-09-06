//! 一級コレクション型への横展開漏れと共通契約を検証する。
use core_command_domain::workflow_definition::{
    ExecutionKind, PhaseId, ScopeGrid, StageGraph, StageMode, StageNodeBuilder, StageNumber,
    StageSlug,
};
use core_command_domain::workspace::{
    AuditFieldKey, AuditFields, BoltRefs, Checkboxes, OrderedAuditEvents,
};
use core_infrastructure::collections::FirstClassCollection;
use std::collections::BTreeMap;

fn check<C: FirstClassCollection + PartialEq + std::fmt::Debug>(collection: &C, expected: usize)
where
    C::Filtered: PartialEq<C> + std::fmt::Debug,
{
    assert_eq!(FirstClassCollection::len(collection), expected);
    assert_eq!(FirstClassCollection::is_empty(collection), expected == 0);
    assert_eq!(
        FirstClassCollection::fold_left(collection, 0, |count, _| count + 1),
        expected
    );
    assert_eq!(
        FirstClassCollection::at(collection, 0).is_some(),
        expected != 0
    );
    assert!(FirstClassCollection::at(collection, expected).is_none());
    assert!(FirstClassCollection::at(collection, usize::MAX).is_none());
    assert_eq!(
        FirstClassCollection::filter(collection, |_| true),
        *collection
    );
    assert!(FirstClassCollection::is_empty(
        &FirstClassCollection::filter(collection, |_| false)
    ));
}

#[test]
fn all_domain_collection_types_share_the_traversal_contract() {
    check(&BoltRefs::parse("[b, a]").unwrap(), 2);
    check(&BoltRefs::empty(), 0);
    check(&Checkboxes::parse("- [x] a — EXECUTE\n- [ ] b — SKIP\n"), 2);
    check(&Checkboxes::parse(""), 0);
    check(
        &OrderedAuditEvents::find_in(
            "\n## Event\n**Timestamp**: 2026-09-06T00:00:00Z\n**Event**: HUMAN_TURN\n\n---\n",
        ),
        1,
    );
    check(&OrderedAuditEvents::find_in(""), 0);
    check(
        &AuditFields::new().with(AuditFieldKey::parse("Stage").unwrap(), "value"),
        1,
    );
    check(&AuditFields::new(), 0);
    let slug = StageSlug::parse("a").unwrap();
    let graph = StageGraph::new(vec![
        StageNodeBuilder::new(
            slug.clone(),
            StageNumber::parse("1.1").unwrap(),
            "A".to_string(),
            PhaseId::Inception,
            ExecutionKind::Always,
            StageMode::Inline,
        )
        .build(),
    ])
    .unwrap();
    check(&graph, 1);
    check(&StageGraph::new(vec![]).unwrap(), 0);
    check(
        &ScopeGrid::new(BTreeMap::from([(
            "poc".to_string(),
            BTreeMap::from([(
                slug,
                core_command_domain::workflow_definition::PlanAction::Execute,
            )]),
        )])),
        1,
    );
    check(&ScopeGrid::default(), 0);
}
