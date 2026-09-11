//! 固定本家の内容確認パーサの可視性・ハッシュ契約。
#![allow(clippy::unwrap_used)]
use core_command_domain::orchestration::{SummaryQuestions, SummaryQuestionsError};
#[test]
fn summary_visibility_matches_the_saved_upstream_observations() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/golden/selfhost-stage1/summary-visibility.json"
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
    assert!(!observations.is_empty(), "空の採取物では適合を判定しない");
    let mut identifiers = std::collections::BTreeSet::new();
    for case in observations {
        assert!(identifiers.insert(case.get("id").unwrap().as_str().unwrap()));
        let content = case.get("content").unwrap().as_str().unwrap();
        let expected = case.get("expected_answer").unwrap().as_str().unwrap();
        let answer = case.get("answer").unwrap().as_str();
        let result = SummaryQuestions::parse(content, expected);
        if answer != Some(expected) {
            assert_eq!(
                result,
                Err(SummaryQuestionsError::InvalidAnswer),
                "{}",
                case.get("id").unwrap()
            );
        } else if let Some(error) = case.get("hash_error").unwrap().as_str() {
            assert_eq!(
                result,
                Err(SummaryQuestionsError::InvalidStructure(error.to_string())),
                "{}",
                case.get("id").unwrap()
            );
        } else {
            assert!(result.is_ok(), "{}: {result:?}", case.get("id").unwrap());
            let result = result.unwrap();
            assert_eq!(result.answer(), expected);
            assert_eq!(
                Some(result.sha256()),
                case.get("hash").unwrap().as_str(),
                "{}",
                case.get("id").unwrap()
            );
        }
    }
}
