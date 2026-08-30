//! ジャーナル・スナップショット行における閉集合の**綴りの正本**。
//!
//! ドメイン側の `as_str` / `parse` は流用しない (`mod.rs` の「綴りの正本はここにある」)。
//! ここの綴りを変えると既に書かれた行が読めなくなるので、対応表は逐語テストで固定する。

use core_command_domain::orchestration::{AutonomyMode, JumpDirection, Status};
use core_command_domain::workflow_definition::{BrownfieldGreenfield, PhaseId, PlanAction};
use core_command_domain::workspace::CheckboxState;

use super::wire_error::WireDecodeError;

/// 実効プラン 1 要素の綴り。
pub(super) const fn plan_action_spelling(value: PlanAction) -> &'static str {
    match value {
        PlanAction::Execute => "Execute",
        PlanAction::Skip => "Skip",
    }
}

/// 実効プラン 1 要素の復号。
pub(super) fn plan_action_of(
    raw: &str,
    field: &'static str,
) -> Result<PlanAction, WireDecodeError> {
    match raw {
        "Execute" => Ok(PlanAction::Execute),
        "Skip" => Ok(PlanAction::Skip),
        other => Err(WireDecodeError::malformed(field, other)),
    }
}

/// checkbox 1 要素の綴り。
///
/// 分類の再実装ではなく**綴りの全単射**である。ワイヤ形式は閉集合の全変種に 1 対 1 で
/// 対応させる必要があり、述語 (`is_in_flight` 等) では表現できない。変種が増えれば
/// 網羅 match がコンパイルエラーで教える。
pub(super) const fn checkbox_spelling(value: CheckboxState) -> &'static str {
    // amadeus-lint: allow(checkbox-vocabulary) — 綴りの全単射 (上の doc を参照)
    match value {
        CheckboxState::Pending => "Pending",
        CheckboxState::InProgress => "InProgress",
        CheckboxState::AwaitingApproval => "AwaitingApproval",
        CheckboxState::Revising => "Revising",
        CheckboxState::Completed => "Completed",
        CheckboxState::Skipped => "Skipped",
    }
}

/// checkbox 1 要素の復号 (スナップショット行の読取 — 本家同型の差分再生の基底)。
pub(super) fn checkbox_of(raw: &str) -> Result<CheckboxState, WireDecodeError> {
    match raw {
        "Pending" => Ok(CheckboxState::Pending),
        "InProgress" => Ok(CheckboxState::InProgress),
        "AwaitingApproval" => Ok(CheckboxState::AwaitingApproval),
        "Revising" => Ok(CheckboxState::Revising),
        "Completed" => Ok(CheckboxState::Completed),
        "Skipped" => Ok(CheckboxState::Skipped),
        other => Err(WireDecodeError::malformed("checkbox", other)),
    }
}

/// ワークフロー全体の状態の綴り。
pub(super) const fn status_spelling(value: Status) -> &'static str {
    match value {
        Status::Running => "Running",
        Status::Completed => "Completed",
    }
}

/// ワークフロー全体の状態の復号。
pub(super) fn status_of(raw: &str) -> Result<Status, WireDecodeError> {
    match raw {
        "Running" => Ok(Status::Running),
        "Completed" => Ok(Status::Completed),
        other => Err(WireDecodeError::malformed("status", other)),
    }
}

/// 自律モードの綴り。
pub(super) const fn autonomy_spelling(value: AutonomyMode) -> &'static str {
    match value {
        AutonomyMode::Autonomous => "Autonomous",
        AutonomyMode::Gated => "Gated",
    }
}

/// 自律モードの復号。
pub(super) fn autonomy_of(raw: &str) -> Result<AutonomyMode, WireDecodeError> {
    match raw {
        "Autonomous" => Ok(AutonomyMode::Autonomous),
        "Gated" => Ok(AutonomyMode::Gated),
        other => Err(WireDecodeError::malformed("autonomy", other)),
    }
}

/// jump の方向の綴り。
pub(super) const fn direction_spelling(value: JumpDirection) -> &'static str {
    match value {
        JumpDirection::Forward => "Forward",
        JumpDirection::Backward => "Backward",
        JumpDirection::Redo => "Redo",
    }
}

/// jump の方向の復号。
pub(super) fn direction_of(raw: &str) -> Result<JumpDirection, WireDecodeError> {
    match raw {
        "Forward" => Ok(JumpDirection::Forward),
        "Backward" => Ok(JumpDirection::Backward),
        "Redo" => Ok(JumpDirection::Redo),
        other => Err(WireDecodeError::malformed("direction", other)),
    }
}

