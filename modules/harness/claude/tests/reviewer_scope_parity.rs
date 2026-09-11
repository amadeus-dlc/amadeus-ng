//! 固定 2.7.1 の `evaluateReviewerScope` と本 build の読み取り範囲判定を全数で突き合わせる。
//!
//! ゴールデンは `scripts/goldens/capture-reviewer-scope.ts` が固定ピン
//! `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の配布実バイトを**実際に走らせて**採取した
//! 入出力である (来歴は同ファイルの `source` を参照)。ここでは同じ入力を本 build の
//! 封筒 → ドメイン判定へ通し、拒否したか・どの綴りを拒否したかをバイトで比較する。
#![allow(clippy::unwrap_used, clippy::expect_used)]
use core_command_domain::orchestration::{
    ExemptPaths, ReviewedStage, ReviewedUnit, ReviewerDispatch, ReviewerScope,
    ReviewerScopeVerdict, ScopeToken,
};
use harness_claude::ReviewerScopeEnvelope;
use serde_json::Value;

/// ゴールデンの差し向けと基点から、本 build の判定に使う範囲を組む。
fn scope(golden: &Value) -> ReviewerScope {
    let dispatch = golden.get("dispatch").unwrap();
    let exempt = dispatch
        .get("exempt")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .map(|entry| ScopeToken::parse(entry).unwrap())
        .collect();
    let dispatch = ReviewerDispatch::new(
        dispatch
            .get("reviewer")
            .and_then(Value::as_str)
            .unwrap()
            .to_string(),
        ReviewedStage::new(
            dispatch
                .get("stage")
                .and_then(Value::as_str)
                .unwrap()
                .to_string(),
        ),
        ReviewedUnit::parse(dispatch.get("unit").and_then(Value::as_str).unwrap()).unwrap(),
        ExemptPaths::new(exempt),
    );
    let context = golden.get("context").unwrap();
    ReviewerScope::of(
        &dispatch,
        context.get("recordRoot").and_then(Value::as_str),
        context.get("cwd").and_then(Value::as_str),
    )
}

#[test]
fn every_upstream_observation_is_reproduced_byte_for_byte() {
    let golden: Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/upstream-a277af21/reviewer-scope/cases.json"
    ))
    .unwrap();
    assert_eq!(
        golden
            .get("source")
            .and_then(|source| source.get("commit"))
            .and_then(Value::as_str),
        Some("a277af218f0df7f325d3b8be7b6d90fce2c5bd40"),
        "ゴールデンの来歴が固定ピンでない"
    );
    let scope = scope(&golden);
    let observations = golden
        .get("observations")
        .and_then(Value::as_array)
        .unwrap();
    assert!(observations.len() >= 165, "採取件数が減っている");

    let mut divergences: Vec<String> = Vec::new();
    for observation in observations {
        let id = observation.get("id").and_then(Value::as_str).unwrap();
        let tool = observation.get("tool").and_then(Value::as_str).unwrap();
        let input = observation.get("tool_input").unwrap();
        let expected_block = observation.get("block").and_then(Value::as_bool).unwrap();
        let expected_target = observation
            .get("target")
            .and_then(Value::as_str)
            .unwrap_or_default();

        // 契約 JSON の直列化は canon-json に固定されているので (`clippy.toml`)、
        // ハーネス入力は `Value` の表示から逐語で組む。
        let stdin = format!(
            r#"{{"hook_event_name":"PreToolUse","tool_name":{},"tool_input":{input}}}"#,
            Value::String(tool.to_string())
        );
        let envelope = ReviewerScopeEnvelope::parse(&stdin);
        let verdict = envelope
            .tool()
            .map_or(ReviewerScopeVerdict::Allowed, |tool| {
                scope.judge(tool, envelope.candidates())
            });
        let (block, target) = match &verdict {
            ReviewerScopeVerdict::Allowed => (false, String::new()),
            ReviewerScopeVerdict::Blocked(blocked) => (true, blocked.target().as_str().to_string()),
        };
        if block != expected_block || target != expected_target {
            divergences.push(format!(
                "{id} tool={tool} input={input}\n  upstream: block={expected_block} target={expected_target:?}\n  本 build: block={block} target={target:?}"
            ));
        }
    }
    assert!(
        divergences.is_empty(),
        "{} / {} 件が本家と違う:\n{}",
        divergences.len(),
        observations.len(),
        divergences.join("\n")
    );
}
