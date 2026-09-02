//! 閉集合の**行の綴り**を 1 か所に集める (公開型ゼロの内部モジュール)。
//!
//! 行の値は集約のクエリの答えの写しである。答えがドメインの型 (`Status` / `CheckboxState` /
//! `NextDecision` / `JumpDirection` / `CommandError`) のとき、それを 1 列の文字列にする綴りが
//! 要る。**ドメインが綴りを持っているものは絶対にここへ書かない** — `PhaseId::as_str` /
//! `PlanAction::as_str` / `ExecutionKind::as_str` / `StageMode::as_str` /
//! `ReviewClass::as_str` / `AutonomyMode::as_state_field` はそのまま使う。
//!
//! ここに在るのは「ドメインが読取面の綴りを持っていない値」だけであり、いずれも kebab-case
//! である。upstream の逐語ではない — 逐語文言を組むのは出す側 (プレゼンタ) の仕事であり、
//! 行が運ぶのは**キーになる分類子**である (`coding-rules/cqrs-boundaries.md` 規則 6 の
//! 2026-09-02 追記)。

use core_command_domain::orchestration::{CommandError, JumpDirection, NextDecision, Status};
use core_command_domain::workspace::CheckboxState;

/// `read_next_jump.outcome` の受理されなかった場合の値。
pub(crate) const JUMP_REFUSED: &str = "refused";

/// ワークフロー全体の 2 値。
pub(crate) const fn status(value: Status) -> &'static str {
    match value {
        Status::Running => "running",
        Status::Completed => "completed",
    }
}

/// checkbox の 6 値 (状態ファイル上のマーカー文字とは別の面 — 行のキーは語である)。
pub(crate) const fn checkbox(value: CheckboxState) -> &'static str {
    // amadeus-lint: allow(checkbox-vocabulary) 分類の再実装ではなく**読取面の綴り表**である — 全 6 変種の網羅写像で判断を 1 つも含まず、腕が欠ければビルドが落ちる
    match value {
        CheckboxState::Pending => "pending",
        CheckboxState::InProgress => "in-progress",
        CheckboxState::AwaitingApproval => "awaiting-approval",
        CheckboxState::Revising => "revising",
        CheckboxState::Completed => "completed",
        CheckboxState::Skipped => "skipped",
    }
}

/// `next_decision` の 8 分岐。
pub(crate) const fn decision_kind(decision: &NextDecision) -> &'static str {
    match decision {
        NextDecision::RunStage { .. } => "run-stage",
        NextDecision::Done => "done",
        NextDecision::Parked { .. } => "parked",
        NextDecision::UnparkThenResume => "unpark-then-resume",
        NextDecision::ResumeMenu => "resume-menu",
        NextDecision::NewWorkRouting => "new-work-routing",
        NextDecision::RecoverSkipInconsistency { .. } => "recover-skip-inconsistency",
        NextDecision::InconsistentSkip { .. } => "inconsistent-skip",
    }
}

/// `jump_resolve` が受理したときの方向。
pub(crate) const fn jump_direction(direction: JumpDirection) -> &'static str {
    match direction {
        JumpDirection::Forward => "forward",
        JumpDirection::Backward => "backward",
        JumpDirection::Redo => "redo",
    }
}

/// `jump_resolve` が拒否したときの理由。
///
/// `jump_resolve` が実際に返すのは `NotRunning` / `InvalidTarget` / `IntentMismatch` の 3 つ
/// だけだが、拒否理由の閉集合を**丸めずに**全部綴る — 「起きないはずの値をどれかに寄せる」
/// のは行に嘘を書くことであり、変種が増えたときはこの `match` がビルドで教える。
pub(crate) const fn jump_refusal(error: &CommandError) -> &'static str {
    match error {
        CommandError::IntentMismatch => "intent-mismatch",
        CommandError::NotRunning => "not-running",
        CommandError::CheckboxPrecondition { .. } => "checkbox-precondition",
        CommandError::NotSkippable(_) => "not-skippable",
        CommandError::NotStale(_) => "not-stale",
        CommandError::InvalidTarget(_) => "invalid-target",
        CommandError::RefusedUnderAutonomy => "refused-under-autonomy",
        CommandError::SequenceExhausted => "sequence-exhausted",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_statuses_and_the_six_checkboxes_are_spelled_in_kebab_case() {
        assert_eq!(status(Status::Running), "running");
        assert_eq!(status(Status::Completed), "completed");

        let markers = [
            checkbox(CheckboxState::Pending),
            checkbox(CheckboxState::InProgress),
            checkbox(CheckboxState::AwaitingApproval),
            checkbox(CheckboxState::Revising),
            checkbox(CheckboxState::Completed),
            checkbox(CheckboxState::Skipped),
        ];
        assert_eq!(
            markers,
            [
                "pending",
                "in-progress",
                "awaiting-approval",
                "revising",
                "completed",
                "skipped"
            ]
        );
        assert!(markers.iter().all(|word| !word.contains('_')));
    }

    /// 材料を持たない分岐だけをここで固定する。ステージ位置を運ぶ 4 分岐は
    /// [`StageIndex`] を外から作れない (構築子は集約が持つ) ので、実物の集約を再生する
    /// 契約テスト (`tests/read_tables_test.rs`) が固定する。
    ///
    /// [`StageIndex`]: core_command_domain::orchestration::StageIndex
    #[test]
    fn the_payload_free_decisions_spell_themselves_distinctly() {
        let decisions = [
            NextDecision::Done,
            NextDecision::UnparkThenResume,
            NextDecision::ResumeMenu,
            NextDecision::NewWorkRouting,
        ];
        let spelled: Vec<&str> = decisions.iter().map(decision_kind).collect();
        assert_eq!(
            spelled,
            [
                "done",
                "unpark-then-resume",
                "resume-menu",
                "new-work-routing"
            ]
        );
    }

    #[test]
    fn the_three_directions_are_spelled_and_the_refusal_marker_is_its_own_word() {
        assert_eq!(jump_direction(JumpDirection::Forward), "forward");
        assert_eq!(jump_direction(JumpDirection::Backward), "backward");
        assert_eq!(jump_direction(JumpDirection::Redo), "redo");
        assert_eq!(JUMP_REFUSED, "refused");
    }

    #[test]
    fn the_payload_free_refusals_are_spelled_without_rounding() {
        let refusals = [
            jump_refusal(&CommandError::IntentMismatch),
            jump_refusal(&CommandError::NotRunning),
            jump_refusal(&CommandError::RefusedUnderAutonomy),
            jump_refusal(&CommandError::SequenceExhausted),
        ];
        assert_eq!(
            refusals,
            [
                "intent-mismatch",
                "not-running",
                "refused-under-autonomy",
                "sequence-exhausted"
            ]
        );
    }
}
