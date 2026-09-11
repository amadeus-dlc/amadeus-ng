//! 固定本家の埋込テスト契約の受理/拒否。
#![allow(clippy::unwrap_used)]
use core_command_domain::orchestration::EmbeddedTestingContract;
use core_infrastructure::canon_json::{SerializationProfile, serialize};
#[test]
fn embedded_contract_matches_the_saved_upstream_observations() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/golden/selfhost-stage1/testing-contract.json"
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
    let cases = corpus.get("observations").unwrap().as_array().unwrap();
    assert!(!cases.is_empty());
    let mut identifiers = std::collections::BTreeSet::new();
    for case in cases {
        let id = case.get("id").unwrap().as_str().unwrap();
        assert!(identifiers.insert(id));
        let parsed = EmbeddedTestingContract::parse(case.get("plan").unwrap().as_str().unwrap());
        let actual = parsed.as_ref().map_or(serde_json::Value::Null, |parsed| {
            serde_json::from_str(&serialize(
                parsed.value(),
                SerializationProfile::ContractCompact,
            ))
            .unwrap()
        });
        assert_eq!(&actual, case.get("parsed").unwrap(), "{id}");
    }
}

#[test]
fn a_valid_self_hash_does_not_replace_the_current_rule_contract() {
    use core_command_domain::orchestration::{TestingContext, TestingPosture, TestingSections};
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/golden/selfhost-stage1/testing-contract.json"
    ))
    .unwrap();
    let plan = corpus
        .get("observations")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case.get("id").and_then(serde_json::Value::as_str) == Some("valid"))
        .unwrap()
        .get("plan")
        .unwrap()
        .as_str()
        .unwrap();
    let embedded = EmbeddedTestingContract::parse(plan).unwrap();
    let context = TestingContext::new(
        "bugfix".to_string(),
        "minimal".to_string(),
        "brownfield".to_string(),
    );
    let current = TestingPosture::resolve(
        &TestingSections::new(String::new(), String::new(), String::new()),
        &context,
    )
    .unwrap();
    assert!(embedded.is_current(&current));
    let changed = TestingPosture::resolve(
        &TestingSections::new(
            String::new(),
            "- Methodology: tdd".to_string(),
            String::new(),
        ),
        &context,
    )
    .unwrap();
    assert!(
        !embedded.is_current(&changed),
        "計画の自己ハッシュが正しくても、現在の規則が変われば拒否する"
    );
}
