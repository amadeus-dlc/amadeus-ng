//! 工程を進めるエンジン呼出しをシェル文から見分ける。
//!
//! 本家 `hooks/aidlc-state-transition-guard.ts:215-226` の `isLifecycleBoundaryCommand`。
//! 拒否のための検査（[`crate::StateTransitionGuard`]）とは目的が違い、こちらは
//! 「この呼出しの前に利用量を締めるか」だけを決める。
use crate::state_transition_guard::blocked_state_transition;
use harness_infrastructure::{ShellParseError, ShellText, compile_shell_pattern};

/// `bun … aidlc-{orchestrate,state,jump}.ts <verb>` のうち、工程を進めるもの。
///
/// # Errors
/// 固定シェルパターンの構築、または引用・heredocの遮蔽に失敗した場合。
pub(super) fn is_lifecycle_boundary_command(command: &str) -> Result<bool, ShellParseError> {
    let text = ShellText::new(command.to_string()).invocation_text()?;
    let pattern = compile_shell_pattern(
        r#"(?:^|&&|\|\||[;|(\n{])[ \t]*(?:(?:command|exec)\s+)?(?:env(?:\s+-[^\s]+)*\s+)?(?:[A-Za-z_][A-Za-z0-9_]*=(?:"[^"\n]*"|'[^'\n]*'|[^\s;&|]+)\s+)*(?:[^\s"';&|({]+/)?bun(?:\.exe)?(?:\s+run)?\s+(?:"[^"\n]*aidlc-(orchestrate|state|jump)\.ts"|'[^'\n]*aidlc-(orchestrate|state|jump)\.ts'|[^\s;&|]*aidlc-(orchestrate|state|jump)\.ts)\s+([a-z][a-z0-9-]*)\b"#,
    )?;
    for found in pattern.captures_iter(&text) {
        let tool = (1..=3)
            .find_map(|index| found.get(index))
            .map(|value| value.as_str())
            .unwrap_or_default();
        let verb = found.get(4).map(|value| value.as_str()).unwrap_or_default();
        let boundary = match tool {
            "orchestrate" => verb == "report",
            "state" => blocked_state_transition(verb),
            "jump" => verb == "execute",
            _ => false,
        };
        if boundary {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boundary(command: &str) -> bool {
        is_lifecycle_boundary_command(command).expect("固定パターンは構築できる")
    }

    #[test]
    fn reporting_a_stage_result_is_a_lifecycle_boundary() {
        assert!(boundary(
            "bun .claude/tools/aidlc-orchestrate.ts report --result completed"
        ));
    }

    #[test]
    fn a_read_only_orchestrate_verb_is_not_a_boundary() {
        for command in [
            "bun .claude/tools/aidlc-orchestrate.ts next",
            "bun .claude/tools/aidlc-orchestrate.ts park",
            "bun .claude/tools/aidlc-utility.ts --status",
            "ls -la",
            "",
        ] {
            assert!(!boundary(command), "{command}");
        }
    }

    #[test]
    fn blocked_state_verbs_and_jump_execute_are_boundaries() {
        assert!(boundary("bun .claude/tools/aidlc-state.ts approve"));
        assert!(boundary(
            "bun .claude/tools/aidlc-jump.ts execute --stage x"
        ));
        assert!(!boundary(
            "bun .claude/tools/aidlc-jump.ts preview --stage x"
        ));
    }

    #[test]
    fn a_quoted_mention_inside_another_word_is_not_an_invocation() {
        assert!(!boundary(
            "echo 'bun .claude/tools/aidlc-orchestrate.ts report'"
        ));
    }
}
