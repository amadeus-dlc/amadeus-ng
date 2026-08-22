//! JSON 読取の唯一の境界 (W3 / NFR4.3) — 挿入順保持・深さ上限・エラー変種。

use std::fmt;

use crate::value::{JsonValue, Number};

/// ネスト深さの上限 (NFR4.3)。`serde_json` 既定の再帰上限と同じ値。
///
/// **受け入れる最大段数は `MAX_DEPTH - 1` = 127 段**である。`serde_json` は 128 段目で
/// 自前の再帰エラーを返すため、その 1 段手前でこちらが弾くことで、深すぎる入力は必ず
/// [`ParseError::TooDeep`] になり `serde_json` の再帰エラーが表に出ない (決定的な拒否)。
/// `serde_json::Deserializer::disable_recursion_limit` は使わず、上限の数値を
/// このクレートが 1 か所で持つ (スタック枯渇の防止)。
pub const MAX_DEPTH: usize = 128;

/// 読取が失敗した理由。文言はアダプタ層 (message-catalog) が付ける — 本型は材料だけを保持する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// JSON 文法違反。`offset` は入力先頭からのバイト位置。
    Syntax {
        /// 入力先頭からのバイト位置。
        offset: usize,
        /// パーサが返した理由 (診断用)。
        detail: String,
    },
    /// ネスト深さが上限を超えた。
    TooDeep {
        /// 適用された上限 ([`MAX_DEPTH`])。
        limit: usize,
    },
    /// 入力が整形式の UTF-8 ではない。
    Encoding,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Syntax { offset, detail } => {
                write!(f, "JSON 文法違反 (バイト位置 {offset}): {detail}")
            }
            ParseError::TooDeep { limit } => write!(f, "ネスト深さが上限 {limit} を超えた"),
            ParseError::Encoding => f.write_str("入力が整形式の UTF-8 ではない"),
        }
    }
}

/// JSON テキストを読み、オブジェクトのキー順 (挿入順) を保った [`JsonValue`] を返す。
///
/// 読取はこの境界 1 か所に集約する — 呼出側はここを通った値だけを扱い、内部で再検証しない。
///
/// # Errors
///
/// 文法違反は [`ParseError::Syntax`]、ネスト深さが [`MAX_DEPTH`] を超える入力は
/// [`ParseError::TooDeep`] を返す。深さは `serde_json` に渡す前に決定的に判定する。
pub fn parse(text: &str) -> Result<JsonValue, ParseError> {
    check_depth(text)?;
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|error| ParseError::Syntax {
            offset: byte_offset(text, error.line(), error.column()),
            detail: error.to_string(),
        })?;
    Ok(from_serde(value))
}

/// バイト列を JSON テキストとして読む。
///
/// # Errors
///
/// 整形式の UTF-8 でなければ [`ParseError::Encoding`]。以降は [`parse`] と同じ。
pub fn parse_bytes(bytes: &[u8]) -> Result<JsonValue, ParseError> {
    let text = std::str::from_utf8(bytes).map_err(|_| ParseError::Encoding)?;
    parse(text)
}

