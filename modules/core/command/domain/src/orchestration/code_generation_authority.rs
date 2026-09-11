//! 計画承認を現在の指示発行へ結び付ける値。
use super::{ActiveDirective, CodeGenerationRunFloor, PlanApprovalError, PublishedDirective};
use core_infrastructure::canon_json::{JsonValue, Number, ObjectMembers, hash_canonical};
/// 計画承認の対象と、発行時点の識別子。文書だけでは構築しない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeGenerationAuthority {
    directive_epoch: String,
    target_id: String,
    intent_id: String,
    unit: Option<String>,
    run_floor: String,
    source_floor: String,
    marker_revision: u64,
}
impl CodeGenerationAuthority {
    const fn of_values(
        directive_epoch: String,
        target_id: String,
        intent_id: String,
        unit: Option<String>,
        run_floor: String,
        source_floor: String,
        marker_revision: u64,
    ) -> Self {
        Self {
            directive_epoch,
            target_id,
            intent_id,
            unit,
            run_floor,
            source_floor,
            marker_revision,
        }
    }

    /// 記録された発行の値を、対象と指紋の不変条件を確認して組む。
    /// # Errors
    /// 発行識別子・ソース基準・実行境界が不正な場合。
    pub fn new(
        target: &super::PlanTarget,
        intent: &super::IntentId,
        epoch: String,
        run_floor: String,
        source_floor: String,
        revision: u64,
    ) -> Result<Self, PlanApprovalError> {
        let hexadecimal = |value: &str| {
            value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        };
        let epoch_valid = epoch
            .strip_prefix("sha256:")
            .is_some_and(|hex| hex.len() == 64 && hexadecimal(hex));
        let source_valid = source_floor == "unbindable"
            || (matches!(source_floor.len(), 40 | 64) && hexadecimal(&source_floor));
        let floor_valid = run_floor == "unstarted#0"
            || run_floor.rsplit_once('#').is_some_and(|(boundary, count)| {
                count.parse::<u64>().is_ok_and(|count| count > 0)
                    && boundary.split_once(':').is_some_and(|(kind, time)| {
                        matches!(
                            kind,
                            "WORKFLOW_STARTED" | "STAGE_STARTED" | "STAGE_JUMPED" | "GATE_REJECTED"
                        ) && chrono::DateTime::parse_from_rfc3339(time).is_ok()
                    })
            });
        if !epoch_valid || !source_valid || !floor_valid {
            return Err(PlanApprovalError::new(
                "invalid recorded Code Generation authority",
            ));
        }
        Ok(Self::of_values(
            epoch,
            target.id(),
            intent.as_str().to_string(),
            target.unit().map(str::to_string),
            run_floor,
            source_floor,
            revision,
        ))
    }

