//! 一級コレクション型への横展開漏れと共通契約を検証する。
use core_command_domain::orchestration::{
    ArtifactPaths, ReviewClosure, ReviewClosures, ReviewVerdict, StageDisplay, StageEntries,
    StageEntry, StageIndexSet, StageSlots, StageSlugSet, TransitionStep, TransitionSteps,
};
use core_command_domain::workflow_definition::{
    ExecutionKind, PhaseId, PlanAction, ScopeGrid, StageGraph, StageMode, StageNodeBuilder,
    StageNumber, StageSlug,
};
use core_command_domain::workspace::{
    AuditFieldKey, AuditFields, BoltRefs, Checkboxes, OrderedAuditEvents, PromotedSection,
    PromotedSections, RuleLines,
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
            BTreeMap::from([(slug, PlanAction::Execute)]),
        )])),
        1,
    );
    check(&ScopeGrid::default(), 0);
}

#[test]
fn the_orchestration_and_workspace_collections_share_the_traversal_contract() {
    // clippy の unwrap 許可はテスト本体に閉じるので、合成は関数内のクロージャで組む。
    let entry = |name: &str, phase: PhaseId, plan_action: PlanAction| {
        StageEntry::new(
            StageSlug::parse(name).unwrap(),
            phase,
            plan_action,
            false,
            StageDisplay::new(StageNumber::parse("0.1").unwrap(), "Stage", "orchestrator").unwrap(),
        )
    };
    // 2 ステージの合成計画（先頭は initialization = EXECUTE かつ無条件）。
    let plan = StageEntries::new(vec![
        entry("state-init", PhaseId::Initialization, PlanAction::Execute),
        entry("intent-capture", PhaseId::Ideation, PlanAction::Execute),
    ])
    .unwrap();
    check(&plan, 2);
    check(&StageSlots::genesis(&plan), 2);

    // StageIndex の公開構築経路は計画の位置解決である（`StageIndex::new` はクレート内）。
    let position = plan
        .position_of(&StageSlug::parse("intent-capture").unwrap())
        .unwrap();
    check(&StageIndexSet::singleton(position), 1);
    check(&StageIndexSet::empty(), 0);
    check(&plan.slugs_at(&StageIndexSet::singleton(position)), 1);
    check(&StageSlugSet::empty(), 0);

    check(&ArtifactPaths::new(vec!["requirements.md".to_string()]), 1);
    check(&ArtifactPaths::empty(), 0);

    check(
        &TransitionSteps::new(vec![
            TransitionStep::GateStartRecovered,
            TransitionStep::Approve,
        ])
        .unwrap(),
        2,
    );
    check(&TransitionSteps::new(Vec::new()).unwrap(), 0);

    check(
        &ReviewClosures::new(vec![ReviewClosure::new(1, ReviewVerdict::Ready)]),
        1,
    );
    check(&ReviewClosures::empty(), 0);

    check(
        &PromotedSections::new(vec![PromotedSection::new(
            "Way of Working",
            "trunk-based.\n",
        )])
        .unwrap(),
        1,
    );
    check(&PromotedSections::new(Vec::new()).unwrap(), 0);

    check(
        &RuleLines::new(vec!["ALWAYS write tests first".to_string()]),
        1,
    );
    check(&RuleLines::empty(), 0);
}
