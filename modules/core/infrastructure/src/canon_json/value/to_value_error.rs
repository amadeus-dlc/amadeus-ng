//! `to_value` が型付き値を `JsonValue` へ写せなかったときの理由。

use std::fmt;

/// `to_value` が型付き値を `JsonValue` へ写せなかったときの理由。
///
/// 文言はアダプタ層 (message-catalog) が付ける — 本型は材料だけを保持する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToValueError {
    /// serde の直列化が失敗した (非文字列キーのマップ、`Serialize` 実装のエラー等)。
    Serialization(String),
}

impl ToValueError {
    /// 失敗の詳細 (serde が返した文言)。
    #[must_use]
    pub fn detail(&self) -> &str {
        match self {
            ToValueError::Serialization(detail) => detail,
        }
    }
}

impl fmt::Display for ToValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToValueError::Serialization(detail) => {
                write!(f, "型付き値を JsonValue へ変換できない: {detail}")
            }
        }
    }
}

/// `?` で他のエラー型へ持ち上げられるようにする。`source()` は返さない —
/// 原因は serde が返した文言として畳み込んであるため。
impl std::error::Error for ToValueError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_value_error_exposes_detail_and_display() {
        let error = ToValueError::Serialization("key must be a string".to_string());

        assert_eq!(error.detail(), "key must be a string");
        assert_eq!(
            error.to_string(),
            "型付き値を JsonValue へ変換できない: key must be a string"
        );
    }
}
