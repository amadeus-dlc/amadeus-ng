//! `EngineSignal` — エンジンが放出する信号の観測射影 (Quint `DirectiveKind` の 4 値)。
//!
//! [`NextDecision`] の 8 分岐をモデル語彙の 4 値へ畳む導出だけを持つ。モデル語彙の外側にある
//! 3 分岐は「ステージを走らせない・park でもエラーでもない停止」なので `Done` へ落ちる。

use super::next_decision::NextDecision;
use crate::orchestration::StageIndex;

/// エンジンが放出する信号の観測射影 (Quint `DirectiveKind` の 4 値)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EngineSignal {
    /// 名指しのステージを走らせる。実効プランが EXECUTE のステージにしか出ない。
    RunStage(StageIndex),
    /// ループの停止。
    Done,
    /// 意図的に park された状態。`Done` とは別で、スコープ内にはまだ未実施ステージが残る。
    Parked,
    /// plan/cursor 不整合 (実効 SKIP のステージに run-stage を出さない)。
    EngineError,
}

impl From<&NextDecision> for EngineSignal {
    /// BR3.1 の導出規則。
    ///
    /// `RunStage` / `Done` / `Parked` / 2 つの不整合はモデルの 4 値に 1:1 で写る。モデル語彙の
    /// 外側にある 3 分岐 (`UnparkThenResume` / `ResumeMenu` / `NewWorkRouting`) は「ステージを
    /// 走らせない・park でもエラーでもない停止」なので `Done` へ畳む。
    fn from(decision: &NextDecision) -> EngineSignal {
        match decision {
            NextDecision::RunStage { stage, .. } => EngineSignal::RunStage(*stage),
            NextDecision::Done
            | NextDecision::UnparkThenResume
            | NextDecision::ResumeMenu
            | NextDecision::NewWorkRouting => EngineSignal::Done,
            NextDecision::Parked { .. } => EngineSignal::Parked,
            NextDecision::RecoverSkipInconsistency { .. }
            | NextDecision::InconsistentSkip { .. } => EngineSignal::EngineError,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // 索引フィクスチャは主たる従属先 (決定の変種が運ぶ型) と同居する — 兄弟からはこの経路で参照する。
    use super::super::next_decision::index;
    use crate::orchestration::CheckboxState;

    #[test]
    fn the_signal_derives_run_stage_done_parked_and_error() {
        assert_eq!(
            EngineSignal::from(&NextDecision::RunStage {
                stage: index(4),
                gate: false
            }),
            EngineSignal::RunStage(index(4))
        );
        assert_eq!(EngineSignal::from(&NextDecision::Done), EngineSignal::Done);
        assert_eq!(
            EngineSignal::from(&NextDecision::Parked { stage: index(0) }),
            EngineSignal::Parked
        );
        assert_eq!(
            EngineSignal::from(&NextDecision::RecoverSkipInconsistency {
                stage: index(1),
                checkbox: CheckboxState::InProgress
            }),
            EngineSignal::EngineError
        );
        assert_eq!(
            EngineSignal::from(&NextDecision::InconsistentSkip {
                stage: index(1),
                checkbox: CheckboxState::Pending
            }),
            EngineSignal::EngineError
        );
    }

    #[test]
    fn the_decisions_outside_the_model_vocabulary_stop_the_loop() {
        // Quint の DirectiveKind は 4 値しかない。resume / 自由記述の 3 分岐は「ステージを
        // 走らせない・park でもエラーでもない停止」なので Done へ畳む (BR3.1 の導出規則の外側)。
        for decision in [
            NextDecision::UnparkThenResume,
            NextDecision::ResumeMenu,
            NextDecision::NewWorkRouting,
        ] {
            assert_eq!(EngineSignal::from(&decision), EngineSignal::Done);
        }
    }
}
