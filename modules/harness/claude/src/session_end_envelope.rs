//! Claude SessionEndの輸送値。
use serde_json::Value;
#[derive(Debug, Clone, PartialEq, Eq)]
/// 不正JSONでもanonymous/unknownへ落ちる終了通知。
pub struct SessionEndEnvelope {
    session: Option<String>,
    reason: String,
}
impl SessionEndEnvelope {
    const fn new(session: Option<String>, reason: String) -> Self {
        Self { session, reason }
    }
    /// sourceと同じtruthyなreasonだけをStringへ写す。
    #[must_use]
    pub fn parse(input: &str) -> Self {
        let value: Value = serde_json::from_str(input).unwrap_or(Value::Null);
        let session = value
            .as_object()
            .and_then(|fields| fields.get("session_id"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let reason = value
            .as_object()
            .and_then(|fields| fields.get("reason"))
            .filter(|value| match value {
                Value::Null => false,
                Value::Bool(value) => *value,
                Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
                Value::String(value) => !value.is_empty(),
                Value::Array(_) | Value::Object(_) => true,
            })
            .map_or_else(
                || "unknown".into(),
                |value| crate::engine_tool_call::js_string(Some(value)),
            );
        Self::new(session, reason)
    }
    /// hostが送ったsession文字列。
    #[must_use]
    pub fn session(&self) -> Option<&str> {
        self.session.as_deref()
    }
    /// 終了理由の綴り。
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}
