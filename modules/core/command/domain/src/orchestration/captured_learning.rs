//! `CapturedLearning` — 記録すると決まった学びと、その書込みの内訳。

use super::learning::Learning;
use super::learning_disposition::LearningDisposition;

/// 「この学びを、この内訳で書く」と決まった 1 件。
///
/// 投影はこの対を読んで実践行と監査行を描くだけであり、現在のディスクから判定をやり直さない
/// （`coding-rules/cqrs-boundaries.md`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedLearning {
    learning: Learning,
    disposition: LearningDisposition,
}

impl CapturedLearning {
    /// 学びと内訳を束ねる完全コンストラクタ。
    #[must_use]
    pub const fn new(learning: Learning, disposition: LearningDisposition) -> CapturedLearning {
        CapturedLearning {
            learning,
            disposition,
        }
    }

    /// 学び本体。
    #[must_use]
    pub const fn learning(&self) -> &Learning {
        &self.learning
    }

    /// 書込みの内訳。
    #[must_use]
    pub const fn disposition(&self) -> LearningDisposition {
        self.disposition
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::CapturedLearning;
    use crate::orchestration::{
        Learning, LearningCandidateId, LearningDisposition, LearningScope, LearningSource,
        PracticeHeading,
    };

    #[test]
    fn the_pair_keeps_the_learning_and_its_disposition() {
        let learning = Learning::new(
            LearningCandidateId::parse("c1").expect("candidate"),
            LearningScope::Team,
            PracticeHeading::from_routed("Testing Posture"),
            "ALWAYS run the suite",
            LearningSource::Orchestrator,
        );
        let captured = CapturedLearning::new(learning.clone(), LearningDisposition::Fresh);
        assert_eq!(captured.learning(), &learning);
        assert_eq!(captured.disposition(), LearningDisposition::Fresh);
    }
}
