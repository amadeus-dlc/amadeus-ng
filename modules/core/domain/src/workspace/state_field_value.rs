//! `StateFieldValue` — 状態ファイルの単一行フィールド値。コードポイント走査で C0 制御・DEL・
//! U+2028・U+2029 を拒否する (upstream `aidlc-lib.ts:6436-6448`, 03 §5.2)。

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateFieldValue(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsafeLineChar(pub char);

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
            Some(c) => Err(UnsafeLineChar(c)),
            None => Ok(StateFieldValue(s.to_string())),
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
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
}
