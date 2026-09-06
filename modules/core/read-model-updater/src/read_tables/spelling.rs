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

/// `read_scope_change.kind` の 2 値 — 要求された scope が state の scope と同じか。
pub(crate) const fn scope_change(same_as_state: bool) -> &'static str {
    if same_as_state {
        "same-as-state"
    } else {
        "scope-change"
    }
}

/// ワークフロー全体の 2 値。
pub(crate) const fn status(value: Status) -> &'static str {
    match value {
        Status::Running => "running",
        Status::Completed => "completed",
    }
}

/// checkbox の 6 値 (状態ファイル上のマーカー文字とは別の面 — 行のキーは語である)。
///
/// 綴りの正本は `CheckboxState` 自身である (b46 — 逐語文言と読取面が同じ語を要るので
/// ドメインの 1 か所に寄せた)。ここは所有者へ委譲するだけで、写像は持たない。
pub(crate) const fn checkbox(value: CheckboxState) -> &'static str {
    value.spelling()
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
        CommandError::UnknownStage(_) => "unknown-stage",
        CommandError::NoDeclaredReviewer(_) => "no-declared-reviewer",
        CommandError::ReviewerMismatch { .. } => "reviewer-mismatch",
        CommandError::ReviewBudgetExceeded { .. } => "review-budget-exceeded",
        CommandError::ReviewOutOfSequence { .. } => "review-out-of-sequence",
        CommandError::NoPendingReview { .. } => "no-pending-review",
        CommandError::ReviewReceiptMissing { .. } => "review-receipt-missing",
        CommandError::PracticesReceiptMissing(_) => "practices-receipt-missing",
        CommandError::HumanPresenceRequired => "human-presence-required",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use core_command_domain::orchestration::{
        IntentExecution, IntentExecutionEventId, IntentExecutionId, IntentId, StageDisplay,
        StageEntries, StageEntry, StageIndex, Started,
    };
    use core_command_domain::workflow_definition::{PhaseId, PlanAction, StageNumber, StageSlug};

    /// b40 のテスト用固定イベント識別子 (同じ材料から組んだイベントを同値に保つため)。
    fn event_id() -> IntentExecutionEventId {
        IntentExecutionEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002").expect("UUIDv7")
    }

    #[test]
    fn the_scope_comparison_spells_both_answers_in_kebab_case() {
        assert_eq!(scope_change(true), "same-as-state");
        assert_eq!(scope_change(false), "scope-change");
    }

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

    /// ステージ位置を 1 つ借りるためだけの実行。
    ///
    /// `StageIndex` の構築子は集約が持つので、位置を運ぶ拒否理由を組むには実物の集約が要る。
    /// 誕生記録から起こすのが唯一の経路である。
    fn a_stage_index() -> StageIndex {
        let display = StageDisplay::new(
            StageNumber::parse("0.1").expect("番号は文法内"),
            "State Init",
            "orchestrator",
        )
        .expect("単一行");
        let stages = StageEntries::new(vec![StageEntry::new(
            StageSlug::parse("state-init").expect("slug は文法内"),
            PhaseId::Initialization,
            PlanAction::Execute,
            false,
            display,
        )])
        .expect("フィクスチャの計画は不変条件を満たす");
        let started = Started::new(
            event_id(),
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("実行 id"),
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("intent id"),
            stages,
        );
        let occurred_at = DateTime::parse_from_rfc3339("2026-09-02T00:00:00Z")
            .expect("固定の ISO 8601 UTC")
            .with_timezone(&Utc);
        IntentExecution::from((started, occurred_at))
            .stage_index(0)
            .expect("1 本目の位置は在る")
    }

    #[test]
    fn the_refusals_that_name_a_stage_are_spelled_without_rounding_either() {
        // `jump_resolve` が返すのはこのうち `invalid-target` だけだが、拒否理由の閉集合は
        // 丸めずに全部綴る (どれかへ寄せると行に嘘を書く)。残りの 3 つもここで綴りを固定して
        // おかないと、変種が増えたときに `match` の腕だけが増えて綴りが検収されない。
        let stage = a_stage_index();
        let refusals = [
            jump_refusal(&CommandError::CheckboxPrecondition {
                stage,
                actual: CheckboxState::Pending,
            }),
            jump_refusal(&CommandError::NotSkippable(stage)),
            jump_refusal(&CommandError::NotStale(stage)),
            jump_refusal(&CommandError::InvalidTarget(stage)),
        ];
        assert_eq!(
            refusals,
            [
                "checkbox-precondition",
                "not-skippable",
                "not-stale",
                "invalid-target"
            ]
        );

        // b48 で加わった 7 変種も同じ扱いである — レビュー会計の拒否は `jump_resolve` からは
        // 返らないが、閉集合の綴りは丸めずに全部固定する。
        let review = [
            jump_refusal(&CommandError::UnknownStage("nowhere".to_string())),
            jump_refusal(&CommandError::NoDeclaredReviewer(stage)),
            jump_refusal(&CommandError::ReviewerMismatch {
                stage,
                declared: "aidlc-quality-agent".to_string(),
            }),
            jump_refusal(&CommandError::ReviewBudgetExceeded {
                stage,
                ordinal: 3,
                budget: 2,
            }),
            jump_refusal(&CommandError::ReviewOutOfSequence {
                stage,
                iteration: 3,
                expected: 1,
            }),
            jump_refusal(&CommandError::NoPendingReview {
                stage,
                iteration: 1,
            }),
            jump_refusal(&CommandError::ReviewReceiptMissing {
                stage,
                reviewer: "aidlc-quality-agent".to_string(),
            }),
            // b49 で加わった 1 変種（段 12 の昇格受領証）も同じ扱いである。
            jump_refusal(&CommandError::PracticesReceiptMissing(stage)),
        ];
        assert_eq!(
            review,
            [
                "unknown-stage",
                "no-declared-reviewer",
                "reviewer-mismatch",
                "review-budget-exceeded",
                "review-out-of-sequence",
                "no-pending-review",
                "review-receipt-missing",
                "practices-receipt-missing",
            ]
        );

        // 17 変種の綴りは互いに重ならない — 重なれば行から理由を読み分けられない。
        let all = [
            jump_refusal(&CommandError::IntentMismatch),
            jump_refusal(&CommandError::NotRunning),
            jump_refusal(&CommandError::RefusedUnderAutonomy),
            jump_refusal(&CommandError::SequenceExhausted),
            // b50 で加わった 1 変種（昇格の human presence ガード、I11）。
            jump_refusal(&CommandError::HumanPresenceRequired),
            refusals[0],
            refusals[1],
            refusals[2],
            refusals[3],
            review[0],
            review[1],
            review[2],
            review[3],
            review[4],
            review[5],
            review[6],
            review[7],
        ];
        let distinct: std::collections::BTreeSet<&str> = all.iter().copied().collect();
        assert_eq!(distinct.len(), all.len(), "拒否理由の綴りは 1 対 1");
    }
}
