//! `RuleInContext` の永続化 DTO (**読む側**) — 文脈に載る規則 1 件の行の形。

use core_command_domain::workflow_definition::RuleInContext;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::dto_vocabulary::{rule_scope_of, rule_scope_spelling};

/// 文脈に載る規則 1 件の行の形。**フィールド名と並びが契約**である。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct RuleInContextDto {
    path: String,
    scope: String,
}

impl RuleInContextDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き — テストが行を用意する口)。
    pub(super) fn of(rule: &RuleInContext) -> RuleInContextDto {
        RuleInContextDto {
            path: rule.path().to_string(),
            scope: rule_scope_spelling(rule.scope()).to_string(),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<RuleInContext, DtoDecodeError> {
        Ok(RuleInContext::new(
            self.path.clone(),
            rule_scope_of(&self.scope)?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_command_domain::workflow_definition::RuleScope;

    #[test]
    fn the_rule_survives_the_round_trip() {
        for scope in [
            RuleScope::Org,
            RuleScope::Team,
            RuleScope::Project,
            RuleScope::Phase,
        ] {
            let rule = RuleInContext::new("org.md", scope);
            assert_eq!(RuleInContextDto::of(&rule).to_domain().unwrap(), rule);
        }
    }

    #[test]
    fn an_unknown_scope_spelling_is_refused() {
        let dto = RuleInContextDto {
            path: "org.md".to_string(),
            scope: "org".to_string(),
        };
        assert_eq!(
            dto.to_domain().unwrap_err(),
            DtoDecodeError::malformed("rule_scope", "org")
        );
    }
}
