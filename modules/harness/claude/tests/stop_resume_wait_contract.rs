//! 固定本家の共有再開待ちmarker判定を公開APIで照合する。
use harness_claude::StopResumeWait;

#[test]
fn shared_resume_wait_matches_the_fixed_marker_validation() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/stop-resume-wait.json"
    ))
    .expect("固定観測");
    assert_eq!(
        corpus
            .pointer("/source/commit")
            .and_then(serde_json::Value::as_str),
        Some("a277af218f0df7f325d3b8be7b6d90fce2c5bd40")
    );
    let observations = corpus
        .get("observations")
        .and_then(serde_json::Value::as_array)
        .expect("観測配列");
    assert!(!observations.is_empty());
    let mut ids = std::collections::BTreeSet::new();
    for item in observations {
        let id = item
            .get("id")
            .and_then(serde_json::Value::as_str)
            .expect("ID");
        assert!(ids.insert(id), "重複ID: {id}");
        let marker = item
            .get("marker")
            .and_then(serde_json::Value::as_str)
            .expect("marker本文");
        let state = item
            .get("state_sha256")
            .and_then(serde_json::Value::as_str)
            .expect("状態hash");
        let expected = item
            .get("waiting")
            .and_then(serde_json::Value::as_bool)
            .expect("本家判定");
        assert_eq!(
            StopResumeWait::parse(marker).matches_state_hash(state),
            expected,
            "{id}"
        );
    }
}
