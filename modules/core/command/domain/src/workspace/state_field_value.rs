//! `StateFieldValue` — 状態ファイルの単一行フィールド値。コードポイント走査で C0 制御・DEL・
//! U+2028・U+2029 を拒否する (upstream `aidlc-lib.ts:6436-6448`, 03 §5.2)。

use std::fmt;

use serde::{Deserialize, Serialize};

/// 単一行が保証されたフィールド値 (Always Valid — 行を割れる文字はこの型に存在せず、
/// 第二のフィールド行の偽造が不能)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// 復号は `parse` を通す — 直列化の口から不正な値が型へ入り込むのを防ぐ
// (`StageSlug` と同じ house pattern)。
#[serde(try_from = "String")]
pub struct StateFieldValue(String);

/// 拒否理由 — 走査順に**最初に**見つかった不正コードポイント 1 文字。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsafeLineChar(char);

impl UnsafeLineChar {
    /// 拒否のきっかけになったコードポイントから構成する。
    #[must_use]
    pub const fn new(value: char) -> UnsafeLineChar {
        UnsafeLineChar(value)
    }

    /// 拒否のきっかけになったコードポイント。
    #[must_use]
    pub const fn to_char(&self) -> char {
        self.0
    }
}

/// 拒否対象の判定 (upstream `hasUnsafeSingleLineCharacter` と同一集合)。
#[must_use]
pub fn unsafe_line_char(s: &str) -> Option<char> {
    s.chars()
        .find(|&c| (c as u32) <= 0x1f || c as u32 == 0x7f || c == '\u{2028}' || c == '\u{2029}')
}

impl StateFieldValue {
    /// # Errors
    ///
    /// C0 制御・DEL・U+2028・U+2029 を含む値は `UnsafeLineChar` で拒否する。
    pub fn parse(s: &str) -> Result<StateFieldValue, UnsafeLineChar> {
        match unsafe_line_char(s) {
            Some(c) => Err(UnsafeLineChar::new(c)),
            None => Ok(StateFieldValue(s.to_string())),
        }
    }

    /// 検証済みの値 — そのままフィールド行に書ける (エスケープ不要)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UnsafeLineChar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 材料のみ (利用者向け文言はアダプタ層)。不可視の文字なのでコードポイントで示す。
        write!(f, "unsafe line character: U+{:04X}", self.0 as u32)
    }
}

impl std::error::Error for UnsafeLineChar {}

impl TryFrom<String> for StateFieldValue {
    type Error = UnsafeLineChar;

    fn try_from(value: String) -> Result<StateFieldValue, UnsafeLineChar> {
        StateFieldValue::parse(&value)
    }
}

impl fmt::Display for StateFieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn rejects_c0_del_and_unicode_line_separators() {
        assert!(StateFieldValue::parse("line1\nline2").is_err());
        assert!(StateFieldValue::parse("a\tb").is_err());
        assert!(StateFieldValue::parse("a\u{7f}b").is_err());
        assert!(StateFieldValue::parse("a\u{2028}b").is_err());
        assert!(StateFieldValue::parse("a\u{2029}b").is_err());
    }

    #[test]
    fn the_rejection_names_the_first_offending_code_point() {
        // 走査順に**最初に**見つかった 1 文字が拒否理由として返る
        assert_eq!(
            StateFieldValue::parse("a\tb\nc").unwrap_err().to_char(),
            '\t'
        );
        assert_eq!(
            StateFieldValue::parse("a\u{2028}b").unwrap_err(),
            UnsafeLineChar::new('\u{2028}')
        );
        assert_eq!(
            StateFieldValue::parse("a\u{7f}").unwrap_err().to_char(),
            '\u{7f}'
        );
    }

    #[test]
    fn accepts_ordinary_single_line_values_including_unicode() {
        assert!(StateFieldValue::parse("Requirements Analysis — done ✅").is_ok());
        assert!(StateFieldValue::parse("").is_ok());
    }

    proptest! {
        /// パースに成功した値は改行類を一切含まない (行偽造不能)。
        #[test]
        fn parsed_values_never_contain_line_breaks(s in "\\PC*") {
            if let Ok(v) = StateFieldValue::parse(&s) {
                let clean = unsafe_line_char(v.as_str()).is_none()
                    && !v.as_str().contains('\n')
                    && !v.as_str().contains('\r');
                prop_assert!(clean);
            }
        }
    }

    #[test]
    fn the_value_and_its_rejection_render_themselves() {
        assert_eq!(
            StateFieldValue::parse("one line").unwrap().to_string(),
            "one line"
        );
        // 不可視の文字なのでコードポイントで示す (材料のみ)。
        let rejected = StateFieldValue::parse("two\nlines").unwrap_err();
        assert_eq!(rejected.to_string(), "unsafe line character: U+000A");
        let boxed: Box<dyn std::error::Error> = Box::new(rejected);
        assert_eq!(boxed.to_string(), "unsafe line character: U+000A");
    }

    #[test]
    fn the_decode_goes_through_parse() {
        assert_eq!(
            StateFieldValue::try_from("ok".to_string()).unwrap(),
            StateFieldValue::parse("ok").unwrap()
        );
        assert!(StateFieldValue::try_from("bad\u{2028}".to_string()).is_err());
    }
}
