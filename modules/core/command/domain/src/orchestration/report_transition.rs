//! 報告により起きた遷移の値。イベント自身の識別子を重ねない。
use super::ArtifactPaths;

/// 報告イベントに属する遷移事実。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportTransition {
    /// ゲートを開いた。
    GateOpened {
        /// 開放時の成果物。
        artifacts: ArtifactPaths,
    },
    /// ゲートを承認した。
    GateApproved {
        /// 人間の入力。
        user_input: Option<String>,
    },
    /// ゲートを差し戻した。
    GateRejected {
        /// 差し戻し理由。
        feedback: Option<String>,
    },
    /// 改訂してゲートへ戻した。
    StageRevised,
    /// ステージを読み飛ばした。
    StageSkipped {
        /// 読み飛ばした理由。
        reason: String,
    },
}

impl ReportTransition {
    /// この遷移が報告する操作列として成立するか。
    #[must_use]
    pub fn accepts_steps(&self, steps: &super::TransitionSteps) -> bool {
        use super::TransitionStep;
        match self {
            Self::GateOpened { .. } => steps.is_single(TransitionStep::GateStart),
            Self::GateApproved { .. } => {
                steps.is_single(TransitionStep::Approve)
                    || steps.is_pair(TransitionStep::GateStartRecovered, TransitionStep::Approve)
            }
            Self::GateRejected { .. } => steps.is_single(TransitionStep::Reject),
            Self::StageRevised => steps.is_single(TransitionStep::Revise),
            Self::StageSkipped { .. } => steps.is_single(TransitionStep::Skip),
        }
    }
}
