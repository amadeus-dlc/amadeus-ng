//! Claudeの停止直前の会話が、作業進行を伴わない応答かを読む。

use crate::engine_tool_call::{is_engine_tool_call, js_string};
use core_infrastructure::ecmascript::is_whitespace;
use serde_json::Value;

/// 会話履歴の観測結果。状態や自律実行の方針は判断しない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopTranscript {
    conversational: bool,
}

impl StopTranscript {
    const fn new(conversational: bool) -> Self {
        Self { conversational }
    }

    /// Claude形式のJSONLから、最後の人間発言以降の呼出しを判定する。
    #[must_use]
    pub fn parse(transcript: &str) -> Self {
        let mut has_human = false;
        let mut engine_after_human = false;
        for line in transcript.split('\n') {
            let Some(entry) = decode_line(line) else {
                continue;
            };
            let Some(message) = entry.get("message") else {
                continue;
            };
            let kind = entry.get("type").and_then(Value::as_str);
            let role = message.get("role").and_then(Value::as_str);
            let content = message.get("content");
            if kind == Some("user") && role == Some("user") {
                if entry.get("isMeta").and_then(Value::as_bool) == Some(true) {
                    continue;
                }
                if genuine_human(content) {
                    has_human = true;
                    engine_after_human = false;
                }
            } else if kind == Some("assistant")
                && role == Some("assistant")
                && let Some(blocks) = content.and_then(Value::as_array)
            {
                engine_after_human |= blocks.iter().any(|block| {
                    block.get("type").and_then(Value::as_str) == Some("tool_use")
                        && is_engine_tool_call(&js_string(block.get("name")), block.get("input"))
                });
            }
        }
        Self::new(has_human && !engine_after_human)
    }

    /// 人間の発言があり、その後に作業を進める呼出しがなかったか。
    #[must_use]
    pub const fn is_conversational(&self) -> bool {
        self.conversational
    }
}

fn decode_line(line: &str) -> Option<Value> {
    if let Ok(value) = serde_json::from_str(line) {
        return Some(value);
    }
    // JSON.parseが受理する孤立サロゲートや巨大な数値を含む行を、丸ごと落とさない。
    // 先に元のJSON文法を検査し、壊れた行を変換によって有効にすることを防ぐ。
    let raw: &serde_json::value::RawValue = serde_json::from_str(line).ok()?;
    serde_json::from_str(&classification_text(raw.get())).ok()
}

fn classification_text(line: &str) -> String {
    let mut output = String::with_capacity(line.len());
    let mut characters = line.chars().peekable();
    let mut in_string = false;
    while let Some(character) = characters.next() {
        if in_string && character == '\\' {
            let Some(escaped) = characters.next() else {
                break;
            };
            if escaped == 'u' {
                let code: String = characters.by_ref().take(4).collect();
                if u16::from_str_radix(&code, 16)
                    .is_ok_and(|value| (0xd800..=0xdfff).contains(&value))
                {
                    // この判定はASCIIの語境界・固定句・空白だけを観測する。
                    // サロゲートも置換文字もそのいずれでもなく、元の履歴は変更しない。
                    output.push_str("\\ufffd");
                } else {
                    output.push_str("\\u");
                    output.push_str(&code);
                }
            } else {
                output.push('\\');
                output.push(escaped);
            }
        } else if character == '"' {
            in_string = !in_string;
            output.push(character);
        } else if !in_string && (character.is_ascii_digit() || character == '-') {
            let mut number = String::from(character);
            while characters.peek().is_some_and(|next| {
                next.is_ascii_digit() || matches!(next, '.' | 'e' | 'E' | '+' | '-')
            }) {
                if let Some(next) = characters.next() {
                    number.push(next);
                }
            }
            if number.parse::<f64>().is_ok_and(|value| !value.is_finite()) {
                // 数値は人間の文字列でもtrueでもない。値の大小は判定に使わず、
                // 文字列化しても作業用の固定語やフックの固定句にはならない。
                output.push('0');
            } else {
                output.push_str(&number);
            }
        } else {
            output.push(character);
        }
    }
    output
}

fn genuine_human(content: Option<&Value>) -> bool {
    let text = match content {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(blocks)) => {
            if blocks
                .iter()
                .any(|block| block.get("type").and_then(Value::as_str) == Some("tool_result"))
                || !blocks
                    .iter()
                    .any(|block| block.get("type").and_then(Value::as_str) == Some("text"))
            {
                return false;
            }
            blocks
                .iter()
                .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
                .map(|block| js_string(block.get("text")))
                .collect::<String>()
        }
        _ => return false,
    };
    let text = text.trim_start_matches(is_whitespace);
    !(text.starts_with("Stop hook feedback:")
        || text.starts_with("The AIDLC workflow has a pending step")
            && text.contains("workflow loop"))
}

#[cfg(test)]
mod tests {
    use super::{StopTranscript, classification_text, decode_line};

    #[test]
    fn classification_text_keeps_ordinary_escapes_and_finite_numbers_verbatim() {
        assert_eq!(
            classification_text(r#"{"a":"\u0041\n\"x\""}"#),
            r#"{"a":"\u0041\n\"x\""}"#
        );
        assert_eq!(
            classification_text(r#"{"n":-12.5e+3,"m":7}"#),
            r#"{"n":-12.5e+3,"m":7}"#
        );
        // 数字は文字列の中では触らない。
        assert_eq!(classification_text(r#"{"s":"1e999"}"#), r#"{"s":"1e999"}"#);
    }

    #[test]
    fn classification_text_neutralises_only_what_json_parse_would_accept_and_serde_refuses() {
        // 孤立サロゲートは置換文字へ、無限大になる数値は 0 へ。
        assert_eq!(
            classification_text(r#"{"s":"\ud800"}"#),
            r#"{"s":"\ufffd"}"#
        );
        assert_eq!(classification_text(r#"{"n":1e999}"#), r#"{"n":0}"#);
        // 文字列の途中で切れた逃がしは伸ばさない。
        assert_eq!(classification_text(r#"{"s":"a\"#), r#"{"s":"a"#);
    }

    #[test]
    fn decode_line_refuses_broken_json_even_when_neutralising_would_mend_it() {
        assert!(decode_line("").is_none());
        assert!(
            decode_line("{\"s\":\"\\ud800\"").is_none(),
            "閉じない object"
        );
        assert!(decode_line("nope").is_none());
        let mended = decode_line(r#"{"s":"\ud800","n":1e999}"#).expect("JSON.parse が受理する行");
        assert_eq!(
            mended.get("s").and_then(serde_json::Value::as_str),
            Some("\u{fffd}")
        );
        assert_eq!(mended.get("n").and_then(serde_json::Value::as_i64), Some(0));
    }

    #[test]
    fn a_user_entry_whose_content_is_neither_text_nor_blocks_is_not_a_human_turn() {
        for content in ["5", "null", "true", "{\"text\":\"hi\"}"] {
            let transcript =
                format!(r#"{{"type":"user","message":{{"role":"user","content":{content}}}}}"#);
            assert!(
                !StopTranscript::parse(&transcript).is_conversational(),
                "{content}"
            );
        }
        let human = r#"{"type":"user","message":{"role":"user","content":"hi"}}"#;
        assert!(StopTranscript::parse(human).is_conversational());
        // 行に message が無い・JSON でない行は読み飛ばす。
        let noisy = format!("{{\"type\":\"user\"}}\nnot json\n{human}");
        assert!(StopTranscript::parse(&noisy).is_conversational());
    }
}
