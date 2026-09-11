//! 固定本家のClaude履歴判定を公開APIで比較する。
use harness_claude::StopTranscript;

#[test]
fn claude_conversation_classification_matches_fixed_upstream() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/stop-transcript.json"
    ))
    .expect("固定本家の観測");
    assert_eq!(
        corpus
            .pointer("/source/commit")
            .and_then(serde_json::Value::as_str),
        Some("a277af218f0df7f325d3b8be7b6d90fce2c5bd40")
    );
    let cases = corpus
        .get("observations")
        .and_then(serde_json::Value::as_array)
        .expect("観測配列");
    assert!(!cases.is_empty());
    let mut names = std::collections::BTreeSet::new();
    for case in cases {
        let name = case
            .get("id")
            .and_then(serde_json::Value::as_str)
            .expect("観測ID");
        assert!(names.insert(name), "重複ID: {name}");
        let transcript = case
            .get("transcript")
            .and_then(serde_json::Value::as_str)
            .expect("Claude JSONL");
        let expected = case
            .get("conversational")
            .and_then(serde_json::Value::as_bool)
            .expect("本家の判定");
        assert_eq!(
            StopTranscript::parse(transcript).is_conversational(),
            expected,
            "{name}"
        );
    }
}
