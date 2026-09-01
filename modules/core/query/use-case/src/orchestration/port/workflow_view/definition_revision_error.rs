//! `DefinitionRevisionError` — [`DefinitionRevisionView::parse`] が拒否した形。
//!
//! 運ぶのは**材料だけ** (実際の桁数・違反した文字) で、利用者向けの逐語文言は出す側が組む
//! (`coding-rules/error-handling.md`)。
//!
//! [`DefinitionRevisionView::parse`]: super::DefinitionRevisionView::parse

use std::fmt;

use super::definition_revision_view::HEX_LEN;

/// `DefinitionRevisionView::parse` が拒否する形。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinitionRevisionError {
    /// `sha256:` 接頭辞が無い (生 hex の非正準族ダイジェストを取り違えた場合を含む)。
    MissingPrefix,
    /// 接頭辞の後ろが 16 進 64 桁ではない。
    InvalidLength {
        /// 実際の桁数。
        actual: usize,
    },
    /// 16 進小文字 (`0-9a-f`) 以外の文字を含む。大文字 hex もここで落ちる。
    InvalidHexDigit(char),
}

impl fmt::Display for DefinitionRevisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DefinitionRevisionError::MissingPrefix => f.write_str("missing sha256: prefix"),
            DefinitionRevisionError::InvalidLength { actual } => {
                write!(f, "expected {HEX_LEN} hex digits, got {actual}")
            }
            DefinitionRevisionError::InvalidHexDigit(c) => {
                write!(f, "not a lowercase hex digit: {c:?}")
            }
        }
    }
}

impl std::error::Error for DefinitionRevisionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rejection_carries_material_not_wording() {
        assert_eq!(
            DefinitionRevisionError::MissingPrefix.to_string(),
            "missing sha256: prefix"
        );
        assert_eq!(
            DefinitionRevisionError::InvalidLength { actual: 3 }.to_string(),
            "expected 64 hex digits, got 3"
        );
        assert_eq!(
            DefinitionRevisionError::InvalidHexDigit('A').to_string(),
            "not a lowercase hex digit: 'A'"
        );
    }
}
