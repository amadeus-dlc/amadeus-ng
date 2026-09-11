//! ECMAScriptの文字列空白集合。Unicode White_Spaceとは異なる言語上の定義。
/// WhiteSpace と LineTerminator の和集合。
#[must_use]
pub const fn is_whitespace(character: char) -> bool {
    matches!(character, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}
/// String.prototype.trimと同じ空白だけを両端から除く。
#[must_use]
pub fn trim(text: &str) -> &str {
    text.trim_matches(is_whitespace)
}
/// 連続するECMAScript空白を1個のASCII空白にする。両端の空白も1個残す。
#[must_use]
pub fn collapse_whitespace(text: &str) -> String {
    let mut output = String::new();
    let mut was_whitespace = false;
    for character in text.chars() {
        let whitespace = is_whitespace(character);
        if !whitespace {
            output.push(character);
        } else if !was_whitespace {
            output.push(' ');
        }
        was_whitespace = whitespace;
    }
    output
}
/// String.prototype.trimEndと同じ空白だけを末尾から除く。
#[must_use]
pub fn trim_end(text: &str) -> &str {
    text.trim_end_matches(is_whitespace)
}

/// ECMAScriptの配列インデックス上限。
const MAX_ARRAY_INDEX: u64 = (u32::MAX as u64) - 1;

/// キーが ECMAScript の配列インデックス (integer-like) なら、その数値。
///
/// 正準十進表記に限る — 先頭ゼロ (`01`)・符号 (`+1` / `-1`)・小数点 (`1.0`)・指数はいずれも
/// integer-like ではない。`0` だけは唯一の 1 文字ゼロとして許す。
#[must_use]
pub fn array_index(key: &str) -> Option<u64> {
    if key == "0" {
        return Some(0);
    }
    if key.is_empty() || key.starts_with('0') || key.len() > 10 {
        return None;
    }
    if !key.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let value: u64 = key.parse().ok()?;
    if value <= MAX_ARRAY_INDEX {
        Some(value)
    } else {
        None
    }
}
