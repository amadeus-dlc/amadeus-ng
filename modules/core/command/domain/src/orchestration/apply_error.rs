//! `ApplyError` — 適用の内部配管が運ぶ失敗材料 (クレート内私有)。
//!
//! `apply_event` は**失敗を返さない** (オーナー裁定 2026-08-30 — 壊れた歴史はクラッシュが正)。
//! 本型は `mutate` / `resolve` などの内部ヘルパが検出箇所から境界 (`apply_event` の
//! `unwrap_or_else(panic!)`) まで材料を運ぶための配管であり、公開 API には現れない。
//! 通番の検査 (`SequenceGap` / `SequenceExhausted`) と intent 照合は `apply_event` 自身が
//! assert で行うため、変種は検出ヘルパが使う 2 つだけである。

use std::fmt;

use super::stage_slots_error::StageSlotsError;
use crate::workflow_definition::StageSlug;

/// 適用ヘルパの失敗材料 (クラッシュ文言の部品になる)。適用前の状態は保たれる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ApplyError {
    /// ペイロードのステージ slug が `stages` に無い。
    UnknownStage(StageSlug),
    /// 適用後に集約不変条件が破れた (材料は不変条件名)。
    InvariantViolation(String),
}

impl fmt::Display for ApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApplyError::UnknownStage(slug) => write!(f, "unknown stage: {slug}"),
            ApplyError::InvariantViolation(reason) => {
                write!(f, "invariant violation: {reason}")
            }
        }
    }
}

impl std::error::Error for ApplyError {}

impl From<StageSlotsError> for ApplyError {
    /// 位置ごとの記録の列が拒んだ操作を、適用経路の失敗材料へ写す。
    ///
    /// 適用が呼ぶ位置は `resolve` / 区間集合が束縛済みなので `OutOfRange` は起きない。
    /// それでも変換を持つのは、起きたときに**壊れた歴史**として `apply_event` の panic
    /// 経路まで材料ごと届かせるためである (無言の no-op にしない)。
    fn from(error: StageSlotsError) -> ApplyError {
        ApplyError::InvariantViolation(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_definition::StageSlug;

    #[test]
    fn the_unknown_stage_carries_the_slug() {
        let err = ApplyError::UnknownStage(StageSlug::parse("no-such-stage").unwrap());
        assert_eq!(err.to_string(), "unknown stage: no-such-stage");
    }

    #[test]
    fn the_invariant_violation_carries_the_reason() {
        let err = ApplyError::InvariantViolation("cursor_in_scope".to_string());
        assert_eq!(err.to_string(), "invariant violation: cursor_in_scope");
    }

    /// 列が拒んだ位置指定は、壊れた歴史として同じ経路へ流れる (無言の no-op にしない)。
    #[test]
    fn a_refused_position_becomes_broken_history() {
        let error = ApplyError::from(StageSlotsError::OutOfRange {
            stage: crate::orchestration::StageIndex::new(3),
        });
        assert_eq!(
            error.to_string(),
            "invariant violation: stage index out of range: 3"
        );
    }

    #[test]
    fn rejections_compare_by_value() {
        assert_eq!(
            ApplyError::InvariantViolation("a".to_string()),
            ApplyError::InvariantViolation("a".to_string())
        );
        assert_ne!(
            ApplyError::InvariantViolation("a".to_string()),
            ApplyError::InvariantViolation("b".to_string())
        );
    }
}
