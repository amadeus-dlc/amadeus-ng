//! 本家の停止判定で使うツール呼出しの観測。シェルの安全性検査とは別の文法。

use core_infrastructure::ecmascript::is_whitespace;
use serde_json::Value;

pub(super) fn is_engine_tool_call(name: &str, input: Option<&Value>) -> bool {
    let command;
    let text = if ["bash", "shell", "execute_bash"]
        .iter()
        .any(|candidate| name.eq_ignore_ascii_case(candidate))
    {
        command = js_string(input.and_then(|value| value.get("command")));
        &command
    } else {
        name
    };
    // 本家と同じく引用符の内外を解釈せず、&& / || / ; / | / LFで分ける。
    text.replace("&&", "\n")
        .split([';', '|', '\n'])
        .any(|segment| legacy_engagement(segment) || modern_engagement(segment))
}

pub(super) fn js_string(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Bool(value)) => value.to_string(),
        Some(Value::Number(value)) => value.to_string(),
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| js_string(Some(value)))
            .collect::<Vec<_>>()
            .join(","),
        Some(Value::Object(_)) => "[object Object]".to_string(),
    }
}

fn legacy_engagement(segment: &str) -> bool {
    if !["orchestrate", "state", "jump", "bolt", "swarm", "unit"]
        .iter()
        .any(|noun| has_legacy(segment, noun))
    {
        return false;
    }
    let read_only = has_read_only_flag(segment);
    if has_legacy(segment, "orchestrate") {
        let next = has_word(segment, "next");
        let report = has_word(segment, "report");
        return report || next && !read_only;
    }
    if has_legacy(segment, "state") {
        return state_mutation(segment, false);
    }
    if has_legacy(segment, "unit") {
        return !(has_word(segment, "status") || has_word(segment, "merge-status"));
    }
    !read_only
}

fn modern_engagement(segment: &str) -> bool {
    let next = command(segment, &["next"]) || command(segment, &["orchestrate", "next"]);
    let report = command(segment, &["report"]) || command(segment, &["orchestrate", "report"]);
    let park = command(segment, &["park"]) || command(segment, &["orchestrate", "park"]);
    if next || report || park || command(segment, &["orchestrate"]) {
        return report || park || next && !has_read_only_flag(segment);
    }
    if command(segment, &["state"]) {
        return state_mutation(segment, true);
    }
    if ["jump", "bolt", "swarm"]
        .iter()
        .any(|noun| command(segment, &[noun]))
    {
        return !has_read_only_flag(segment);
    }
    command(segment, &["unit"])
        && !command(segment, &["unit", "status"])
        && !command(segment, &["unit", "merge-status"])
}

fn state_mutation(segment: &str, modern: bool) -> bool {
    [
        "approve",
        "advance",
        "finalize",
        "complete-workflow",
        "gate-start",
        "checkbox",
        "park",
        "unpark",
        "set",
        "skip",
        "reject",
        "revise",
        "resume",
    ]
    .iter()
    .any(|verb| has_word(segment, verb))
        || modern && (has_word(segment, "set-status") || has_word(segment, "init"))
}

fn has_read_only_flag(segment: &str) -> bool {
    ["--status", "--doctor", "--help", "--version"]
        .iter()
        .any(|flag| !suffixes(segment, flag, false).is_empty())
}

fn has_legacy(segment: &str, noun: &str) -> bool {
    !suffixes(segment, &format!("aidlc-{noun}"), false).is_empty()
}

fn has_word(text: &str, word: &str) -> bool {
    !suffixes(text, word, true).is_empty()
}

const fn ascii_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn suffixes<'a>(text: &'a str, token: &str, left_boundary: bool) -> Vec<&'a str> {
    text.match_indices(token)
        .filter(|(index, _)| {
            let left = text.as_bytes().get(..*index).and_then(|bytes| bytes.last());
            let right = text.as_bytes().get(index + token.len());
            (!left_boundary || left.is_none_or(|byte| !ascii_word(*byte)))
                && right.is_none_or(|byte| !ascii_word(*byte))
        })
        .filter_map(|(index, _)| text.get(index + token.len()..))
        .collect()
}

fn command(segment: &str, words: &[&str]) -> bool {
    suffixes(segment, "aidlc", true).into_iter().any(|suffix| {
        let mut rest = suffix;
        for word in words {
            if !rest.starts_with(is_whitespace) {
                return false;
            }
            rest = rest.trim_start_matches(is_whitespace);
            let Some(after) = rest.strip_prefix(word) else {
                return false;
            };
            if after
                .as_bytes()
                .first()
                .is_some_and(|byte| ascii_word(*byte))
            {
                return false;
            }
            rest = after;
        }
        true
    })
}

#[cfg(test)]
mod tests {
    use super::{is_engine_tool_call, js_string};
    use serde_json::Value;

    fn value(text: &str) -> Value {
        serde_json::from_str(text).expect("テストの JSON")
    }

    #[test]
    fn js_string_follows_the_ecmascript_string_coercion() {
        assert_eq!(js_string(None), "");
        assert_eq!(js_string(Some(&Value::Null)), "");
        assert_eq!(js_string(Some(&value("\"x\""))), "x");
        assert_eq!(js_string(Some(&value("true"))), "true");
        assert_eq!(js_string(Some(&value("false"))), "false");
        assert_eq!(js_string(Some(&value("3"))), "3");
        assert_eq!(js_string(Some(&value("2.5"))), "2.5");
        assert_eq!(js_string(Some(&value("[1,\"a\",null,[2,3]]"))), "1,a,,2,3");
        assert_eq!(js_string(Some(&value("{\"k\":1}"))), "[object Object]");
    }

    #[test]
    fn a_word_boundary_is_required_after_each_command_word() {
        // `aidlc report` と `aidlc reporter` を区別する。
        let input = |command: &str| value(&format!("{{\"command\":\"{command}\"}}"));
        assert!(is_engine_tool_call(
            "Bash",
            Some(&input("aidlc report --stage x"))
        ));
        assert!(!is_engine_tool_call("Bash", Some(&input("aidlc reporter"))));
        assert!(!is_engine_tool_call("Bash", Some(&input("aidlcreport"))));
        assert!(!is_engine_tool_call("Bash", Some(&input("aidlc"))));
        // command が無ければ空の綴りとして扱う。
        assert!(!is_engine_tool_call("bash", None));
        assert!(!is_engine_tool_call("shell", Some(&value("{}"))));
    }
}
