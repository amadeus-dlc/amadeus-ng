//! 固定本家の計画承認節の保存済み観測。
#![allow(clippy::unwrap_used)]
use core_command_domain::orchestration::PlanQuestions;
#[test]
fn plan_questions_match_the_saved_upstream_observations() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/golden/selfhost-stage1/plan-questions.json"
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
    let observations = corpus.get("observations").unwrap().as_array().unwrap();
    assert!(!observations.is_empty());
    let mut identifiers = std::collections::BTreeSet::new();
    for case in observations {
        let id = case.get("id").unwrap().as_str().unwrap();
        assert!(identifiers.insert(id));
        let value = PlanQuestions::parse(case.get("content").unwrap().as_str().unwrap());
        assert_eq!(
            value.approved(),
            case.get("approved").unwrap().as_bool().unwrap(),
            "{id}"
        );
        assert_eq!(
            value.pending(),
            case.get("pending").unwrap().as_bool().unwrap(),
            "{id}"
        );
        assert_eq!(
            value.fingerprint(),
            case.get("fingerprint").unwrap().as_str(),
            "{id}"
        );
    }
}
