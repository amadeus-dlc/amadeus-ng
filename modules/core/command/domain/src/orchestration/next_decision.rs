//! `NextDecision` — `next` の状態依存分岐の閉集合 (BR3.1)。集約 `IntentExecution` のクエリ
//! [`IntentExecution::next_decision`](super::IntentExecution::next_decision) の結果 (書込なし)。
//!
//! 判断の所在は集約である (仕様 10 §2.3 / ADR-002 ④ — 状態の所有者の外で判断する Ask 型を
//! 避ける)。RMU はこの答えをリードモデル (`read_next_answer`) へ投影し、クエリ側はそれを
//! 読んで返すだけになる (`coding-rules/cqrs-boundaries.md` 規則 3 / 6 の 2026-09-02 追記)。

use super::stage_index::StageIndex;
use crate::workspace::CheckboxState;

/// `next_decision` の結果。状態依存の分岐だけを表す閉集合 (entities.md NextDecision)。
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_stage_and_the_error_arms_carry_the_stage() {
        let decision = NextDecision::RunStage {
            stage: StageIndex::new(2),
            gate: true,
        };
        assert_eq!(
            decision,
            NextDecision::RunStage {
                stage: StageIndex::new(2),
                gate: true
            }
        );
        let inconsistent = NextDecision::InconsistentSkip {
            stage: StageIndex::new(1),
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
        assert_eq!(name(&NextDecision::UnparkThenResume), "UnparkThenResume");
        assert_eq!(name(&NextDecision::NewWorkRouting), "NewWorkRouting");
        assert_eq!(
            name(&NextDecision::Parked {
                stage: StageIndex::new(0)
            }),
            "Parked"
        );
        assert_eq!(
            name(&NextDecision::RecoverSkipInconsistency {
                stage: StageIndex::new(1),
                checkbox: CheckboxState::InProgress
            }),
            "RecoverSkipInconsistency"
        );
        assert_eq!(
            name(&NextDecision::InconsistentSkip {
                stage: StageIndex::new(1),
                checkbox: CheckboxState::Pending
            }),
            "InconsistentSkip"
        );
    }
}
