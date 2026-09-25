//! `ParseError` — JsonValue への読取 ([`super::parse`] / [`super::parse_bytes`]) が失敗した理由。

use std::fmt;

#[cfg(doc)]
use super::MAX_DEPTH;

/// 読取が失敗した理由。利用者向け文言は呼出側が組み立て、本型は診断用の材料を保持する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// JSON 文法違反。`offset` は入力先頭からのバイト位置。
    Syntax {
        /// 入力先頭からのバイト位置。
        offset: usize,
        /// パーサが返した理由 (診断用)。
        detail: String,
    },
    /// ネスト深さが上限に達した。
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

/// `?` で他のエラー型へ持ち上げられるようにする。`source()` は返さない —
/// 原因は `detail` に文字列として畳み込んであり、連鎖させる内部エラーを保持しないため。
impl std::error::Error for ParseError {}
