//! `ReviewerScopeBlock` — 読み取り範囲が拒否した 1 件の呼出しの材料。
use super::{InspectedTool, ReviewedStage, ReviewedUnit, ScopeToken};
use crate::workspace::{AuditFieldKey, AuditFieldKeyError, AuditFields};

/// 兄弟 Unit へ届くため拒否した呼出し。
///
/// 監査行 (`REVIEWER_SCOPE_BLOCKED`) と拒否文言の材料をこの 1 つの値が持つ — 呼出側が
/// 工具・対象・ステージ・Unit を別々に持ち回ると、どれか 1 つだけ取り違えた行が書ける
/// (`coding-rules/tell-dont-ask.md`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewerScopeBlock {
    tool: InspectedTool,
    target: ScopeToken,
    stage: ReviewedStage,
    unit: ReviewedUnit,
}
impl ReviewerScopeBlock {
    /// 拒否した呼出しの全材料を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        tool: InspectedTool,
        target: ScopeToken,
        stage: ReviewedStage,
        unit: ReviewedUnit,
    ) -> ReviewerScopeBlock {
        ReviewerScopeBlock {
            tool,
            target,
            stage,
            unit,
        }
    }
    /// 拒否した呼出しの工具。
    #[must_use]
    pub const fn tool(&self) -> InspectedTool {
        self.tool
    }
    /// 越境した綴り。
    #[must_use]
    pub const fn target(&self) -> &ScopeToken {
        &self.target
    }
    /// 差し向け記録が名乗るステージ (記録の綴りのまま)。
    #[must_use]
    pub const fn stage(&self) -> &ReviewedStage {
        &self.stage
    }
    /// レビュー対象の Unit (記録の綴りのまま)。
    #[must_use]
    pub const fn unit(&self) -> &ReviewedUnit {
        &self.unit
    }

    /// 拒否行の監査項目 (upstream `REVIEWER_SCOPE_BLOCKED` の `Tool` / `Target` /
    /// `Stage` / `Unit` — 4 つとも必須)。
    ///
    /// # Errors
    /// 監査項目名の鋳造に失敗した場合 (4 つとも文法に合うが、失敗を握り潰さない)。
    pub fn audit_fields(&self) -> Result<AuditFields, AuditFieldKeyError> {
        Ok(AuditFields::new()
            .with(AuditFieldKey::parse("Tool")?, self.tool.as_str())
            .with(AuditFieldKey::parse("Target")?, self.target.as_str())
            .with(AuditFieldKey::parse("Stage")?, self.stage.as_str())
            .with(AuditFieldKey::parse("Unit")?, self.unit.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::ReviewerScopeBlock;
    use crate::orchestration::{InspectedTool, ReviewedStage, ReviewedUnit, ScopeToken};

    #[test]
    fn a_block_names_four_fields_in_the_upstream_order() {
        let block = ReviewerScopeBlock::new(
            InspectedTool::parse("Read").unwrap(),
            ScopeToken::parse("/r/construction/u2-beta/design.md").unwrap(),
            ReviewedStage::new("functional-design".to_string()),
            ReviewedUnit::parse("u1-alpha").unwrap(),
        );
        let rendered: Vec<(String, String)> = block
            .audit_fields()
            .unwrap()
            .iter()
            .map(|(key, value)| (key.as_str().to_string(), value.as_str().to_string()))
            .collect();
        assert_eq!(
            rendered,
            [
                ("Tool".to_string(), "Read".to_string()),
                (
                    "Target".to_string(),
                    "/r/construction/u2-beta/design.md".to_string()
                ),
                ("Stage".to_string(), "functional-design".to_string()),
                ("Unit".to_string(), "u1-alpha".to_string()),
            ]
        );
    }

    #[test]
    fn the_stage_and_unit_are_written_exactly_as_the_dispatch_record_spelled_them() {
        // upstream `emitReviewerScopeBlocked` は差し向け記録の値をそのまま渡す — 空の stage も
        // 区切りを含む unit も、監査の行にはその綴りで現れる (hook-differential c24 / c25)。
        let block = ReviewerScopeBlock::new(
            InspectedTool::parse("Read").unwrap(),
            ScopeToken::parse("/r/construction/u2-beta/design.md").unwrap(),
            ReviewedStage::new(String::new()),
            ReviewedUnit::parse("a/b").unwrap(),
        );
        let rendered: Vec<(String, String)> = block
            .audit_fields()
            .unwrap()
            .iter()
            .map(|(key, value)| (key.as_str().to_string(), value.as_str().to_string()))
            .collect();
        assert_eq!(
            rendered.get(2..4),
            Some(
                [
                    ("Stage".to_string(), String::new()),
                    ("Unit".to_string(), "a/b".to_string()),
                ]
                .as_slice()
            )
        );
    }
}
