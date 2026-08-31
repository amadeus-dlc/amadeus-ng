//! ジャーナル・スナップショット行における閉集合の**綴りの正本**。
//!
//! ドメイン側の `as_str` / `parse` は流用しない (`mod.rs` の「綴りの正本はここにある」)。
//! ここの綴りを変えると既に書かれた行が読めなくなるので、対応表は逐語テストで固定する。

use core_command_domain::orchestration::{AutonomyMode, Status};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, ExecutionKind, PhaseId, PlanAction, ReviewCapValue, ReviewClass,
    RuleScope, SkeletonDefault, StageMode,
};
use core_command_domain::workspace::CheckboxState;

use super::dto_decode_error::DtoDecodeError;

/// 実効プラン 1 要素の綴り。
pub(super) const fn plan_action_spelling(value: PlanAction) -> &'static str {
    match value {
        PlanAction::Execute => "Execute",
        PlanAction::Skip => "Skip",
    }
}

/// 実効プラン 1 要素の復号。
pub(super) fn plan_action_of(raw: &str, field: &'static str) -> Result<PlanAction, DtoDecodeError> {
    match raw {
        "Execute" => Ok(PlanAction::Execute),
        "Skip" => Ok(PlanAction::Skip),
        other => Err(DtoDecodeError::malformed(field, other)),
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
pub(super) fn checkbox_of(raw: &str) -> Result<CheckboxState, DtoDecodeError> {
    match raw {
        "Pending" => Ok(CheckboxState::Pending),
        "InProgress" => Ok(CheckboxState::InProgress),
        "AwaitingApproval" => Ok(CheckboxState::AwaitingApproval),
        "Revising" => Ok(CheckboxState::Revising),
        "Completed" => Ok(CheckboxState::Completed),
        "Skipped" => Ok(CheckboxState::Skipped),
        other => Err(DtoDecodeError::malformed("checkbox", other)),
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
pub(super) fn status_of(raw: &str) -> Result<Status, DtoDecodeError> {
    match raw {
        "Running" => Ok(Status::Running),
        "Completed" => Ok(Status::Completed),
        other => Err(DtoDecodeError::malformed("status", other)),
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
pub(super) fn autonomy_of(raw: &str) -> Result<AutonomyMode, DtoDecodeError> {
    match raw {
        "Autonomous" => Ok(AutonomyMode::Autonomous),
        "Gated" => Ok(AutonomyMode::Gated),
        other => Err(DtoDecodeError::malformed("autonomy", other)),
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
pub(super) fn phase_of(raw: &str, field: &'static str) -> Result<PhaseId, DtoDecodeError> {
    match raw {
        "Initialization" => Ok(PhaseId::Initialization),
        "Ideation" => Ok(PhaseId::Ideation),
        "Inception" => Ok(PhaseId::Inception),
        "Construction" => Ok(PhaseId::Construction),
        "Operation" => Ok(PhaseId::Operation),
        other => Err(DtoDecodeError::malformed(field, other)),
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
pub(super) fn project_type_of(raw: &str) -> Result<BrownfieldGreenfield, DtoDecodeError> {
    match raw {
        "brownfield" => Ok(BrownfieldGreenfield::Brownfield),
        "greenfield" => Ok(BrownfieldGreenfield::Greenfield),
        other => Err(DtoDecodeError::malformed("project_type", other)),
    }
}

/// ステージの実行区分の綴り (**ジャーナル面**。`stage-graph.json` 面の `ALWAYS` とは別物)。
pub(super) const fn execution_kind_spelling(value: ExecutionKind) -> &'static str {
    match value {
        ExecutionKind::Always => "Always",
        ExecutionKind::Conditional => "Conditional",
    }
}

/// ステージの実行区分の復号 (ジャーナル面)。
pub(super) fn execution_kind_of(raw: &str) -> Result<ExecutionKind, DtoDecodeError> {
    match raw {
        "Always" => Ok(ExecutionKind::Always),
        "Conditional" => Ok(ExecutionKind::Conditional),
        other => Err(DtoDecodeError::malformed("execution", other)),
    }
}

/// ステージの実行様式の綴り (ジャーナル面)。
pub(super) const fn stage_mode_spelling(value: StageMode) -> &'static str {
    match value {
        StageMode::Inline => "Inline",
        StageMode::Subagent => "Subagent",
        StageMode::Pipeline => "Pipeline",
        StageMode::Mob => "Mob",
        StageMode::AgentTeam => "AgentTeam",
    }
}

/// ステージの実行様式の復号 (ジャーナル面)。
pub(super) fn stage_mode_of(raw: &str) -> Result<StageMode, DtoDecodeError> {
    match raw {
        "Inline" => Ok(StageMode::Inline),
        "Subagent" => Ok(StageMode::Subagent),
        "Pipeline" => Ok(StageMode::Pipeline),
        "Mob" => Ok(StageMode::Mob),
        "AgentTeam" => Ok(StageMode::AgentTeam),
        other => Err(DtoDecodeError::malformed("mode", other)),
    }
}

/// レビュー重量の綴り (ジャーナル面)。
pub(super) const fn review_class_spelling(value: ReviewClass) -> &'static str {
    match value {
        ReviewClass::Advisory => "Advisory",
        ReviewClass::Adversarial => "Adversarial",
    }
}

/// レビュー重量の復号 (ジャーナル面)。
pub(super) fn review_class_of(raw: &str) -> Result<ReviewClass, DtoDecodeError> {
    match raw {
        "Advisory" => Ok(ReviewClass::Advisory),
        "Adversarial" => Ok(ReviewClass::Adversarial),
        other => Err(DtoDecodeError::malformed("review_class", other)),
    }
}

/// 規則の出自の綴り (ジャーナル面)。
pub(super) const fn rule_scope_spelling(value: RuleScope) -> &'static str {
    match value {
        RuleScope::Org => "Org",
        RuleScope::Team => "Team",
        RuleScope::Project => "Project",
        RuleScope::Phase => "Phase",
    }
}

/// 規則の出自の復号 (ジャーナル面)。
pub(super) fn rule_scope_of(raw: &str) -> Result<RuleScope, DtoDecodeError> {
    match raw {
        "Org" => Ok(RuleScope::Org),
        "Team" => Ok(RuleScope::Team),
        "Project" => Ok(RuleScope::Project),
        "Phase" => Ok(RuleScope::Phase),
        other => Err(DtoDecodeError::malformed("rule_scope", other)),
    }
}

/// スコープ既定の walking skeleton の綴り (ジャーナル面)。
pub(super) const fn skeleton_default_spelling(value: SkeletonDefault) -> &'static str {
    match value {
        SkeletonDefault::On => "On",
        SkeletonDefault::Off => "Off",
    }
}

/// スコープ既定の walking skeleton の復号 (ジャーナル面)。
pub(super) fn skeleton_default_of(raw: &str) -> Result<SkeletonDefault, DtoDecodeError> {
    match raw {
        "On" => Ok(SkeletonDefault::On),
        "Off" => Ok(SkeletonDefault::Off),
        other => Err(DtoDecodeError::malformed("skeleton", other)),
    }
}

/// レビュー上限の綴り (ジャーナル面)。
///
/// `None` は「`none` と**宣言された**」ことを表す値である — 「宣言が無い」は
/// `Option<ReviewCapValue>` の `None` 側 (ワイヤ上は欄そのものの不在) が表す。
pub(super) const fn review_cap_spelling(value: ReviewCapValue) -> &'static str {
    match value {
        ReviewCapValue::Adversarial => "Adversarial",
        ReviewCapValue::Advisory => "Advisory",
        ReviewCapValue::None => "None",
    }
}

/// レビュー上限の復号 (ジャーナル面)。
pub(super) fn review_cap_of(raw: &str) -> Result<ReviewCapValue, DtoDecodeError> {
    match raw {
        "Adversarial" => Ok(ReviewCapValue::Adversarial),
        "Advisory" => Ok(ReviewCapValue::Advisory),
        "None" => Ok(ReviewCapValue::None),
        other => Err(DtoDecodeError::malformed("review_cap", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_definition_spelling_round_trips() {
        for value in [ExecutionKind::Always, ExecutionKind::Conditional] {
            assert_eq!(
                execution_kind_of(execution_kind_spelling(value)).unwrap(),
                value
            );
        }
        for value in [
            StageMode::Inline,
            StageMode::Subagent,
            StageMode::Pipeline,
            StageMode::Mob,
            StageMode::AgentTeam,
        ] {
            assert_eq!(stage_mode_of(stage_mode_spelling(value)).unwrap(), value);
        }
        for value in [ReviewClass::Advisory, ReviewClass::Adversarial] {
            assert_eq!(
                review_class_of(review_class_spelling(value)).unwrap(),
                value
            );
        }
        for value in [
            RuleScope::Org,
            RuleScope::Team,
            RuleScope::Project,
            RuleScope::Phase,
        ] {
            assert_eq!(rule_scope_of(rule_scope_spelling(value)).unwrap(), value);
        }
        for value in [SkeletonDefault::On, SkeletonDefault::Off] {
            assert_eq!(
                skeleton_default_of(skeleton_default_spelling(value)).unwrap(),
                value
            );
        }
        for value in [
            ReviewCapValue::Adversarial,
            ReviewCapValue::Advisory,
            ReviewCapValue::None,
        ] {
            assert_eq!(review_cap_of(review_cap_spelling(value)).unwrap(), value);
        }
    }

    #[test]
    fn the_definition_journal_spellings_differ_from_the_stage_graph_face() {
        // 同じ値でも面が違えば綴りも違う — ドメイン側の `as_str` を流用しない理由である。
        assert_eq!(execution_kind_spelling(ExecutionKind::Always), "Always");
        assert_eq!(ExecutionKind::Always.as_str(), "ALWAYS");
        assert_eq!(stage_mode_spelling(StageMode::AgentTeam), "AgentTeam");
        assert_eq!(StageMode::AgentTeam.as_str(), "agent-team");
        assert_eq!(rule_scope_spelling(RuleScope::Org), "Org");
        assert_eq!(RuleScope::Org.as_str(), "org");
    }

    #[test]
    fn an_unknown_definition_spelling_is_refused_with_its_raw_value() {
        assert_eq!(
            execution_kind_of("ALWAYS").unwrap_err(),
            DtoDecodeError::malformed("execution", "ALWAYS")
        );
        assert!(stage_mode_of("agent-team").is_err());
        assert!(review_class_of("advisory").is_err());
        assert!(rule_scope_of("org").is_err());
        assert!(skeleton_default_of("on").is_err());
        assert!(review_cap_of("none").is_err());
    }

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
            DtoDecodeError::malformed("overlay", "EXECUTE")
        );
        assert!(checkbox_of("done").is_err());
        assert!(status_of("running").is_err());
        assert!(autonomy_of("gated").is_err());
        assert!(phase_of("ideation", "phase").is_err());
        assert!(project_type_of("Greenfield").is_err());
    }
}
