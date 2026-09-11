//! Claudeの利用量畳み込みフックの輸送値。状態も承認も読まない。
use crate::fold_mode::FoldMode;
use crate::lifecycle_boundary_command::is_lifecycle_boundary_command;
use serde_json::Value;

/// Pre/PostToolUseの封筒のうち、利用量の producer が使う値だけを保つ。
///
/// session文字列の安全形検査は配置を知る合成ルートが持つので、ここでは素通しする
/// （[`crate::SessionStartEnvelope`] と同じ分担）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldUsageEnvelope {
    session: Option<String>,
    transcript: Option<String>,
    fold_mode: FoldMode,
}

impl FoldUsageEnvelope {
    const fn new(session: Option<String>, transcript: Option<String>, fold_mode: FoldMode) -> Self {
        Self {
            session,
            transcript,
            fold_mode,
        }
    }

    /// 不正JSON・非object・値の欠落はいずれも「畳むものが無い」観測にする。
    #[must_use]
    pub fn parse(input: &str) -> Self {
        let Ok(value) = serde_json::from_str::<Value>(input) else {
            return Self::new(None, None, FoldMode::Holdback);
        };
        let Some(fields) = value.as_object() else {
            return Self::new(None, None, FoldMode::Holdback);
        };
        let session = fields
            .get("session_id")
            .and_then(Value::as_str)
            .filter(|session| !session.is_empty())
            .map(str::to_owned);
        let transcript = fields
            .get("transcript_path")
            .and_then(Value::as_str)
            .filter(|path| !path.is_empty())
            .map(str::to_owned);
        let fold_mode =
            if fields.get("hook_event_name").and_then(Value::as_str) == Some("PreToolUse") {
                let tool = fields
                    .get("tool_name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if lifecycle_boundary(tool, fields.get("tool_input")) {
                    FoldMode::FlushAll
                } else {
                    FoldMode::SealMain
                }
            } else {
                FoldMode::Holdback
            };
        Self::new(session, transcript, fold_mode)
    }

    /// hostのsession文字列（安全形の検査は呼出側が行う）。
    #[must_use]
    pub fn session(&self) -> Option<&str> {
        self.session.as_deref()
    }

    /// 会話履歴のパス。無いときは何もしない。
    #[must_use]
    pub fn transcript(&self) -> Option<&str> {
        self.transcript.as_deref()
    }

    /// この呼出しでどこまでの群を締めるか。
    #[must_use]
    pub const fn fold_mode(&self) -> FoldMode {
        self.fold_mode
    }
}

/// この呼出しが工程を進めるか。シェル文の解析に失敗した封筒は境界と見なさない。
fn lifecycle_boundary(tool: &str, input: Option<&Value>) -> bool {
    if !["bash", "shell", "execute_bash"]
        .iter()
        .any(|candidate| tool.eq_ignore_ascii_case(candidate))
    {
        return crate::engine_tool_call::is_engine_tool_call(tool, input);
    }
    input
        .and_then(|input| input.get("command"))
        .and_then(Value::as_str)
        .is_some_and(|command| is_lifecycle_boundary_command(command).unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_post_tool_use_envelope_carries_the_session_and_the_transcript() {
        let envelope = FoldUsageEnvelope::parse(
            r#"{"session_id":"abc-123","hook_event_name":"PostToolUse","transcript_path":"/tmp/t.jsonl"}"#,
        );
        assert_eq!(envelope.session(), Some("abc-123"));
        assert_eq!(envelope.transcript(), Some("/tmp/t.jsonl"));
    }

    #[test]
    fn a_missing_session_still_leaves_the_transcript_to_fold() {
        let envelope = FoldUsageEnvelope::parse(r#"{"transcript_path":"/tmp/t.jsonl"}"#);
        assert_eq!(envelope.session(), None);
        assert_eq!(envelope.transcript(), Some("/tmp/t.jsonl"));
    }

    #[test]
    fn malformed_and_non_object_input_yields_nothing_to_fold() {
        for input in [
            "",
            "not json",
            "[]",
            "null",
            "{}",
            r#"{"transcript_path":""}"#,
        ] {
            let envelope = FoldUsageEnvelope::parse(input);
            assert_eq!(envelope.transcript(), None, "入力: {input}");
        }
    }
}
