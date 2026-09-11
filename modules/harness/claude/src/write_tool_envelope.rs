//! 書込み系ツールの PreToolUse 封筒から、書込み先だけを取り出す。
use core_command_domain::orchestration::{WriteTarget, WriteTargets};
use core_infrastructure::collections::FirstClassCollection as _;
use harness_infrastructure::ShellWriteTargets;
use serde_json::Value;
use std::path::Path;

/// upstream `WRITE_TOOLS` — ファイルを直接書き換える工具 (逐語)。
const WRITE_TOOLS: [&str; 4] = ["Write", "Edit", "MultiEdit", "NotebookEdit"];

/// PreToolUse 入力 1 件を、工具名と書込み先の列として読んだもの。
///
/// # `Bash` も書込みとして検査する
///
/// upstream `writeTargets` と同じく、`Bash` のときはコマンド文字列を解析し、出力リダイレクトの
/// 送り先と変更系コマンドの操作対象を書込み先として返す (`hooks/review-freeze-command.ts` の
/// `shellWriteTargets`)。読取り専用のシェル呼出しは宛先を生まない。
///
/// 相対綴りの基点は封筒の `cwd`、無ければ呼出側が渡す作業ディレクトリである
/// (upstream `parsed.cwd ?? projectDir`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteToolEnvelope {
    tool: String,
    targets: WriteTargets,
}
impl WriteToolEnvelope {
    /// 工具名と書込み先を同時に構築する (**この型の唯一の構築経路**)。
    const fn new(tool: String, targets: WriteTargets) -> Self {
        Self { tool, targets }
    }
    /// PreToolUse の JSON を読む。オブジェクトでない入力は工具名なしとして扱う。
    ///
    /// `project_dir` は封筒が `cwd` を名乗らないときに相対綴りを解決する基点。
    #[must_use]
    pub fn parse(input: &str, project_dir: &Path) -> WriteToolEnvelope {
        let value = serde_json::from_str::<Value>(input).unwrap_or(Value::Null);
        let tool = value
            .get("tool_name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let targets = if tool == "Bash" {
            let cwd = value
                .get("cwd")
                .and_then(Value::as_str)
                .map_or_else(|| project_dir.to_path_buf(), std::path::PathBuf::from);
            let command = value
                .get("tool_input")
                .and_then(|input| input.get("command"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            WriteTargets::new(ShellWriteTargets::parse(command, &cwd).fold_left(
                Vec::new(),
                |mut acc: Vec<WriteTarget>, target| {
                    if let Ok(target) = WriteTarget::parse(target) {
                        acc.push(target);
                    }
                    acc
                },
            ))
        } else if WRITE_TOOLS.contains(&tool.as_str()) {
            let input = value.get("tool_input");
            let named = ["file_path", "notebook_path", "path"]
                .into_iter()
                .filter_map(|key| input.and_then(|input| input.get(key)));
            let listed = input
                .and_then(|input| input.get("paths"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten();
            WriteTargets::new(
                named
                    .chain(listed)
                    .filter_map(Value::as_str)
                    .filter_map(|raw| WriteTarget::parse(raw).ok())
                    .collect(),
            )
        } else {
            WriteTargets::empty()
        };
        WriteToolEnvelope::new(tool, targets)
    }
    /// 呼び出された工具名 (未知・欠落は空文字)。
    #[must_use]
    pub fn tool(&self) -> &str {
        &self.tool
    }
    /// この呼出しが書き換える宛先。
    #[must_use]
    pub const fn targets(&self) -> &WriteTargets {
        &self.targets
    }
}

#[cfg(test)]
mod tests {
    use super::WriteToolEnvelope;
    use core_command_domain::orchestration::WriteTarget;
    use std::path::Path;

    fn parse(input: &str) -> WriteToolEnvelope {
        WriteToolEnvelope::parse(input, Path::new("/w"))
    }

    fn targets(envelope: &WriteToolEnvelope) -> Vec<String> {
        envelope.targets().fold_left(Vec::new(), |mut acc, target| {
            acc.push(target.as_str().to_string());
            acc
        })
    }

    #[test]
    fn a_write_names_its_file_path() {
        let envelope = parse(
            r#"{"tool_name":"Write","tool_input":{"file_path":"/w/x/inception/requirements-analysis/requirements.md","content":"..."}}"#,
        );
        assert_eq!(envelope.tool(), "Write");
        assert_eq!(
            targets(&envelope),
            ["/w/x/inception/requirements-analysis/requirements.md"]
        );
    }

    #[test]
    fn every_write_tool_key_is_collected_in_order() {
        let envelope = parse(
            r#"{"tool_name":"NotebookEdit","tool_input":{"notebook_path":"/a.ipynb","path":"/b.md","paths":["/c.md","/d.md"]}}"#,
        );
        assert_eq!(targets(&envelope), ["/a.ipynb", "/b.md", "/c.md", "/d.md"]);
    }

    #[test]
    fn a_read_only_tool_names_no_write_target() {
        for input in [
            r#"{"tool_name":"Read","tool_input":{"file_path":"/w/x.md"}}"#,
            r#"{"tool_name":"Grep","tool_input":{"pattern":"x"}}"#,
        ] {
            assert!(parse(input).targets().is_empty());
        }
    }

    #[test]
    fn malformed_input_and_missing_values_yield_no_target() {
        for input in [
            "not json",
            "[]",
            r#"{"tool_name":"Write"}"#,
            r#"{"tool_name":"Write","tool_input":{"file_path":""}}"#,
            r#"{"tool_name":"Write","tool_input":{"file_path":42}}"#,
        ] {
            let envelope = parse(input);
            assert!(envelope.targets().is_empty(), "{input}");
        }
    }

    #[test]
    fn a_shell_write_names_the_targets_its_command_would_change() {
        let envelope = parse(
            r#"{"tool_name":"Bash","tool_input":{"command":"printf x >> /w/x/inception/requirements-analysis/requirements.md"}}"#,
        );
        assert_eq!(envelope.tool(), "Bash");
        assert_eq!(
            targets(&envelope),
            ["/w/x/inception/requirements-analysis/requirements.md"]
        );
    }

    #[test]
    fn a_read_only_shell_call_names_no_target() {
        for command in ["cat /w/a.md", "grep -r x /w", "ls"] {
            let input = format!(r#"{{"tool_name":"Bash","tool_input":{{"command":"{command}"}}}}"#);
            assert!(parse(&input).targets().is_empty(), "{command}");
        }
    }

    #[test]
    fn a_shell_relative_target_is_resolved_against_the_envelope_cwd_then_the_project_dir() {
        let with_cwd =
            parse(r#"{"tool_name":"Bash","cwd":"/other","tool_input":{"command":"rm a.md"}}"#);
        assert_eq!(targets(&with_cwd), ["/other/a.md"]);
        let without_cwd = parse(r#"{"tool_name":"Bash","tool_input":{"command":"rm a.md"}}"#);
        assert_eq!(targets(&without_cwd), ["/w/a.md"]);
    }

    #[test]
    fn a_shell_call_with_no_command_string_names_no_target() {
        for input in [
            r#"{"tool_name":"Bash"}"#,
            r#"{"tool_name":"Bash","tool_input":{}}"#,
            r#"{"tool_name":"Bash","tool_input":{"command":42}}"#,
        ] {
            assert!(parse(input).targets().is_empty(), "{input}");
        }
    }

    #[test]
    fn windows_separators_are_folded_by_the_target_value_object() {
        let envelope =
            parse(r#"{"tool_name":"Edit","tool_input":{"file_path":"C:\\w\\x\\requirements.md"}}"#);
        assert_eq!(targets(&envelope), ["C:/w/x/requirements.md"]);
        assert_eq!(
            envelope.targets().at(0),
            Some(&WriteTarget::parse("C:/w/x/requirements.md").unwrap())
        );
    }
}
