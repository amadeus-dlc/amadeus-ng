//! `ReviewFreezeBlock` — 凍結が拒否した 1 件の書込みの材料。
use super::{UnitName, WriteTarget};
use crate::workflow_definition::StageSlug;
use crate::workspace::{AuditFieldKey, AuditFieldKeyError, AuditFields};
/// 終端の受領証を無効化するため拒否した書込み。
///
/// 監査行 (`REVIEW_FREEZE_BLOCKED`) と拒否文言の材料をこの 1 つの値が持つ — 呼出側が
/// 対象・ステージ・Unit を別々に持ち回ると、どれか 1 つだけ取り違えた行が書ける。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewFreezeBlock {
    target: WriteTarget,
    stage: StageSlug,
    unit: Option<UnitName>,
}
impl ReviewFreezeBlock {
    /// 拒否した書込みの全材料を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        target: WriteTarget,
        stage: StageSlug,
        unit: Option<UnitName>,
    ) -> ReviewFreezeBlock {
        ReviewFreezeBlock {
            target,
            stage,
            unit,
        }
    }
    /// 拒否した書込み先。
    #[must_use]
    pub const fn target(&self) -> &WriteTarget {
        &self.target
    }
    /// 受領証を持つステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }
    /// 書込みが名指した Unit (ステージ水準の書込みでは `None`)。
    #[must_use]
    pub const fn unit(&self) -> Option<&UnitName> {
        self.unit.as_ref()
    }

    /// 拒否行の監査項目 (upstream `REVIEW_FREEZE_BLOCKED` の `Tool` / `Target` / `Stage` と、
    /// per-unit の書込みだけが名乗る `Unit`)。
    ///
    /// 項目の組み立てを**この値が所有する**のは、対象・ステージ・Unit を外へ取り出して
    /// 呼出側が組むと、別の書込みの Unit を載せた行が書けてしまうからである
    /// (`coding-rules/tell-dont-ask.md`)。
    ///
    /// # Errors
    /// 監査項目名の鋳造に失敗した場合 (4 つとも文法に合うが、失敗を握り潰さない)。
    pub fn audit_fields(&self, tool: &str) -> Result<AuditFields, AuditFieldKeyError> {
        let fields = AuditFields::new()
            .with(AuditFieldKey::parse("Tool")?, tool)
            .with(AuditFieldKey::parse("Target")?, self.target.as_str())
            .with(AuditFieldKey::parse("Stage")?, self.stage.as_str());
        match self.unit.as_ref() {
            Some(unit) => Ok(fields.with(AuditFieldKey::parse("Unit")?, unit.as_str())),
            None => Ok(fields),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ReviewFreezeBlock;
    use crate::orchestration::{UnitName, WriteTarget};
    use crate::workflow_definition::StageSlug;

    fn block(unit: Option<&str>) -> ReviewFreezeBlock {
        ReviewFreezeBlock::new(
            WriteTarget::parse("/w/x/inception/requirements-analysis/requirements.md").unwrap(),
            StageSlug::parse("requirements-analysis").unwrap(),
            unit.map(|unit| UnitName::parse(unit).unwrap()),
        )
    }

    fn rendered(block: &ReviewFreezeBlock, tool: &str) -> Vec<(String, String)> {
        block
            .audit_fields(tool)
            .unwrap()
            .iter()
            .map(|(key, value)| (key.as_str().to_string(), value.as_str().to_string()))
            .collect()
    }

    #[test]
    fn a_stage_level_block_names_three_fields_in_the_upstream_order() {
        assert_eq!(
            rendered(&block(None), "Write"),
            [
                ("Tool".to_string(), "Write".to_string()),
                (
                    "Target".to_string(),
                    "/w/x/inception/requirements-analysis/requirements.md".to_string()
                ),
                ("Stage".to_string(), "requirements-analysis".to_string()),
            ]
        );
    }

    #[test]
    fn a_unit_scoped_block_appends_the_unit_after_the_stage() {
        let fields = rendered(&block(Some("u2-workflow-authority")), "Edit");
        assert_eq!(fields.len(), 4);
        assert_eq!(
            fields.last().cloned(),
            Some(("Unit".to_string(), "u2-workflow-authority".to_string()))
        );
        assert_eq!(
            fields.first().map(|(_, value)| value.as_str()),
            Some("Edit")
        );
    }
}
