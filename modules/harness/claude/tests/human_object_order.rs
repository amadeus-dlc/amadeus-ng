//! 固定本家の抽出関数によるオブジェクト選択順。
#![allow(clippy::unwrap_used)]
#[test]
fn object_response_order_matches_fixed_source_observations() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/human-object-order.json"
    ))
    .unwrap();
    let cases = corpus.get("observations").unwrap().as_array().unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        let input = case.get("stdin").unwrap().as_str().unwrap();
        let response = harness_claude::HumanTurnEnvelope::parse(input);
        assert_eq!(
            response.response(),
            case.get("response").unwrap().as_str().unwrap(),
            "{}",
            case.get("id").unwrap()
        );
    }
}