/// 文字列リテラルの外側で `{` / `[` を数え、[`MAX_DEPTH`] 段目に達した時点で弾く。
fn check_depth(text: &str) -> Result<(), ParseError> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for byte in text.bytes() {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                if depth >= MAX_DEPTH {
                    return Err(ParseError::TooDeep { limit: MAX_DEPTH });
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

/// `serde_json` の 1 始まり line / column をバイト位置へ写す。
fn byte_offset(text: &str, line: usize, column: usize) -> usize {
    let mut start_of_line = 0usize;
    for (index, current) in text.split_inclusive('\n').enumerate() {
        if index + 1 == line {
            return (start_of_line + column.saturating_sub(1)).min(text.len());
        }
        start_of_line += current.len();
    }
    text.len()
}

/// `serde_json::Value` を挿入順を保ったまま [`JsonValue`] へ写す。
fn from_serde(value: serde_json::Value) -> JsonValue {
    match value {
        serde_json::Value::Null => JsonValue::Null,
        serde_json::Value::Bool(flag) => JsonValue::Bool(flag),
        serde_json::Value::Number(number) => JsonValue::Number(number_from_serde(&number)),
        serde_json::Value::String(text) => JsonValue::String(text),
        serde_json::Value::Array(items) => {
            JsonValue::Array(items.into_iter().map(from_serde).collect())
        }
        serde_json::Value::Object(map) => JsonValue::Object(
            map.into_iter()
                .map(|(key, child)| (key, from_serde(child)))
                .collect(),
        ),
    }
}

/// `serde_json` の数値表現を [`Number`] へ写す (非負は u64 優先、負は i64、残りは f64)。
fn number_from_serde(number: &serde_json::Number) -> Number {
    if let Some(value) = number.as_u64() {
        Number::PosInt(value)
    } else if let Some(value) = number.as_i64() {
        Number::NegInt(value)
    } else {
        Number::Float(number.as_f64().unwrap_or(f64::NAN))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_display_reports_offset_and_detail() {
        let error = ParseError::Syntax {
            offset: 7,
            detail: "expected value".to_string(),
        };

        assert_eq!(
            error.to_string(),
            "JSON 文法違反 (バイト位置 7): expected value"
        );
    }

    #[test]
    fn too_deep_display_reports_the_limit() {
        let error = ParseError::TooDeep { limit: MAX_DEPTH };

        assert_eq!(error.to_string(), "ネスト深さが上限 128 を超えた");
    }

    #[test]
    fn encoding_display_is_fixed() {
        assert_eq!(
            ParseError::Encoding.to_string(),
            "入力が整形式の UTF-8 ではない"
        );
    }

    #[test]
    fn max_depth_is_the_serde_json_default() {
        assert_eq!(MAX_DEPTH, 128);
    }

    #[test]
    fn variants_carry_distinguishable_material() {
        let a = ParseError::Syntax {
            offset: 1,
            detail: "x".to_string(),
        };
        let b = ParseError::Syntax {
            offset: 2,
            detail: "x".to_string(),
        };

        assert_ne!(a, b, "offset は同値関係に含まれる");
        assert_ne!(a, ParseError::Encoding);
        assert_ne!(ParseError::TooDeep { limit: 128 }, ParseError::Encoding);
    }
}

#[cfg(test)]
mod parse_tests {
    use super::*;
    use crate::value::ObjectMembers;

    fn object_keys(value: &JsonValue) -> Vec<String> {
        match value {
            JsonValue::Object(members) => members.iter().map(|(k, _)| k.to_string()).collect(),
            other => panic!("object を期待したが {other:?}"),
        }
    }

    #[test]
    fn objects_keep_their_insertion_order() {
        let value = parse(r#"{"z":1,"a":2,"m":3}"#).unwrap();

        assert_eq!(object_keys(&value), vec!["z", "a", "m"]);
    }

    #[test]
    fn duplicate_keys_are_last_wins_at_the_first_position() {
        let value = parse(r#"{"a":1,"b":2,"a":3}"#).unwrap();

        assert_eq!(object_keys(&value), vec!["a", "b"]);
        let JsonValue::Object(members) = &value else {
            panic!("object を期待した");
        };
        assert_eq!(
            members.get("a"),
            Some(&JsonValue::Number(Number::PosInt(3)))
        );
    }

    #[test]
    fn numbers_prefer_unsigned_then_signed_then_float() {
        let value = parse("[0,42,-42,1.0,-0.0,18446744073709551615]").unwrap();
        let JsonValue::Array(items) = value else {
            panic!("array を期待した");
        };

        assert_eq!(items[0], JsonValue::Number(Number::PosInt(0)));
        assert_eq!(items[1], JsonValue::Number(Number::PosInt(42)));
        assert_eq!(items[2], JsonValue::Number(Number::NegInt(-42)));
        assert_eq!(items[3], JsonValue::Number(Number::Float(1.0)));
        assert_eq!(items[4], JsonValue::Number(Number::Float(-0.0)));
        assert_eq!(
            items[5],
            JsonValue::Number(Number::PosInt(18_446_744_073_709_551_615))
        );
    }

    #[test]
    fn scalars_and_containers_round_trip_into_the_value_tree() {
        assert_eq!(parse("null").unwrap(), JsonValue::Null);
        assert_eq!(parse("true").unwrap(), JsonValue::Bool(true));
        assert_eq!(
            parse(r#""あ""#).unwrap(),
            JsonValue::String("あ".to_string())
        );
        assert_eq!(parse("[]").unwrap(), JsonValue::Array(Vec::new()));
        assert_eq!(
            parse("{}").unwrap(),
            JsonValue::Object(ObjectMembers::new())
        );
    }

    #[test]
    fn syntax_errors_report_a_byte_offset() {
        let text = r#"{"a":}"#;
        let want = text.find('}').unwrap();

        match parse(text).unwrap_err() {
            ParseError::Syntax { offset, detail } => {
                assert_eq!(offset, want, "値の来るべき位置にある `}}` を指す");
                assert!(!detail.is_empty(), "理由が空");
            }
            other => panic!("Syntax を期待したが {other:?}"),
        }
    }

    #[test]
    fn syntax_error_offsets_account_for_earlier_lines() {
        let text = "{\n  \"a\": ,\n}";

        let want = text.find(',').unwrap();

        match parse(text).unwrap_err() {
            ParseError::Syntax { offset, .. } => {
                assert_eq!(offset, want, "先行行のバイト数を足した位置を指す");
            }
            other => panic!("Syntax を期待したが {other:?}"),
        }
    }

    fn nested(depth: usize) -> String {
        format!("{}{}", "[".repeat(depth), "]".repeat(depth))
    }

    #[test]
    fn nesting_just_below_the_limit_is_accepted() {
        assert!(parse(&nested(MAX_DEPTH - 1)).is_ok(), "127 段は読める");
    }

    #[test]
    fn nesting_at_or_beyond_the_limit_is_rejected_before_serde() {
        for depth in [MAX_DEPTH, MAX_DEPTH + 1, MAX_DEPTH * 2] {
            assert_eq!(
                parse(&nested(depth)).unwrap_err(),
                ParseError::TooDeep { limit: MAX_DEPTH },
                "{depth} 段は serde_json の再帰エラーではなく TooDeep で弾く"
            );
        }
    }

    #[test]
    fn brackets_inside_strings_do_not_count_toward_depth() {
        let text = format!(r#"["{}"]"#, "[".repeat(MAX_DEPTH * 2));

        assert!(
            parse(&text).is_ok(),
            "文字列リテラル内の括弧は深さに数えない"
        );
    }

    #[test]
    fn escaped_characters_inside_strings_do_not_confuse_the_depth_scan() {
        // 末尾がエスケープされたバックスラッシュの文字列。走査が `\\` を 1 文字として
        // 扱えないと、閉じ引用符を見落として以降の構造を文字列内と誤認する。
        let text = r#"["a\\",{"b":[1]},"c\"d"]"#;

        let value = parse(text).unwrap();
        let JsonValue::Array(items) = value else {
            panic!("array を期待した");
        };
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], JsonValue::String("a\\".to_string()));
        assert_eq!(items[2], JsonValue::String("c\"d".to_string()));
    }

    #[test]
    fn parse_bytes_rejects_invalid_utf8() {
        assert_eq!(
            parse_bytes(&[0x22, 0xff, 0x22]).unwrap_err(),
            ParseError::Encoding
        );
    }

    #[test]
    fn parse_bytes_accepts_valid_utf8() {
        assert_eq!(
            parse_bytes(r#"{"あ":1}"#.as_bytes()).unwrap(),
            parse(r#"{"あ":1}"#).unwrap()
        );
    }

    #[test]
    fn lone_surrogate_escapes_are_rejected_as_syntax_errors() {
        let error = parse(r#""\ud800""#).unwrap_err();

        assert!(
            matches!(error, ParseError::Syntax { .. }),
            "孤立サロゲートは読取段階で拒否される (既知の非対称)"
        );
    }
}
