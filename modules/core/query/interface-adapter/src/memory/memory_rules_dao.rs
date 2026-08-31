//! `MemoryRulesDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{MemoryRules, MemoryRulesDao, MemoryRulesReadError};

/// memory 層ルール束の読取結果を握るダブル。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryMemoryRulesDao {
    held: Result<MemoryRules, MemoryRulesReadError>,
}

impl InMemoryMemoryRulesDao {
    /// 読めたルール束 (空も正常 — ルール未整備は bare run-stage)。
    #[must_use]
    pub const fn holding(rules: MemoryRules) -> InMemoryMemoryRulesDao {
        InMemoryMemoryRulesDao { held: Ok(rules) }
    }

    /// 必須ルールファイルが在るのに読めない。
    #[must_use]
    pub const fn failing(error: MemoryRulesReadError) -> InMemoryMemoryRulesDao {
        InMemoryMemoryRulesDao { held: Err(error) }
    }
}

impl MemoryRulesDao for InMemoryMemoryRulesDao {
    fn find(&self) -> Result<MemoryRules, MemoryRulesReadError> {
        self.held.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_query_use_case::orchestration::RuleContent;
    use core_query_use_case::workflow_view::PhaseView;
    use std::collections::BTreeMap;

    #[test]
    fn the_double_replays_whatever_it_was_given() {
        let rules = MemoryRules::new(
            vec![RuleContent::new(
                "memory/org.md".to_string(),
                "# Org\n".to_string(),
            )],
            BTreeMap::new(),
        );
        let held = InMemoryMemoryRulesDao::holding(rules).find().unwrap();
        assert_eq!(
            held.plan_for(PhaseView::Inception)
                .unwrap()
                .delivered_paths(),
            ["memory/org.md"]
        );

        let error =
            MemoryRulesReadError::new("memory/org.md".to_string(), "permission denied".to_string());
        assert_eq!(
            InMemoryMemoryRulesDao::failing(error.clone())
                .find()
                .unwrap_err(),
            error
        );
    }

    #[test]
    fn an_empty_bundle_is_a_normal_answer() {
        let held = InMemoryMemoryRulesDao::holding(MemoryRules::default())
            .find()
            .unwrap();
        assert!(held.plan_for(PhaseView::Inception).unwrap().is_empty());
    }
}
