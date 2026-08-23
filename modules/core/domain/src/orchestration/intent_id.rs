//! `IntentId` — 集約 `WorkflowExecution` の識別子 (entities.md IntentId)。

use std::fmt;

/// intent の記録ディレクトリ名にあたる集約識別子 (Always Valid — 不正値はこの型に存在しない)。
///
/// 形は **kebab 表記** (`[a-z0-9]` の 1 文字以上からなる区間を `-` で連結) に限る。
/// `Ord` は生文字列の辞書順。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IntentId(String);

/// `IntentId::parse` が拒否する形。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentIdError {
    /// 前後の空白を除くと空になる。
    Empty,
    /// `-` で区切った区間が空 (先頭・末尾の `-`、`--` の連続)。位置は 0 始まりの区間番号。
    EmptySegment {
        /// 空だった区間の 0 始まり位置。
        position: usize,
    },
    /// `[a-z0-9-]` 以外の文字を含む。
    InvalidChar(char),
}

impl IntentId {
    /// 前後の空白を落としてから検証する。
    ///
    /// # Errors
    ///
    /// 空・空区間・`[a-z0-9-]` 以外の文字を拒否する。
    pub fn parse(s: &str) -> Result<IntentId, IntentIdError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(IntentIdError::Empty);
        }
        if let Some(c) = trimmed
            .chars()
            .find(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-'))
        {
            return Err(IntentIdError::InvalidChar(c));
        }
        for (position, segment) in trimmed.split('-').enumerate() {
            if segment.is_empty() {
                return Err(IntentIdError::EmptySegment { position });
            }
        }
        Ok(IntentId(trimmed.to_string()))
    }

    /// 生の識別子文字列 (trim 済み)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IntentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for IntentIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntentIdError::Empty => f.write_str("empty"),
            IntentIdError::EmptySegment { position } => {
                write!(f, "empty segment at position {position}")
            }
            IntentIdError::InvalidChar(c) => write!(f, "invalid character '{c}'"),
        }
    }
}

impl std::error::Error for IntentIdError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashSet};

    #[test]
    fn parse_accepts_the_record_directory_name() {
        for raw in [
            "260822-stage1-selfhost",
            "stage1-selfhost-a1b2c3d4",
            "u2",
            "0f",
        ] {
            let id = IntentId::parse(raw).unwrap();
            assert_eq!(id.as_str(), raw);
            assert_eq!(id.to_string(), raw);
        }
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_before_validation() {
        let id = IntentId::parse("  260822-stage1-selfhost\n").unwrap();
        assert_eq!(id.as_str(), "260822-stage1-selfhost");
        assert_eq!(id, IntentId::parse("260822-stage1-selfhost").unwrap());
    }

    #[test]
    fn an_empty_or_blank_value_cannot_be_constructed() {
        assert_eq!(IntentId::parse(""), Err(IntentIdError::Empty));
        assert_eq!(IntentId::parse("  \t\n"), Err(IntentIdError::Empty));
    }

    #[test]
    fn a_segment_may_not_be_empty() {
        assert_eq!(
            IntentId::parse("a--b"),
            Err(IntentIdError::EmptySegment { position: 1 })
        );
        assert_eq!(
            IntentId::parse("-abc"),
            Err(IntentIdError::EmptySegment { position: 0 })
        );
        assert_eq!(
            IntentId::parse("abc-"),
            Err(IntentIdError::EmptySegment { position: 1 })
        );
    }

    #[test]
    fn only_lowercase_alphanumerics_and_the_separator_are_allowed() {
        assert_eq!(
            IntentId::parse("Stage1"),
            Err(IntentIdError::InvalidChar('S'))
        );
        assert_eq!(
            IntentId::parse("stage_1"),
            Err(IntentIdError::InvalidChar('_'))
        );
        assert_eq!(
            IntentId::parse("stage 1"),
            Err(IntentIdError::InvalidChar(' '))
        );
    }

    #[test]
    fn ordering_is_the_lexicographic_order_of_the_raw_string() {
        let mut sorted: Vec<IntentId> = ["b-2", "a-1", "c-3"]
            .iter()
            .map(|s| IntentId::parse(s).unwrap())
            .collect();
        sorted.sort();
        let raw: Vec<&str> = sorted.iter().map(IntentId::as_str).collect();
        assert_eq!(raw, ["a-1", "b-2", "c-3"]);
    }

    #[test]
    fn the_id_works_as_a_map_and_set_key() {
        let a = IntentId::parse("u2-aggregate").unwrap();
        let b = IntentId::parse(" u2-aggregate ").unwrap();
        let mut hashed = HashSet::new();
        hashed.insert(a.clone());
        assert!(hashed.contains(&b));
        let ordered: BTreeSet<IntentId> = [a, b].into_iter().collect();
        assert_eq!(ordered.len(), 1);
    }

    #[test]
    fn the_rejection_carries_material_not_wording() {
        assert_eq!(IntentIdError::Empty.to_string(), "empty");
        assert_eq!(
            IntentIdError::EmptySegment { position: 2 }.to_string(),
            "empty segment at position 2"
        );
        assert_eq!(
            IntentIdError::InvalidChar('_').to_string(),
            "invalid character '_'"
        );
    }
}
