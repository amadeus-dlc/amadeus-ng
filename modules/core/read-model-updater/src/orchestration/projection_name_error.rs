//! `ProjectionName::parse` が拒否する形 (C6 checkpoint.projection)。

use std::fmt;

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

impl fmt::Display for ProjectionNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectionNameError::Empty => f.write_str("empty"),
            ProjectionNameError::Length { actual } => {
                write!(
                    f,
                    "length {actual} (max {})",
                    super::projection_name::MAX_LEN
                )
            }
            ProjectionNameError::Format { position } => write!(f, "format at {position}"),
        }
    }
}

impl std::error::Error for ProjectionNameError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::ProjectionName;

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
