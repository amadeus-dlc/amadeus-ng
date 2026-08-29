//! 直列化の本体 — 体裁 (BR1.5)・数値 (BR1.3)・エスケープ (BR1.4)・キー順 (BR1.1 / BR1.2)。

use crate::canon_json::canonical;
use crate::canon_json::profile::{Indent, SerializationProfile};
use crate::canon_json::value::{JsonValue, Number, ObjectMembers};

/// JS の `Number` が整数を正確に保てる上限。これを超える整数は JS 側が f64 に丸めてから
/// 表記するため、こちらも f64 経路で書かないとバイト一致しない (BR1.3)。
const MAX_EXACT_INTEGER: u64 = 1 << 53;

/// 小文字 16 進の桁 (`\uXXXX` エスケープ用)。
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// 値をプロファイルの規則で直列化する。
///
/// 同じ値・同じプロファイルなら常に同じバイト列を返す (乱数・時刻・環境に依存しない)。
/// 非有限数は `null` に落ちるため、この関数は失敗しない。
#[must_use]
pub fn serialize(value: &JsonValue, profile: SerializationProfile) -> String {
    let mut out = String::new();
    write_value(&mut out, value, profile, 0);
    if profile.trailing_newline() {
        out.push('\n');
    }
    out
}

fn write_value(out: &mut String, value: &JsonValue, profile: SerializationProfile, depth: usize) {
    match value {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(true) => out.push_str("true"),
        JsonValue::Bool(false) => out.push_str("false"),
        JsonValue::Number(number) => write_number(out, *number),
        JsonValue::String(text) => write_string(out, text),
        JsonValue::Array(items) => write_array(out, items, profile, depth),
        JsonValue::Object(members) => write_object(out, members, profile, depth),
    }
}

fn write_array(out: &mut String, items: &[JsonValue], profile: SerializationProfile, depth: usize) {
    if items.is_empty() {
        out.push_str("[]");
        return;
    }
    out.push('[');
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_break(out, profile, depth + 1);
        write_value(out, item, profile, depth + 1);
    }
    write_break(out, profile, depth);
    out.push(']');
}

fn write_object(
    out: &mut String,
    members: &ObjectMembers,
    profile: SerializationProfile,
    depth: usize,
) {
    if members.is_empty() {
        out.push_str("{}");
        return;
    }
    let entries: Vec<(&str, &JsonValue)> = members.iter().collect();
    let order = canonical::member_order(members, profile.key_order());

    out.push('{');
    for (i, index) in order.into_iter().enumerate() {
        // `order` の要素は必ず `entries` の有効な添字 (canonical::member_order が同じ
        // `members` から作る) — else 分岐には到達しない。
        let Some(&(key, value)) = entries.get(index) else {
            continue;
        };
        if i > 0 {
            out.push(',');
        }
        write_break(out, profile, depth + 1);
        write_string(out, key);
        out.push(':');
        if profile.indent() != Indent::None {
            out.push(' ');
        }
        write_value(out, value, profile, depth + 1);
    }
    write_break(out, profile, depth);
    out.push('}');
}

/// pretty のときだけ改行 + 深さ分のインデントを書く (compact / canonical では何もしない)。
fn write_break(out: &mut String, profile: SerializationProfile, depth: usize) {
    let unit = profile.indent().unit();
    if unit.is_empty() {
        return;
    }
    out.push('\n');
    for _ in 0..depth {
        out.push_str(unit);
    }
}

/// 数値の表記 (BR1.3)。2^53 を超える整数は JS と同じく f64 に丸めてから書く。
fn write_number(out: &mut String, number: Number) {
    match number {
        Number::PosInt(value) if value <= MAX_EXACT_INTEGER => {
            out.push_str(&value.to_string());
        }
        Number::PosInt(value) => write_f64(out, value as f64),
        Number::NegInt(value) if value.unsigned_abs() <= MAX_EXACT_INTEGER => {
            out.push_str(&value.to_string());
        }
        Number::NegInt(value) => write_f64(out, value as f64),
        Number::Float(value) => write_f64(out, value),
    }
}

