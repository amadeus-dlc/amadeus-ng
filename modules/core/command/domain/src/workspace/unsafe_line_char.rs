//! `UnsafeLineChar` — `StateFieldValue::parse` が拒否した単一行安全でない文字。

use std::fmt;

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

impl fmt::Display for UnsafeLineChar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 材料のみ (利用者向け文言はアダプタ層)。不可視の文字なのでコードポイントで示す。
        write!(f, "unsafe line character: U+{:04X}", self.0 as u32)
    }
}

impl std::error::Error for UnsafeLineChar {}
