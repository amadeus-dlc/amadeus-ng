//! 通常ClaudeのTaskUpdateにある実行中stageの輸送値。
/// activeFormの末尾から得たstage名。ワークフロー状態の判断は含まない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskUpdateEnvelope {
    stage: String,
}
impl TaskUpdateEnvelope {
    const fn new(stage: String) -> Self {
        Self { stage }
    }
    /// 不正JSON・通常Claudeに適用しない入力・末尾slugなしを無視する。
    #[must_use]
    pub fn parse(input: &str) -> Option<Self> {
        let value: serde_json::Value = serde_json::from_str(input).ok()?;
        let input = value.as_object()?.get("tool_input")?.as_object()?;
        if input.get("source").and_then(serde_json::Value::as_str) == Some("ide-audit-sync")
            || input.get("status").and_then(serde_json::Value::as_str) != Some("in_progress")
        {
            return None;
        }
        let active = input.get("activeForm")?.as_str()?;
        let (_, stage) = active.strip_suffix(']')?.rsplit_once('[')?;
        let mut bytes = stage.bytes();
        if !bytes.next()?.is_ascii_lowercase()
            || !bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return None;
        }
        Some(Self::new(stage.to_string()))
    }
    /// transport上のstage名。
    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }
}
