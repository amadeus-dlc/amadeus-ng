//! Claudeのコマンド封筒を、正規エンジンへの入口制約に照合する。
use core_infrastructure::ecmascript::trim;
use serde_json::Value;
/// 拒否文言だけを持つフック応答。状態や監査を更新しない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateTransitionGuard {
    denial: Option<String>,
}
impl StateTransitionGuard {
    const fn new(denial: Option<String>) -> Self {
        Self { denial }
    }
    /// 本家のBash入力だけを検査する。
    /// # Errors
    /// 固定シェルパターンの構築または文字位置の照合に失敗した場合。
    pub fn evaluate(input: &str) -> Result<Self, harness_infrastructure::ShellParseError> {
        let parsed = serde_json::from_str::<Value>(input).ok();
        Ok(Self::new(match parsed.as_ref() {
            Some(input) => denial(input)?,
            None => None,
        }))
    }
    /// exit 2で返す逐語文言。Noneは無変更の許可。
    #[must_use]
    pub fn denial(&self) -> Option<&str> {
        self.denial.as_deref()
    }
}
fn denial(input: &Value) -> Result<Option<String>, harness_infrastructure::ShellParseError> {
    if input.get("tool_name").and_then(Value::as_str) != Some("Bash") {
        return Ok(None);
    }
    let command = input
        .get("tool_input")
        .and_then(|value| value.get("command"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let text = harness_infrastructure::ShellText::new(command.to_string()).invocation_text()?;
    let pattern = harness_infrastructure::compile_shell_pattern(
        r#"(?:^|&&|\|\||[;|(\n{])[ \t]*(?:(?:command|exec)\s+)?(?:env(?:\s+-[^\s]+)*\s+)?(?:[A-Za-z_][A-Za-z0-9_]*=(?:"[^"\n]*"|'[^'\n]*'|[^\s;&|]+)\s+)*(?:[^\s"';&|({]+/)?bun(?:\.exe)?(?:\s+run)?\s+(?:"[^"\n]*aidlc-state\.ts"|'[^'\n]*aidlc-state\.ts'|[^\s;&|]*aidlc-state\.ts)\s+([a-z][a-z0-9-]*)\b"#,
    )?;
    for found in pattern.captures_iter(&text) {
        if let Some(verb) = found
            .get(1)
            .map(|value| value.as_str())
            .filter(|verb| blocked_state_transition(verb))
        {
            return Ok(Some(format!(
                "Stage status cannot be changed with aidlc-state.ts {verb} because that bypasses the workflow's completion and approval checks. Use aidlc-orchestrate.ts report --stage <slug> --result <awaiting-approval|approved|rejected|revised|completed|skipped>; use aidlc-orchestrate.ts park to pause, and next/jump to move through the workflow."
            )));
        }
    }
    let agent = trim(
        input
            .get("agent_type")
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    if !agent.is_empty()
        && let Some(command) = super::delegated_lifecycle::detect(command)?
    {
        return Ok(Some(format!(
            "Delegated agent \"{agent}\" cannot run {command} because only the main workflow session can change stage status or routing. Return the artifact, contribution, or review verdict to the main session without parking, resuming, reporting, routing, or presenting an approval question."
        )));
    }

    Ok(None)
}
pub(super) fn blocked_state_transition(verb: &str) -> bool {
    matches!(
        verb,
        "set"
            | "checkbox"
            | "advance"
            | "finalize"
            | "complete-workflow"
            | "gate-start"
            | "approve"
            | "reject"
            | "revise"
            | "skip"
            | "park"
            | "refresh-unit-progress"
            | "fold-unit-merge"
    )
}