/// フェーズの綴り (**ジャーナル面**。`stage-graph.json` 面の小文字とは別物)。
pub(super) const fn phase_spelling(value: PhaseId) -> &'static str {
    match value {
        PhaseId::Initialization => "Initialization",
        PhaseId::Ideation => "Ideation",
        PhaseId::Inception => "Inception",
        PhaseId::Construction => "Construction",
        PhaseId::Operation => "Operation",
    }
}

/// フェーズの復号 (ジャーナル面)。
pub(super) fn phase_of(raw: &str, field: &'static str) -> Result<PhaseId, WireDecodeError> {
    match raw {
        "Initialization" => Ok(PhaseId::Initialization),
        "Ideation" => Ok(PhaseId::Ideation),
        "Inception" => Ok(PhaseId::Inception),
        "Construction" => Ok(PhaseId::Construction),
        "Operation" => Ok(PhaseId::Operation),
        other => Err(WireDecodeError::malformed(field, other)),
    }
}

/// プロジェクト種別の綴り (**小文字** — `stage-graph.json` 面と同じ綴りだが由来は別)。
pub(super) const fn project_type_spelling(value: BrownfieldGreenfield) -> &'static str {
    match value {
        BrownfieldGreenfield::Brownfield => "brownfield",
        BrownfieldGreenfield::Greenfield => "greenfield",
    }
}

/// プロジェクト種別の復号。
pub(super) fn project_type_of(raw: &str) -> Result<BrownfieldGreenfield, WireDecodeError> {
    match raw {
        "brownfield" => Ok(BrownfieldGreenfield::Brownfield),
        "greenfield" => Ok(BrownfieldGreenfield::Greenfield),
        other => Err(WireDecodeError::malformed("project_type", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_spelling_round_trips() {
        for value in [PlanAction::Execute, PlanAction::Skip] {
            assert_eq!(
                plan_action_of(plan_action_spelling(value), "overlay").unwrap(),
                value
            );
        }
        for value in [
            CheckboxState::Pending,
            CheckboxState::InProgress,
            CheckboxState::AwaitingApproval,
            CheckboxState::Revising,
            CheckboxState::Completed,
            CheckboxState::Skipped,
        ] {
            assert_eq!(checkbox_of(checkbox_spelling(value)).unwrap(), value);
        }
        for value in [Status::Running, Status::Completed] {
            assert_eq!(status_of(status_spelling(value)).unwrap(), value);
        }
        for value in [AutonomyMode::Autonomous, AutonomyMode::Gated] {
            assert_eq!(autonomy_of(autonomy_spelling(value)).unwrap(), value);
        }
        for value in [
            JumpDirection::Forward,
            JumpDirection::Backward,
            JumpDirection::Redo,
        ] {
            assert_eq!(direction_of(direction_spelling(value)).unwrap(), value);
        }
        for value in [
            PhaseId::Initialization,
            PhaseId::Ideation,
            PhaseId::Inception,
            PhaseId::Construction,
            PhaseId::Operation,
        ] {
            assert_eq!(phase_of(phase_spelling(value), "phase").unwrap(), value);
        }
        for value in BrownfieldGreenfield::ALL {
            assert_eq!(
                project_type_of(project_type_spelling(value)).unwrap(),
                value
            );
        }
    }

    #[test]
    fn the_journal_spellings_are_fixed_verbatim() {
        // 行に書かれて残る綴りである。ドメイン側の `as_str` とは**別物**で、
        // 例えば PhaseId はここでは大文字始まり、`stage-graph.json` 面では小文字である。
        assert_eq!(plan_action_spelling(PlanAction::Execute), "Execute");
        assert_eq!(checkbox_spelling(CheckboxState::InProgress), "InProgress");
        assert_eq!(status_spelling(Status::Running), "Running");
        assert_eq!(autonomy_spelling(AutonomyMode::Gated), "Gated");
        assert_eq!(direction_spelling(JumpDirection::Forward), "Forward");
        assert_eq!(phase_spelling(PhaseId::Ideation), "Ideation");
        assert_eq!(
            PhaseId::Ideation.as_str(),
            "ideation",
            "面が違えば綴りも違う"
        );
        assert_eq!(
            project_type_spelling(BrownfieldGreenfield::Greenfield),
            "greenfield"
        );
    }

    #[test]
    fn an_unknown_spelling_is_refused_with_its_raw_value() {
        assert_eq!(
            plan_action_of("EXECUTE", "overlay").unwrap_err(),
            WireDecodeError::malformed("overlay", "EXECUTE")
        );
        assert!(checkbox_of("done").is_err());
        assert!(status_of("running").is_err());
        assert!(autonomy_of("gated").is_err());
        assert!(direction_of("forward").is_err());
        assert!(phase_of("ideation", "phase").is_err());
        assert!(project_type_of("Greenfield").is_err());
    }
}
