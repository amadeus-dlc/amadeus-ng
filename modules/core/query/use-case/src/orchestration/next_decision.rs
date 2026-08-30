//! `NextRequest` / `NextDecision` / `EngineSignal` — `next` の状態依存分岐 (BR3.1)。
//!
//! 状態**非依存**の分岐 (read-only フラグ、名詞トークン、scope 検証、compose、`--single` 等) は
//! 実行状態ビューのクエリではなくユースケース前段の要求分類に属する (BR3.2)。ここに来るのは
//! 「実行状態を見なければ決まらない」観測だけである。

use crate::execution_view::{CheckboxState, StageIndex};

/// [`crate::execution_view::ExecutionStateView::next_decision`] への入力のうち、ワークフロー
/// 状態の判断に要る観測 (entities.md NextRequest)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NextRequest {
    resume: bool,
    reentry: bool,
    free_text: bool,
}

/// 何も観測していない素の要求 (通常のループ 1 周)。
impl Default for NextRequest {
    fn default() -> NextRequest {
        NextRequest::new(false, false, false)
    }
}

impl NextRequest {
    /// 3 観測を束ねる。
    ///
    /// `resume` = `--resume` 指定、`reentry` = `--stage` / `--phase` / `--review` / `--new-intent`
    /// のいずれか (park ガードを外す再入フラグ)、`free_text` = 稼働中に自由記述 prose が来た。
    #[must_use]
    pub const fn new(resume: bool, reentry: bool, free_text: bool) -> NextRequest {
        NextRequest {
            resume,
            reentry,
            free_text,
        }
    }

    /// `--resume` 指定があったか。
    #[must_use]
    pub const fn is_resume(self) -> bool {
        self.resume
    }

    /// 再入フラグがあったか (park ガードを外す)。
    #[must_use]
    pub const fn is_reentry(self) -> bool {
        self.reentry
    }

    /// 稼働中に自由記述が来たか。
    #[must_use]
    pub const fn is_free_text(self) -> bool {
        self.free_text
    }
}

/// 状態依存の分岐だけを表す閉集合 (entities.md NextDecision)。書込なし。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NextDecision {
    /// 名指しのステージを走らせる。
    RunStage {
        /// 走らせるステージ。
        stage: StageIndex,
        /// そのステージがゲート付きか (`phase != initialization` — BR1.3)。
        gate: bool,
    },
    /// ループの停止 (完了・エピローグ・冪等な終端)。
    Done,
    /// park された状態での停止。
    Parked {
        /// park している位置。
        stage: StageIndex,
    },
    /// park 中に `--resume` が来た — unpark してから再開メニューへ。
    UnparkThenResume,
    /// 非 park で `--resume` が来た — 再開メニューへ。
    ResumeMenu,
    /// 稼働中の自由記述 — 新規作業のルーティングへ。
    NewWorkRouting,
    /// カーソルが実効 SKIP かつ着手済み — 復旧可能な plan/cursor 不整合。
    RecoverSkipInconsistency {
        /// 不整合を起こしているステージ。
        stage: StageIndex,
        /// そのステージの観測 checkbox。
        checkbox: CheckboxState,
    },
    /// カーソルが実効 SKIP かつ未着手 — 復旧経路のない plan/cursor 不整合。
    InconsistentSkip {
        /// 不整合を起こしているステージ。
        stage: StageIndex,
        /// そのステージの観測 checkbox。
        checkbox: CheckboxState,
    },
}

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
    use crate::execution_view::{ExecutionStateView, ExecutionStatus, StageProgressView};
    use crate::workflow_view::{PhaseView, PlanActionView, ScopeSlugView, StageSlugView};

    /// [`StageIndex`] を作れるのは実行状態ビューだけなので、テストも同じ経路を通す。
    fn index(value: usize) -> StageIndex {
        let stages = (0..8)
            .map(|i| {
                StageProgressView::new(
                    StageSlugView::parse(&format!("stage-{i}")).unwrap(),
                    PhaseView::Inception,
                    CheckboxState::Pending,
                    PlanActionView::Execute,
                )
            })
            .collect();
        ExecutionStateView::new(
            ScopeSlugView::parse("classic").unwrap(),
            ExecutionStatus::Running,
            "stage-0",
            None,
            "t",
            stages,
        )
        .unwrap()
        .stage_index(value)
        .unwrap()
    }

    #[test]
    fn the_request_carries_the_three_state_relevant_observations() {
        let req = NextRequest::new(true, false, true);
        assert!(req.is_resume());
        assert!(!req.is_reentry());
        assert!(req.is_free_text());
    }

    #[test]
    fn a_plain_request_observes_nothing() {
        let req = NextRequest::default();
        assert!(!req.is_resume());
        assert!(!req.is_reentry());
        assert!(!req.is_free_text());
    }

    #[test]
    fn run_stage_and_the_error_arms_carry_the_stage() {
        let decision = NextDecision::RunStage {
            stage: index(2),
            gate: true,
        };
        assert_eq!(
            decision,
            NextDecision::RunStage {
                stage: index(2),
                gate: true
            }
        );
        let inconsistent = NextDecision::InconsistentSkip {
            stage: index(1),
            checkbox: CheckboxState::Pending,
        };
        assert_ne!(inconsistent, NextDecision::Done);
    }

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

    #[test]
    fn the_eight_decisions_are_matched_exhaustively() {
        fn name(decision: &NextDecision) -> &'static str {
            match decision {
                NextDecision::RunStage { .. } => "RunStage",
                NextDecision::Done => "Done",
                NextDecision::Parked { .. } => "Parked",
                NextDecision::UnparkThenResume => "UnparkThenResume",
                NextDecision::ResumeMenu => "ResumeMenu",
                NextDecision::NewWorkRouting => "NewWorkRouting",
                NextDecision::RecoverSkipInconsistency { .. } => "RecoverSkipInconsistency",
                NextDecision::InconsistentSkip { .. } => "InconsistentSkip",
            }
        }
        assert_eq!(name(&NextDecision::Done), "Done");
        assert_eq!(name(&NextDecision::ResumeMenu), "ResumeMenu");
        assert_eq!(
            name(&NextDecision::RunStage {
                stage: index(0),
                gate: false
            }),
            "RunStage"
        );
        assert_eq!(name(&NextDecision::Parked { stage: index(0) }), "Parked");
        assert_eq!(name(&NextDecision::UnparkThenResume), "UnparkThenResume");
        assert_eq!(name(&NextDecision::NewWorkRouting), "NewWorkRouting");
        assert_eq!(
            name(&NextDecision::RecoverSkipInconsistency {
                stage: index(1),
                checkbox: CheckboxState::InProgress
            }),
            "RecoverSkipInconsistency"
        );
        assert_eq!(
            name(&NextDecision::InconsistentSkip {
                stage: index(1),
                checkbox: CheckboxState::Pending
            }),
            "InconsistentSkip"
        );
    }
}
