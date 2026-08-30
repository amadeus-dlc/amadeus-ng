//! ジャーナル行における閉集合の**綴りの正本** (読む側の写し)。
//!
//! ドメイン側の `as_str` / `parse` は流用しない — 同じ値でも面ごとに綴りが違うからである
//! (例: `PhaseId` はジャーナル上 `"Ideation"` だが `stage-graph.json` 上は `"ideation"`)。
//! 書く側の同名モジュールとも共有しない (`mod.rs` の側ごと専用化)。両者が一致していることは
//! 横断適合テストが固定する。

use core_command_domain::orchestration::AutonomyMode;
use core_command_domain::workflow_definition::{BrownfieldGreenfield, PhaseId, PlanAction};

use super::wire_error::WireDecodeError;

/// 実効プラン 1 要素の綴り。
pub(crate) const fn plan_action_spelling(value: PlanAction) -> &'static str {
    match value {
        PlanAction::Execute => "Execute",
        PlanAction::Skip => "Skip",
    }
}

/// 実効プラン 1 要素の復号。
pub(crate) fn plan_action_of(
    raw: &str,
    field: &'static str,
) -> Result<PlanAction, WireDecodeError> {
    match raw {
        "Execute" => Ok(PlanAction::Execute),
        "Skip" => Ok(PlanAction::Skip),
        other => Err(WireDecodeError::malformed(field, other)),
    }
}

/// 自律モードの綴り。
pub(crate) const fn autonomy_spelling(value: AutonomyMode) -> &'static str {
    match value {
        AutonomyMode::Autonomous => "Autonomous",
        AutonomyMode::Gated => "Gated",
    }
}

/// 自律モードの復号。
pub(crate) fn autonomy_of(raw: &str) -> Result<AutonomyMode, WireDecodeError> {
    match raw {
        "Autonomous" => Ok(AutonomyMode::Autonomous),
        "Gated" => Ok(AutonomyMode::Gated),
        other => Err(WireDecodeError::malformed("autonomy", other)),
    }
}

/// フェーズの綴り (**ジャーナル面**。`stage-graph.json` 面の小文字とは別物)。
pub(crate) const fn phase_spelling(value: PhaseId) -> &'static str {
    match value {
        PhaseId::Initialization => "Initialization",
        PhaseId::Ideation => "Ideation",
        PhaseId::Inception => "Inception",
        PhaseId::Construction => "Construction",
        PhaseId::Operation => "Operation",
    }
}

/// フェーズの復号 (ジャーナル面)。
pub(crate) fn phase_of(raw: &str, field: &'static str) -> Result<PhaseId, WireDecodeError> {
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
pub(crate) const fn project_type_spelling(value: BrownfieldGreenfield) -> &'static str {
    match value {
        BrownfieldGreenfield::Brownfield => "brownfield",
        BrownfieldGreenfield::Greenfield => "greenfield",
    }
}

/// プロジェクト種別の復号。
pub(crate) fn project_type_of(raw: &str) -> Result<BrownfieldGreenfield, WireDecodeError> {
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
            WireDecodeError::malformed("overlay", "EXECUTE")
        );
        assert!(autonomy_of("gated").is_err());
        assert!(phase_of("ideation", "phase").is_err());
        assert!(project_type_of("Greenfield").is_err());
    }
}
