//! Claude の JSON 封筒が不正・欠落・型違いの入力をどう扱うかの契約。
//!
//! 「不正入力を拒否する」「欠落を既定へ落とす」「JS の真偽値・文字列化の規則を逐語で写す」を
//! 固定する。プロダクトの動作を写すだけの検査ではなく、本家 (`.claude/hooks/*.ts`) と同じ
//! 観測になることを主張する。
use harness_claude::{
    HumanTurnEnvelope, SessionEndEnvelope, SessionStartEnvelope, SubagentEnvelopeError,
    SubagentStopEnvelope,
};

#[test]
fn a_subagent_stop_ignores_non_object_input_and_refuses_a_non_string_message() {
    assert_eq!(SubagentStopEnvelope::parse("not json").unwrap(), None);
    assert_eq!(SubagentStopEnvelope::parse("[1,2]").unwrap(), None);
    assert_eq!(SubagentStopEnvelope::parse("\"text\"").unwrap(), None);
    let error = SubagentStopEnvelope::parse(r#"{"last_assistant_message": 42}"#)
        .expect_err("文字列以外の message は本家の slice を適用できない");
    assert_eq!(error, SubagentEnvelopeError::MessageType);
    assert_eq!(error.to_string(), "last_assistant_message must be a string");
    assert!(SubagentStopEnvelope::parse(r#"{"last_assistant_message": {}}"#).is_err());
}

#[test]
fn a_subagent_stop_reads_its_fields_with_javascript_truthiness() {
    let envelope = SubagentStopEnvelope::parse(
        r#"{"session_id":"s1","agent_type":"aidlc-developer-agent","agent_id":"a1","last_assistant_message":"done"}"#,
    )
    .unwrap()
    .expect("object は封筒になる");
    assert_eq!(envelope.session(), Some("s1"));
    assert_eq!(envelope.agent_type(), "aidlc-developer-agent");
    assert_eq!(envelope.agent_id(), "a1");
    assert_eq!(envelope.message(), "done");

    // 欠落と null は既定へ落ちる。
    let bare = SubagentStopEnvelope::parse("{}").unwrap().unwrap();
    assert_eq!(bare.session(), None);
    assert_eq!(bare.agent_type(), "unknown");
    assert_eq!(bare.agent_id(), "");
    assert_eq!(bare.message(), "");
    let null = SubagentStopEnvelope::parse(
        r#"{"agent_type":null,"agent_id":null,"last_assistant_message":null}"#,
    )
    .unwrap()
    .unwrap();
    assert_eq!(null.agent_type(), "unknown");
    assert_eq!(null.agent_id(), "");

    // agent_id は JS の truthy 判定: false / 0 / "" は空、true / 数 / 配列 / object は文字列化。
    for (input, expected) in [
        (r#"{"agent_id": false}"#, ""),
        (r#"{"agent_id": 0}"#, ""),
        (r#"{"agent_id": ""}"#, ""),
        (r#"{"agent_id": true}"#, "true"),
        (r#"{"agent_id": 7}"#, "7"),
        (r#"{"agent_id": [1,"a"]}"#, "1,a"),
        (r#"{"agent_id": {"k":1}}"#, "[object Object]"),
    ] {
        let envelope = SubagentStopEnvelope::parse(input).unwrap().unwrap();
        assert_eq!(envelope.agent_id(), expected, "{input}");
    }
    // agent_type は null だけが既定で、それ以外は文字列化する。
    let typed = SubagentStopEnvelope::parse(r#"{"agent_type": 0}"#)
        .unwrap()
        .unwrap();
    assert_eq!(typed.agent_type(), "0");
}

#[test]
fn a_subagent_stop_message_keeps_only_the_first_200_utf16_units() {
    let long = "あ".repeat(250);
    let input = format!(r#"{{"last_assistant_message":"{long}"}}"#);
    let envelope = SubagentStopEnvelope::parse(&input).unwrap().unwrap();
    assert_eq!(envelope.message().chars().count(), 200);
    // サロゲートペアの途中で切れても落ちない (lossy)。
    let pairs = "😀".repeat(101);
    let input = format!(r#"{{"last_assistant_message":"{pairs}"}}"#);
    let envelope = SubagentStopEnvelope::parse(&input).unwrap().unwrap();
    assert_eq!(envelope.message().encode_utf16().count(), 200);
}

#[test]
fn a_session_start_classifies_empty_malformed_and_non_object_input() {
    let empty = SessionStartEnvelope::parse("");
    assert_eq!(empty.source(), "startup");
    assert_eq!(empty.session(), None);
    assert_eq!(empty.transcript(), "");
    assert!(!empty.rebind_check());
    assert_eq!(
        SessionStartEnvelope::parse("{not json").source(),
        "malformed"
    );
    assert_eq!(SessionStartEnvelope::parse("[]").source(), "unknown");
    assert_eq!(SessionStartEnvelope::parse("\"s\"").source(), "unknown");
}

#[test]
fn a_session_start_source_follows_javascript_truthiness() {
    for (input, expected) in [
        (r#"{}"#, "unknown"),
        (r#"{"source": null}"#, "unknown"),
        (r#"{"source": false}"#, "unknown"),
        (r#"{"source": 0}"#, "unknown"),
        (r#"{"source": ""}"#, "unknown"),
        (r#"{"source": "resume"}"#, "resume"),
        (r#"{"source": true}"#, "true"),
        (r#"{"source": 2}"#, "2"),
        (r#"{"source": ["a"]}"#, "a"),
        (r#"{"source": {}}"#, "[object Object]"),
    ] {
        assert_eq!(
            SessionStartEnvelope::parse(input).source(),
            expected,
            "{input}"
        );
    }
    let full = SessionStartEnvelope::parse(
        r#"{"source":"startup","session_id":"s1","transcript_path":"/t.jsonl","rebind_check":true}"#,
    );
    assert_eq!(full.session(), Some("s1"));
    assert_eq!(full.transcript(), "/t.jsonl");
    assert!(full.rebind_check());
    // rebind_check は真偽値の true だけを認める。
    assert!(!SessionStartEnvelope::parse(r#"{"rebind_check":"true"}"#).rebind_check());
    assert!(!SessionStartEnvelope::parse(r#"{"rebind_check":1}"#).rebind_check());
    // session_id / transcript_path は文字列以外を捨てる。
    let typed = SessionStartEnvelope::parse(r#"{"session_id":1,"transcript_path":["x"]}"#);
    assert_eq!(typed.session(), None);
    assert_eq!(typed.transcript(), "");
}

#[test]
fn a_session_end_falls_back_to_unknown_and_anonymous() {
    let malformed = SessionEndEnvelope::parse("{oops");
    assert_eq!(malformed.session(), None);
    assert_eq!(malformed.reason(), "unknown");
    assert_eq!(SessionEndEnvelope::parse("").reason(), "unknown");
    assert_eq!(SessionEndEnvelope::parse("[1]").reason(), "unknown");
    for (input, expected) in [
        (r#"{"reason": null}"#, "unknown"),
        (r#"{"reason": false}"#, "unknown"),
        (r#"{"reason": 0}"#, "unknown"),
        (r#"{"reason": ""}"#, "unknown"),
        (r#"{"reason": "clear"}"#, "clear"),
        (r#"{"reason": true}"#, "true"),
        (r#"{"reason": 1.5}"#, "1.5"),
        (r#"{"reason": ["logout",2]}"#, "logout,2"),
        (r#"{"reason": {"a":1}}"#, "[object Object]"),
    ] {
        assert_eq!(
            SessionEndEnvelope::parse(input).reason(),
            expected,
            "{input}"
        );
    }
    let full = SessionEndEnvelope::parse(r#"{"session_id":"s9","reason":"exit"}"#);
    assert_eq!(full.session(), Some("s9"));
    assert_eq!(full.reason(), "exit");
    assert_eq!(
        SessionEndEnvelope::parse(r#"{"session_id":9}"#).session(),
        None
    );
}

#[test]
fn a_human_turn_unwraps_nested_json_strings_arrays_and_objects() {
    // 文字列が JSON を運ぶときは中身を辿る。数値の JSON はそのままの綴りを保つ。
    let nested = HumanTurnEnvelope::parse(r#"{"session_id":" s ","prompt":"{\"answer\":\"B\"}"}"#);
    assert_eq!(nested.session(), "s");
    assert_eq!(nested.response(), "B");
    let numeric = HumanTurnEnvelope::parse(r#"{"prompt":"  2  "}"#);
    assert_eq!(numeric.response(), "2");
    // 配列は最初の空でない要素。
    let array = HumanTurnEnvelope::parse(r#"{"message":["", "  ", "C", "D"]}"#);
    assert_eq!(array.response(), "C");
    let array_of_arrays = HumanTurnEnvelope::parse(r#"{"user_prompt":[[], ["E"]]}"#);
    assert_eq!(array_of_arrays.response(), "E");
    // 真偽値・数値・null は文字列にならない。
    let empty = HumanTurnEnvelope::parse(r#"{"prompt": true, "tool_response": 3}"#);
    assert_eq!(empty.response(), "");
    // 不正 JSON は空の封筒。
    let malformed = HumanTurnEnvelope::parse("nope");
    assert_eq!(malformed.session(), "");
    assert_eq!(malformed.response(), "");
    // 優先順: prompt → user_prompt → message → tool_response → toolResponse。
    let ordered = HumanTurnEnvelope::parse(r#"{"toolResponse":"z","message":"m","prompt":""}"#);
    assert_eq!(ordered.response(), "m");
}
