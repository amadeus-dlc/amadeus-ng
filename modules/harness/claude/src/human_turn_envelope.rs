//! Claudeの応答封筒。未知の形でも旧来のpresence記録を妨げない。
use core_infrastructure::ecmascript::trim;
use serde_json::Value;
/// セッションと元の応答。権限判断は含まない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanTurnEnvelope {
    session: String,
    response: String,
}
impl HumanTurnEnvelope {
    /// 配布フックと同じ優先順で文字列を取り出す。番号は元の文字列を保持する。
    #[must_use]
    pub fn parse(input: &str) -> Self {
        let parsed = serde_json::from_str::<Value>(input).unwrap_or(Value::Null);
        let session = trim(
            parsed
                .get("session_id")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        )
        .to_string();
        let response = [
            "prompt",
            "user_prompt",
            "message",
            "tool_response",
            "toolResponse",
        ]
        .iter()
        .filter_map(|key| parsed.get(key))
        .map(extract_text)
        .find(|text| !text.is_empty())
        .unwrap_or_default();
        Self { session, response }
    }
    /// 封筒のセッション識別子。
    #[must_use]
    pub fn session(&self) -> &str {
        &self.session
    }
    /// 元の応答。
    #[must_use]
    pub fn response(&self) -> &str {
        &self.response
    }
}
fn extract_text(value: &Value) -> String {
    match value {
        Value::String(text) => {
            let text = trim(text);
            match serde_json::from_str::<Value>(text) {
                Ok(Value::Number(_)) => text.to_string(),
                Ok(parsed) => extract_text(&parsed),
                Err(_) => text.to_string(),
            }
        }
        Value::Array(values) => values
            .iter()
            .map(extract_text)
            .find(|text| !text.is_empty())
            .unwrap_or_default(),
        Value::Object(fields) => {
            let mut ordered: Vec<_> = fields.iter().collect();
            ordered.sort_by_key(|(key, _)| {
                core_infrastructure::ecmascript::array_index(key).map_or((1, 0), |index| (0, index))
            });
            [
                "answer",
                "answers",
                "selected",
                "selection",
                "value",
                "label",
                "text",
            ]
            .iter()
            .filter_map(|key| fields.get(*key))
            .chain(ordered.into_iter().map(|(_, value)| value))
            .map(extract_text)
            .find(|text| !text.is_empty())
            .unwrap_or_default()
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::HumanTurnEnvelope;
    #[test]
    fn integer_object_keys_follow_ecmascript_property_order() {
        let envelope = HumanTurnEnvelope::parse(
            r#"{"session_id":"s","prompt":{"2":"Approve Plan","1":"Request Changes"}}"#,
        );
        assert_eq!(envelope.response(), "Request Changes");
    }
    #[test]
    fn a_numeric_json_value_is_not_a_protected_response_candidate() {
        let envelope = HumanTurnEnvelope::parse(r#"{"session_id":"s","prompt":1}"#);
        assert_eq!(envelope.session(), "s");
        assert_eq!(envelope.response(), "");
    }
    #[test]
    fn a_numeric_string_remains_the_original_human_response() {
        let envelope = HumanTurnEnvelope::parse(r#"{"session_id":"s","prompt":"1"}"#);
        assert_eq!(envelope.response(), "1");
    }
    #[test]
    fn ecmascript_bom_whitespace_is_removed_from_session_and_response() {
        let envelope = HumanTurnEnvelope::parse(r#"{"session_id":"\uFEFFs","prompt":"\uFEFF1"}"#);
        assert_eq!(envelope.session(), "s");
        assert_eq!(envelope.response(), "1");
    }
    #[test]
    fn nel_is_preserved_in_session_and_response() {
        let envelope = HumanTurnEnvelope::parse(r#"{"session_id":"\u0085s","prompt":"\u00851"}"#);
        assert_eq!(envelope.session(), "\u{85}s");
        assert_eq!(envelope.response(), "\u{85}1");
    }
}
