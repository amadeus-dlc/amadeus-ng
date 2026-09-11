//! Claude SessionStartの輸送値。
use serde_json::Value;
#[derive(Debug, Clone, PartialEq, Eq)]
/// sourceの分類を保つ開始通知。workflowの状態は読まない。
pub struct SessionStartEnvelope {
    source: String,
    session: Option<String>,
    transcript: String,
    rebind_check: bool,
}
impl SessionStartEnvelope {
    const fn new(
        source: String,
        session: Option<String>,
        transcript: String,
        rebind_check: bool,
    ) -> Self {
        Self {
            source,
            session,
            transcript,
            rebind_check,
        }
    }
    /// 空入力はstartup、不正JSONはmalformed、非objectはunknownとして保持する。
    #[must_use]
    pub fn parse(input: &str) -> Self {
        if input.is_empty() {
            return Self::new("startup".into(), None, String::new(), false);
        }
        let value: Value = match serde_json::from_str(input) {
            Ok(value) => value,
            Err(_) => return Self::new("malformed".into(), None, String::new(), false),
        };
        let Some(fields) = value.as_object() else {
            return Self::new("unknown".into(), None, String::new(), false);
        };
        let source = fields
            .get("source")
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
        Self::new(
            source,
            fields
                .get("session_id")
                .and_then(Value::as_str)
                .map(str::to_owned),
            fields
                .get("transcript_path")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
            fields.get("rebind_check") == Some(&Value::Bool(true)),
        )
    }
    /// sourceの綴り。
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
    /// hostのsession文字列。
    #[must_use]
    pub fn session(&self) -> Option<&str> {
        self.session.as_deref()
    }
    /// transcriptパス。空なら更新しない。
    #[must_use]
    pub fn transcript(&self) -> &str {
        &self.transcript
    }
    /// 再選択の通知だけを求める観測。
    #[must_use]
    pub const fn rebind_check(&self) -> bool {
        self.rebind_check
    }
}