    /// 発行済み指示と集約の実行境界から承認対象を解決する。
    /// # Errors
    /// 指示が欠落、対象不一致、または発行が古い場合。
    pub fn resolve(
        directive: Option<&ActiveDirective>,
        floor: Option<&CodeGenerationRunFloor>,
        unit: Option<&str>,
        state_sha256: Option<&str>,
    ) -> Result<Self, PlanApprovalError> {
        let state = state_sha256.ok_or_else(|| {
            PlanApprovalError::new(
                "Code Generation approval authority requires an active workflow state",
            )
        })?;
        let unavailable = || {
            PlanApprovalError::new(
                "Code Generation approval authority is unavailable because the active directive is missing, stale, or legacy; run a fresh `next`",
            )
        };
        let directive = directive
            .filter(|directive| directive.state_sha256() == state)
            .ok_or_else(unavailable)?;
        let floor = floor.ok_or_else(unavailable)?;
        let stage = directive.directive().stage().as_str();
        if stage != "code-generation" {
            return Err(PlanApprovalError::new(format!(
                "Code Generation approval authority does not match active directive stage \"{stage}\""
            )));
        }
        let PublishedDirective::RunStage {
            unit: issued_unit, ..
        } = directive.directive()
        else {
            let kind = if matches!(directive.directive(), PublishedDirective::Error { .. }) {
                "error"
            } else {
                "load-steering"
            };
            return Err(PlanApprovalError::new(format!(
                "Code Generation approval authority requires a run-stage or invoke-swarm directive, got \"{kind}\""
            )));
        };
        if let Some(unit) = unit {
            if issued_unit.as_deref() != Some(unit) {
                return Err(PlanApprovalError::new(format!(
                    "Code Generation approval target unit \"{unit}\" does not match active directive unit \"{}\"",
                    issued_unit.as_deref().unwrap_or("(none)")
                )));
            }
        } else if issued_unit.is_some() {
            return Err(PlanApprovalError::new(
                "Stage-level Code Generation approval requires a zero-Unit run-stage directive",
            ));
        }
        let target_id = unit.map_or_else(
            || "stage:code-generation".to_string(),
            |unit| format!("unit:{unit}"),
        );
        let source_floor = directive.source_floor().unwrap_or("unbindable").to_string();
        let text = |text: &str| JsonValue::String(text.to_string());
        let mut fields = ObjectMembers::new();
        fields.insert("version", JsonValue::Number(Number::PosInt(2)));
        fields.insert("project", text(directive.project_sha256()));
        fields.insert("intent", text(directive.intent_id().as_str()));
        fields.insert("state", text(directive.state_sha256()));
        fields.insert("stage", text(stage));
        fields.insert(
            "directive_unit",
            issued_unit.as_deref().map_or(JsonValue::Null, text),
        );
        fields.insert("kind", text("run-stage"));
        fields.insert(
            "issuance_revision",
            JsonValue::Number(Number::PosInt(directive.revision())),
        );
        fields.insert(
            "owner_epoch",
            JsonValue::Number(Number::PosInt(directive.owner_epoch())),
        );
        fields.insert(
            "context_epoch",
            JsonValue::Number(Number::PosInt(directive.context_epoch())),
        );
        fields.insert("continue_token", JsonValue::Null);
        fields.insert("target", text(&target_id));
        fields.insert("source_floor", text(&source_floor));
        let directive_epoch = hash_canonical(&JsonValue::Object(fields)).rendered();
        Ok(Self::of_values(
            directive_epoch,
            target_id,
            directive.intent_id().as_str().to_string(),
            unit.map(str::to_string),
            floor.rendered(),
            source_floor,
            directive.revision(),
        ))
    }
    /// 計画・テスト指示・解決済み契約を、この承認対象へ束縛する。
    #[must_use]
    pub fn approval_fingerprint(
        &self,
        plan: &str,
        instructions: &str,
        contract_hash: &str,
    ) -> String {
        let mut fields = ObjectMembers::new();
        for (name, value) in [
            ("plan", plan),
            ("instructions", instructions),
            ("testing_contract", contract_hash),
            ("target", self.target_id()),
            ("intent", self.intent_id()),
            ("directive_epoch", self.directive_epoch()),
            ("run_floor", self.run_floor()),
            ("source_floor", self.source_floor()),
        ] {
            fields.insert(name, JsonValue::String(value.to_string()));
        }
        hash_canonical(&JsonValue::Object(fields)).rendered()
    }

    /// 指示の発行エポック。
    #[must_use]
    pub fn directive_epoch(&self) -> &str {
        &self.directive_epoch
    }
    /// 承認の対象。
    #[must_use]
    pub fn target_id(&self) -> &str {
        &self.target_id
    }
    /// 依頼の識別子。
    #[must_use]
    pub fn intent_id(&self) -> &str {
        &self.intent_id
    }
    /// 個別作業単位。段階全体ではNone。
    #[must_use]
    pub fn unit(&self) -> Option<&str> {
        self.unit.as_deref()
    }
    /// 試行の境界。
    #[must_use]
    pub fn run_floor(&self) -> &str {
        &self.run_floor
    }
    /// 指示が束縛したソースの基準。
    #[must_use]
    pub fn source_floor(&self) -> &str {
        &self.source_floor
    }
    /// 発行回数。
    #[must_use]
    pub const fn marker_revision(&self) -> u64 {
        self.marker_revision
    }
}
