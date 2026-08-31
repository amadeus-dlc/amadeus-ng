//! `ProjectionName` — 投影の名前 (entities.md ProjectionName、C6 checkpoint.projection)。

use std::fmt;

/// 投影名の上限文字数 (C6 `checkpoint.projection`)。
const MAX_LEN: usize = 64;

/// 投影の名前 (Always Valid — 不正値はこの型に存在しない)。
///
/// 形は kebab (`^[a-z][a-z0-9-]*$`、1〜64 字)。例: `state-file` / `audit-shard`。
/// 正規化 (小文字化・空白除去) はしない — 受理か拒否のみで、チェックポイント表の
/// 主キーとしてそのまま使える形であることを型で保証する。
///
/// `Ord` は生文字列の辞書順。チェックポイントを名前で引く表の鍵になる。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectionName(String);

/// `ProjectionName::parse` が拒否する形 (材料のみ — 利用者向け文言はアダプタ層)。
///
/// **この型は他のエラーを内包しない** — 3 変種の材料 (長さ・位置) はすべて自分の `Display`
/// が描く。したがって `Error::source` の連鎖は無く、既定 (`None`) が正しい。常に `None` を
/// 返すだけの `source` は書かない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionNameError {
    /// 空文字列。
    Empty,
    /// 64 字を超える。
    Length {
        /// 実際の文字数。
        actual: usize,
    },
    /// 先頭が小文字英字でない、または `[a-z0-9-]` の外の文字がある。
    Format {
        /// 最初に形式へ合わなかった文字の 0 始まり位置。
        position: usize,
    },
}

impl ProjectionName {
    /// kebab (`^[a-z][a-z0-9-]*$`、1〜64 字) として検証する。正規化はしない。
    ///
    /// # Errors
    ///
    /// 空・64 字超過・先頭が小文字英字でない・`[a-z0-9-]` の外の文字を、それぞれ拒否する。
    pub fn parse(s: &str) -> Result<ProjectionName, ProjectionNameError> {
        if s.is_empty() {
            return Err(ProjectionNameError::Empty);
        }
        let actual = s.chars().count();
        if actual > MAX_LEN {
            return Err(ProjectionNameError::Length { actual });
        }
        for (position, c) in s.chars().enumerate() {
            let ok = if position == 0 {
                c.is_ascii_lowercase()
            } else {
                c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'
            };
            if !ok {
                return Err(ProjectionNameError::Format { position });
            }
        }
        Ok(ProjectionName(s.to_string()))
    }

    /// 検証済みの投影名。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProjectionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for ProjectionNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectionNameError::Empty => f.write_str("empty"),
            ProjectionNameError::Length { actual } => {
                write!(f, "length {actual} (max {MAX_LEN})")
            }
            ProjectionNameError::Format { position } => write!(f, "format at {position}"),
        }
    }
}

impl std::error::Error for ProjectionNameError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_projection_names_are_accepted_verbatim() {
        assert_eq!(
            ProjectionName::parse("state-file").unwrap().as_str(),
            "state-file"
        );
        assert_eq!(
            ProjectionName::parse("audit-shard").unwrap().as_str(),
            "audit-shard"
        );
    }

    #[test]
    fn a_single_letter_and_digits_after_the_head_are_accepted() {
        assert_eq!(ProjectionName::parse("a").unwrap().as_str(), "a");
        assert_eq!(ProjectionName::parse("a1-b2").unwrap().as_str(), "a1-b2");
    }

    #[test]
    fn the_empty_name_is_rejected() {
        assert_eq!(ProjectionName::parse(""), Err(ProjectionNameError::Empty));
    }

    #[test]
    fn a_name_longer_than_sixty_four_characters_is_rejected() {
        let long = "a".repeat(65);
        assert_eq!(
            ProjectionName::parse(&long),
            Err(ProjectionNameError::Length { actual: 65 })
        );
        assert!(ProjectionName::parse(&"a".repeat(64)).is_ok());
    }

    #[test]
    fn the_head_must_be_a_lowercase_letter() {
        assert_eq!(
            ProjectionName::parse("1abc"),
            Err(ProjectionNameError::Format { position: 0 })
        );
        assert_eq!(
            ProjectionName::parse("-abc"),
            Err(ProjectionNameError::Format { position: 0 })
        );
    }

    #[test]
    fn uppercase_underscore_and_whitespace_are_rejected_with_their_position() {
        assert_eq!(
            ProjectionName::parse("stateFile"),
            Err(ProjectionNameError::Format { position: 5 })
        );
        assert_eq!(
            ProjectionName::parse("state_file"),
            Err(ProjectionNameError::Format { position: 5 })
        );
        assert_eq!(
            ProjectionName::parse("state file"),
            Err(ProjectionNameError::Format { position: 5 })
        );
    }

    #[test]
    fn names_order_and_compare_by_value_so_they_can_key_a_checkpoint_map() {
        let a = ProjectionName::parse("audit-shard").unwrap();
        let b = ProjectionName::parse("state-file").unwrap();
        assert!(a < b);
        assert_eq!(a, ProjectionName::parse("audit-shard").unwrap());
    }

    #[test]
    fn the_display_is_the_raw_name() {
        assert_eq!(
            ProjectionName::parse("state-file").unwrap().to_string(),
            "state-file"
        );
        assert_eq!(
            ProjectionNameError::Format { position: 3 }.to_string(),
            "format at 3"
        );
    }

    #[test]
    fn every_rejection_renders_its_material_and_nothing_else() {
        // `Display` は材料だけを描く (利用者向け文言はアダプタ層の message-catalog —
        // coding-rules/error-handling.md)。3 変種すべての綴りを固定する。
        assert_eq!(ProjectionNameError::Empty.to_string(), "empty");
        assert_eq!(
            ProjectionNameError::Length { actual: 65 }.to_string(),
            "length 65 (max 64)"
        );
        assert_eq!(
            ProjectionNameError::Format { position: 0 }.to_string(),
            "format at 0"
        );
        // parse から返る値でも同じ綴りになること (材料が素通しであること)。
        assert_eq!(ProjectionName::parse("").unwrap_err().to_string(), "empty");
        assert_eq!(
            ProjectionName::parse(&"a".repeat(65))
                .unwrap_err()
                .to_string(),
            "length 65 (max 64)"
        );
    }
}
