//! 固定本家の文面式を実行した観測と、公開描画APIを全文比較する。
#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "固定観測の欠落や未知の種別は、テストを即時失敗させる"
)]
use harness_claude::{SessionContextNotices, SessionStartContext, SessionWorkflowFields};
use serde_json::Value;

fn field<'a>(value: &'a Value, name: &str) -> &'a str {
    value.get(name).and_then(Value::as_str).expect("表示材料")
}

#[test]
fn session_start_text_matches_every_fixed_rendering_observation() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/session-start-context.json"
    ))
    .expect("固定元観測");
    assert_eq!(
        corpus.pointer("/source/commit").and_then(Value::as_str),
        Some("a277af218f0df7f325d3b8be7b6d90fce2c5bd40")
    );
    let cases = corpus
        .get("observations")
        .and_then(Value::as_array)
        .expect("観測");
    assert_eq!(cases.len(), 11);
    for case in cases {
        let input = case.get("input").expect("input");
        let session = field(input, "session");
        let context = match field(case, "kind") {
            "workflow" => {
                let fields = SessionWorkflowFields::new(
                    field(input, "scope").into(),
                    field(input, "phase").into(),
                    field(input, "stage").into(),
                    field(input, "status").into(),
                    field(input, "agent").into(),
                    field(input, "last").into(),
                    field(input, "next").into(),
                );
                let notices = SessionContextNotices::new(
                    field(input, "rebind_offer").into(),
                    field(input, "unit_line").into(),
                    input
                        .get("recovery")
                        .and_then(Value::as_bool)
                        .expect("recovery"),
                    input
                        .get("uncompiled_stages")
                        .and_then(Value::as_array)
                        .expect("drift")
                        .iter()
                        .map(|name| name.as_str().expect("stage").to_string())
                        .collect(),
                );
                SessionStartContext::workflow(&fields, session, &notices)
            }
            "runtime" => SessionStartContext::runtime(session),
            "probe" => SessionStartContext::rebind_probe(session, field(input, "offer")),
            _ => panic!("未知の観測種別"),
        };
        assert_eq!(
            format!("{}\n", context.to_json_line()),
            field(case, "stdout"),
            "{}",
            field(case, "id")
        );
    }
    for case in corpus
        .get("rebind_cases")
        .and_then(Value::as_array)
        .expect("再選択案内")
    {
        assert_eq!(
            SessionStartContext::format_rebind_offer(
                field(case, "was"),
                field(case, "live"),
                field(case, "instruction")
            ),
            field(case, "expected")
        );
    }
}
