//! `DefinitionIdError` — [`DefinitionIdView::parse`] が拒否した形。
//!
//! 運ぶのは**材料だけ** (どの制御文字が混ざっていたか) で、利用者向けの逐語文言は出す側が
//! 組む (`coding-rules/error-handling.md`)。
//!
//! [`DefinitionIdView::parse`]: super::DefinitionIdView::parse

use std::fmt;

/// `DefinitionIdView::parse` が拒否する形。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinitionIdError {
    /// 前後の空白を除くと空になる。
    Empty,
    /// 制御文字を含む。
    ControlCharacter(char),
}

impl fmt::Display for DefinitionIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DefinitionIdError::Empty => f.write_str("empty"),
            DefinitionIdError::ControlCharacter(c) => {
                write!(f, "control character U+{:04X}", u32::from(*c))
            }
        }
    }
}

impl std::error::Error for DefinitionIdError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rejection_carries_material_not_wording() {
        assert_eq!(DefinitionIdError::Empty.to_string(), "empty");
        assert_eq!(
            DefinitionIdError::ControlCharacter('\u{7}').to_string(),
            "control character U+0007"
        );
    }
}
