//! `LearningObservation` — 1 件の学びと、その両側の実測。

use super::learning::Learning;
use super::learning_disposition::LearningDisposition;

/// 人が残すと決めた学びと、書込み直前に実測した両側の在否。
///
/// 在否の正本は**ディスク**である — 監査シャードとメモリ層は人も編集する面なので、集約の
/// 履歴だけからは「片側だけ消えた」を知れない。だから実測を材料として受け取る
/// （`coding-rules/aggregate-references.md`「判断に要るデータは引数で渡す」）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningObservation {
    learning: Learning,
    recorded_in_audit: bool,
    present_in_practice_file: bool,
}

impl LearningObservation {
    /// 学びと両側の実測を束ねる完全コンストラクタ。
    #[must_use]
    pub const fn new(
        learning: Learning,
        recorded_in_audit: bool,
        present_in_practice_file: bool,
    ) -> LearningObservation {
        LearningObservation {
            learning,
            recorded_in_audit,
            present_in_practice_file,
        }
    }

    /// 実測から導いた書込みの内訳。両側在るなら `None`。
    #[must_use]
    pub const fn disposition(&self) -> Option<LearningDisposition> {
        LearningDisposition::of_presence(self.recorded_in_audit, self.present_in_practice_file)
    }

    /// 学び本体。
    #[must_use]
    pub const fn learning(&self) -> &Learning {
        &self.learning
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::LearningObservation;
    use crate::orchestration::{
        Learning, LearningCandidateId, LearningDisposition, LearningScope, LearningSource,
        PracticeHeading,
    };

    fn learning(text: &str) -> Learning {
        Learning::new(
            LearningCandidateId::parse("c1").expect("candidate"),
            LearningScope::Project,
            PracticeHeading::corrections(),
            text,
            LearningSource::Orchestrator,
        )
    }

    #[test]
    fn a_learning_present_on_both_sides_has_nothing_to_write() {
        assert_eq!(
            LearningObservation::new(learning("a"), true, true).disposition(),
            None
        );
    }

    #[test]
    fn a_fresh_learning_writes_both_sides() {
        assert_eq!(
            LearningObservation::new(learning("a"), false, false).disposition(),
            Some(LearningDisposition::Fresh)
        );
    }
}
