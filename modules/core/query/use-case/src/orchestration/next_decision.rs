//! `NextDecision` — `next` の状態依存分岐 (BR3.1) の閉集合。
//!
//! 状態**非依存**の分岐 (read-only フラグ、名詞トークン、scope 検証、compose、`--single` 等) は
//! 実行状態ビューのクエリではなくユースケース前段の要求分類に属する (BR3.2)。ここに来るのは
//! 「実行状態を見なければ決まらない」観測だけである。

use crate::orchestration::{CheckboxState, StageIndex};

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

/// [`StageIndex`] を作れるのは実行状態ビューだけなので、テストも同じ経路を通す。
///
/// 兄弟の `engine_signal` も同じ索引を要る — 主たる従属先 (この決定の変種が運ぶ型) の側に
/// 置き、兄弟からは `super::next_decision::index` で参照する。
#[cfg(test)]
pub(super) fn index(value: usize) -> StageIndex {
    use crate::orchestration::{
        ExecutionStateView, ExecutionStatus, PhaseView, PlanActionView, ScopeSlugView,
        StageProgressView, StageSlugView,
    };

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

#[cfg(test)]
mod tests {
    use super::*;

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