/// ECMA-262 `Number::toString` 互換の f64 表記。
///
/// 最短往復表記の数字列 `s` (桁数 `k`) と 10 進位置 `n` (値 = `s × 10^(n-k)`) を
/// `{:e}` から取り出し、仕様の 4 規則で組み立てる。
fn write_f64(out: &mut String, value: f64) {
    if !value.is_finite() {
        out.push_str("null");
        return;
    }
    if value == 0.0 {
        // -0.0 == 0.0 なので負ゼロもここで '0' になる (BR1.3)。
        out.push('0');
        return;
    }
    if value < 0.0 {
        out.push('-');
    }

    let repr = format!("{:e}", value.abs());
    let (mantissa, exponent) = repr.split_once('e').unwrap_or((repr.as_str(), "0"));
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let n = exponent.parse::<i32>().unwrap_or(0) + 1;

    if k <= n && n <= 21 {
        out.push_str(&digits);
        out.push_str(&"0".repeat((n - k).unsigned_abs() as usize));
    } else if 0 < n && n <= 21 {
        let split = n.unsigned_abs() as usize;
        out.push_str(&digits[..split]);
        out.push('.');
        out.push_str(&digits[split..]);
    } else if -6 < n && n <= 0 {
        out.push_str("0.");
        out.push_str(&"0".repeat(n.unsigned_abs() as usize));
        out.push_str(&digits);
    } else {
        if k == 1 {
            out.push_str(&digits);
        } else {
            out.push_str(&digits[..1]);
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        let exp = n - 1;
        out.push(if exp >= 0 { '+' } else { '-' });
        out.push_str(&exp.unsigned_abs().to_string());
    }
}

/// 文字列の最小エスケープ (BR1.4)。`/`・U+007F・非 ASCII・U+2028 / U+2029 は生出力。
fn write_string(out: &mut String, text: &str) {
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if (control as u32) < 0x20 => {
                out.push_str("\\u");
                let code = control as u32;
                for shift in [12, 8, 4, 0] {
                    // ニブル (0..16) の変換なので添字は必ず範囲内 — `unwrap_or` の既定分岐には
                    // 到達しない。
                    out.push(
                        HEX_DIGITS
                            .get(((code >> shift) & 0xf) as usize)
                            .copied()
                            .unwrap_or(b'0') as char,
                    );
                }
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canon_json::value::ObjectMembers;

    fn compact(value: &JsonValue) -> String {
        serialize(value, SerializationProfile::ContractCompact)
    }

    fn num(n: Number) -> JsonValue {
        JsonValue::Number(n)
    }

    fn array(values: Vec<JsonValue>) -> JsonValue {
        JsonValue::Array(values)
    }

    fn object(pairs: Vec<(&str, JsonValue)>) -> JsonValue {
        let mut members = ObjectMembers::new();
        for (key, value) in pairs {
            members.insert(key, value);
        }
        JsonValue::Object(members)
    }

    #[test]
    fn integers_are_written_in_decimal() {
        assert_eq!(compact(&num(Number::PosInt(0))), "0");
        assert_eq!(compact(&num(Number::PosInt(42))), "42");
        assert_eq!(compact(&num(Number::NegInt(-42))), "-42");
        assert_eq!(
            compact(&num(Number::PosInt(9_007_199_254_740_992))),
            "9007199254740992",
            "2^53 ちょうどは整数経路のまま"
        );
    }

    #[test]
    fn integers_beyond_the_exact_range_take_the_float_path() {
        assert_eq!(
            compact(&num(Number::PosInt(9_007_199_254_740_993))),
            "9007199254740992",
            "2^53+1 は f64 に丸められる"
        );
        assert_eq!(
            compact(&num(Number::NegInt(-9_007_199_254_740_993))),
            "-9007199254740992"
        );
        assert_eq!(
            compact(&num(Number::PosInt(u64::MAX))),
            "18446744073709552000"
        );
        assert_eq!(
            compact(&num(Number::NegInt(i64::MIN))),
            "-9223372036854776000"
        );
        assert_eq!(
            compact(&num(Number::PosInt(9_223_372_036_854_775_807))),
            "9223372036854776000"
        );
    }

    #[test]
    fn float_integral_values_drop_the_decimal_point() {
        assert_eq!(compact(&num(Number::Float(1.0))), "1");
        assert_eq!(compact(&num(Number::Float(-3.0))), "-3");
        assert_eq!(compact(&num(Number::Float(100.0))), "100");
        assert_eq!(compact(&num(Number::Float(1e2))), "100");
    }

    #[test]
    fn exponent_thresholds_match_ecma262() {
        assert_eq!(compact(&num(Number::Float(1e20))), "100000000000000000000");
        assert_eq!(compact(&num(Number::Float(1e21))), "1e+21");
        assert_eq!(compact(&num(Number::Float(1e-6))), "0.000001");
        assert_eq!(compact(&num(Number::Float(1e-7))), "1e-7");
        assert_eq!(compact(&num(Number::Float(123e-20))), "1.23e-18");
        assert_eq!(compact(&num(Number::Float(1.5e300))), "1.5e+300");
        assert_eq!(
            compact(&num(Number::Float(f64::MAX))),
            "1.7976931348623157e+308"
        );
        assert_eq!(compact(&num(Number::Float(5e-324))), "5e-324");
        assert_eq!(compact(&num(Number::Float(-2.5e-7))), "-2.5e-7");
        assert_eq!(compact(&num(Number::Float(0.1))), "0.1");
        assert_eq!(compact(&num(Number::Float(-0.25))), "-0.25");
    }

    #[test]
    fn negative_zero_is_written_as_zero() {
        assert_eq!(compact(&num(Number::Float(-0.0))), "0");
        assert_eq!(compact(&num(Number::Float(0.0))), "0");
        assert_eq!(compact(&num(Number::PosInt(0))), "0");
    }

    #[test]
    fn non_finite_numbers_become_null() {
        assert_eq!(compact(&num(Number::Float(f64::NAN))), "null");
        assert_eq!(compact(&num(Number::Float(f64::INFINITY))), "null");
        assert_eq!(compact(&num(Number::Float(f64::NEG_INFINITY))), "null");
        assert_eq!(
            compact(&array(vec![
                num(Number::Float(f64::NAN)),
                num(Number::PosInt(1)),
            ])),
            "[null,1]",
            "配列要素は詰められない"
        );
    }

    #[test]
    fn only_the_minimal_escape_set_is_applied() {
        assert_eq!(compact(&JsonValue::String("a\"b".to_string())), r#""a\"b""#);
        assert_eq!(compact(&JsonValue::String("a\\b".to_string())), r#""a\\b""#);
        assert_eq!(compact(&JsonValue::String("a\nb".to_string())), r#""a\nb""#);
        assert_eq!(compact(&JsonValue::String("a\rb".to_string())), r#""a\rb""#);
        assert_eq!(compact(&JsonValue::String("a\tb".to_string())), r#""a\tb""#);
        assert_eq!(
            compact(&JsonValue::String("a\u{8}b".to_string())),
            r#""a\bb""#
        );
        assert_eq!(
            compact(&JsonValue::String("a\u{c}b".to_string())),
            r#""a\fb""#
        );
        assert_eq!(
            compact(&JsonValue::String("\u{0}\u{7}\u{1f}".to_string())),
            r#""\u0000\u0007\u001f""#,
            "短縮形の無い C0 は小文字 4 桁 hex"
        );
    }

    #[test]
    fn slash_del_and_line_separators_are_written_raw() {
        assert_eq!(compact(&JsonValue::String("a/b".to_string())), "\"a/b\"");
        assert_eq!(
            compact(&JsonValue::String("\u{7f}".to_string())),
            "\"\u{7f}\"",
            "U+007F (DEL) は C0 ではないのでエスケープしない"
        );
        assert_eq!(
            compact(&JsonValue::String("\u{2028}\u{2029}".to_string())),
            "\"\u{2028}\u{2029}\"",
            "行区切り U+2028 / U+2029 は生出力 (upstream 実測)"
        );
        assert_eq!(
            compact(&JsonValue::String("aあ漢字🎉".to_string())),
            "\"aあ漢字🎉\""
        );
    }

    #[test]
    fn compact_and_canonical_carry_no_whitespace() {
        let value = object(vec![
            ("z", num(Number::PosInt(1))),
            ("a", array(vec![JsonValue::Bool(true), JsonValue::Null])),
        ]);

        assert_eq!(compact(&value), r#"{"z":1,"a":[true,null]}"#);
        assert_eq!(
            serialize(&value, SerializationProfile::HashCanonical),
            r#"{"a":[true,null],"z":1}"#
        );
    }

    #[test]
    fn pretty_uses_two_spaces_and_a_trailing_newline() {
        let value = object(vec![
            ("z", object(vec![("b", num(Number::PosInt(1)))])),
            ("a", array(vec![num(Number::PosInt(1)), JsonValue::Null])),
        ]);

        assert_eq!(
            serialize(&value, SerializationProfile::ContractPretty),
            "{\n  \"z\": {\n    \"b\": 1\n  },\n  \"a\": [\n    1,\n    null\n  ]\n}\n"
        );
    }

    #[test]
    fn empty_containers_stay_on_one_line_in_every_profile() {
        let value = object(vec![("arr", array(vec![])), ("obj", object(vec![]))]);

        assert_eq!(compact(&value), r#"{"arr":[],"obj":{}}"#);
        assert_eq!(
            serialize(&value, SerializationProfile::ContractPretty),
            "{\n  \"arr\": [],\n  \"obj\": {}\n}\n"
        );
        assert_eq!(
            serialize(
                &JsonValue::Array(vec![]),
                SerializationProfile::ContractPretty
            ),
            "[]\n"
        );
        assert_eq!(
            serialize(
                &JsonValue::Object(ObjectMembers::new()),
                SerializationProfile::ContractPretty
            ),
            "{}\n"
        );
    }

    #[test]
    fn scalar_roots_and_key_escaping() {
        assert_eq!(compact(&JsonValue::Null), "null");
        assert_eq!(compact(&JsonValue::Bool(true)), "true");
        assert_eq!(compact(&JsonValue::Bool(false)), "false");
        assert_eq!(
            compact(&object(vec![("a\nb", JsonValue::Null)])),
            r#"{"a\nb":null}"#,
            "キーも値と同じ規則でエスケープする"
        );
    }

    #[test]
    fn key_order_rules_apply_per_profile() {
        let value = object(vec![
            ("b", num(Number::PosInt(1))),
            ("10", num(Number::PosInt(2))),
            ("2", num(Number::PosInt(3))),
            ("a", num(Number::PosInt(4))),
            ("0", num(Number::PosInt(5))),
        ]);

        assert_eq!(compact(&value), r#"{"0":5,"2":3,"10":2,"b":1,"a":4}"#);
        assert_eq!(
            serialize(&value, SerializationProfile::HashCanonical),
            r#"{"0":5,"2":3,"10":2,"a":4,"b":1}"#
        );
    }

    #[test]
    fn canonical_sorting_is_recursive() {
        let value = object(vec![(
            "z",
            object(vec![
                (
                    "b",
                    array(vec![object(vec![
                        ("y", JsonValue::Null),
                        ("x", JsonValue::Null),
                    ])]),
                ),
                ("a", JsonValue::Bool(true)),
            ]),
        )]);

        assert_eq!(
            serialize(&value, SerializationProfile::HashCanonical),
            r#"{"z":{"a":true,"b":[{"x":null,"y":null}]}}"#
        );
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::*;
    use crate::canon_json::value::arbitrary;

    proptest! {
        /// 決定性 — 同じ値・同じプロファイルなら常に同じバイト列 (NFR1.1)。
        #[test]
        fn serialization_is_deterministic(value in arbitrary::any_value()) {
            for profile in SerializationProfile::ALL {
                prop_assert_eq!(
                    serialize(&value, *profile),
                    serialize(&value.clone(), *profile)
                );
            }
        }

        /// hash-canonical 出力はメンバの挿入順に依存しない (正準化の要件)。
        #[test]
        fn hash_canonical_output_is_invariant_under_key_permutation(
            pairs in arbitrary::unique_pairs(),
            rotation in 0usize..8,
        ) {
            let mut rotated = pairs.clone();
            let len = rotated.len();
            if len > 0 {
                rotated.rotate_left(rotation % len);
            }

            prop_assert_eq!(
                serialize(&arbitrary::object_of(pairs), SerializationProfile::HashCanonical),
                serialize(&arbitrary::object_of(rotated), SerializationProfile::HashCanonical)
            );
        }

        /// 非有限数は出力に漏れず、必ず `null` になる (BR1.3)。
        #[test]
        fn non_finite_numbers_never_reach_the_output(value in arbitrary::any_value()) {
            for profile in SerializationProfile::ALL {
                let text = serialize(&value, *profile);
                prop_assert!(!text.contains("NaN"));
                prop_assert!(!text.contains("inf"));
                prop_assert!(!text.contains("Infinity"));
            }
        }

        /// 出力は常に整形式の JSON として読み戻せる (自己整合性)。
        #[test]
        fn every_profile_emits_reparsable_json(value in arbitrary::any_value()) {
            for profile in SerializationProfile::ALL {
                let text = serialize(&value, *profile);
                prop_assert!(
                    crate::canon_json::parse::parse(text.trim_end_matches('\n')).is_ok(),
                    "読み戻せない出力: {text:?}"
                );
            }
        }
    }
}
