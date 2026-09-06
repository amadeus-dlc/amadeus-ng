//! `StageSlotsError` — 位置ごとの記録の列の不変条件・範囲を破った形。

use std::fmt;

use super::stage_index::StageIndex;

/// [`StageSlots`] の不変条件違反と、位置指定コマンドの範囲外
/// (材料のみ — 利用者向け文言はアダプタ層)。
///
/// [`StageSlots`]: super::StageSlots
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageSlotsError {
    /// 位置が 0 件 (実行は必ず 1 つ以上のステージを持つ)。
    Empty,
    /// 同じ slug が 2 回以上現れる (BR1.5 — ステージ参照の解決先が一意でなくなる)。
    DuplicateSlug {
        /// 文書順で最初に重複した slug。
        slug: String,
    },
    /// 位置指定コマンドが列の外を指した (別の実行の位置を渡された等)。
    OutOfRange {
        /// 指された位置。
        stage: StageIndex,
    },
}

impl fmt::Display for StageSlotsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StageSlotsError::Empty => f.write_str("empty stage slot list"),
            StageSlotsError::DuplicateSlug { slug } => {
                write!(f, "duplicate stage slug: {slug}")
            }
            StageSlotsError::OutOfRange { stage } => {
                write!(f, "stage index out of range: {stage}")
            }
        }
    }
}

impl std::error::Error for StageSlotsError {}

#[cfg(test)]
mod tests {
    use super::StageSlotsError;
    use crate::orchestration::stage_index::StageIndex;

    #[test]
    fn every_violation_renders_its_material() {
        assert_eq!(StageSlotsError::Empty.to_string(), "empty stage slot list");
        assert_eq!(
            StageSlotsError::DuplicateSlug {
                slug: "intent-capture".to_string(),
            }
            .to_string(),
            "duplicate stage slug: intent-capture"
        );
        assert_eq!(
            StageSlotsError::OutOfRange {
                stage: StageIndex::new(7),
            }
            .to_string(),
            "stage index out of range: 7"
        );
    }

    #[test]
    fn the_violation_is_a_std_error() {
        let error: &dyn std::error::Error = &StageSlotsError::Empty;
        assert!(error.source().is_none(), "材料を自分で持つので連鎖しない");
    }

    #[test]
    fn violations_compare_by_value() {
        assert_eq!(StageSlotsError::Empty, StageSlotsError::Empty);
        assert_ne!(
            StageSlotsError::Empty,
            StageSlotsError::OutOfRange {
                stage: StageIndex::new(0),
            }
        );
    }
}
