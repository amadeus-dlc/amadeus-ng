//! Claudeの委譲完了入力とUTF-16の表示上限。
use crate::engine_tool_call::js_string;
use serde_json::Value;
#[derive(Debug, Clone, PartialEq, Eq)]
/// 有効なJSON objectから得た委譲完了の輸送値。
pub struct SubagentStopEnvelope {
    session: Option<String>,
    agent_type: String,
    agent_id: String,
    message: String,
}
impl SubagentStopEnvelope {
    const fn new(
        session: Option<String>,
        agent_type: String,
        agent_id: String,
        message: String,
    ) -> Self {
        Self {
            session,
            agent_type,
            agent_id,
            message,
        }
    }
    /// 不正JSON/非objectは無視する。messageのUTF-16先頭200単位を保持する。
    /// # Errors
    /// messageが文字列以外で、本家のsliceを適用できない場合。
    pub fn parse(input: &str) -> Result<Option<Self>, super::SubagentEnvelopeError> {
        let Ok(value) = serde_json::from_str::<Value>(input) else {
            return Ok(None);
        };
        let Some(object) = value.as_object() else {
            return Ok(None);
        };
        let message = match object.get("last_assistant_message") {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(message)) => {
                String::from_utf16_lossy(&message.encode_utf16().take(200).collect::<Vec<_>>())
            }
            _ => return Err(super::SubagentEnvelopeError::MessageType),
        };
        let agent_type = object
            .get("agent_type")
            .filter(|value| !value.is_null())
            .map_or_else(|| "unknown".into(), |value| js_string(Some(value)));
        let agent_id = object
            .get("agent_id")
            .filter(|value| truthy(value))
            .map_or_else(String::new, |value| js_string(Some(value)));
        Ok(Some(Self::new(
            object
                .get("session_id")
                .and_then(Value::as_str)
                .map(str::to_owned),
            agent_type,
            agent_id,
            message,
        )))
    }
    /// hostのセッション名。安全なファイル名かは接続境界で確認する。
    #[must_use]
    pub fn session(&self) -> Option<&str> {
        self.session.as_deref()
    }
    /// 委譲先の種類。
    #[must_use]
    pub fn agent_type(&self) -> &str {
        &self.agent_type
    }
    /// 委譲先の識別子。
    #[must_use]
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }
    /// 200 UTF-16単位に切り詰めた本文。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}
fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}
