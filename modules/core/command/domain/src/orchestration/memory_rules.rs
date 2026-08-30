//! `MemoryRules` — active-space の memory 層 (決定論的 steering) の読取済みルール束。
//!
//! ファイルの読取 (I/O) は合成ルートの loader が行い、本型は**読み終えた値**を運ぶ
//! (issue #46 — 旧 `RuleBundleSource` ポートの廃止。use-case のポートは Repository を
//! 目指して縮める)。フェーズの選択と配信計画への分割・パックはドメインの純計算であり、
//! [`MemoryRules::plan_for`] → [`SteeringPlan::pack`] が行う。
//!
//! 読み順は memory 層の解決順 `org → team → project → phases/<phase>` (strict-additive) —
//! `base` の並びは loader が解決順で渡し、本型はその順序を保持して phase ルールを後置する。

use std::collections::BTreeMap;

use super::directive::RuleContent;
use super::steering_plan::{SteeringPlan, UnsplittableSection};
use crate::workflow_definition::PhaseId;

/// 読取済みの memory 層ルール束 (base 3 ファイル + フェーズ別ファイル)。
///
/// ファイルが**無い**のは正常 (ルール未整備・initialization はフェーズルールを持たない) —
/// 無いファイルは単に列に現れない。「在るのに読めない」は loader が blocking で止めるので、
/// 本型に失敗の表現は無い (Always Valid)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemoryRules {
    base: Vec<RuleContent>,
    phases: BTreeMap<PhaseId, RuleContent>,
}

impl MemoryRules {
    /// 読み終えたルール束を組む — `base` は解決順 (`org → team → project`)、`phases` は
    /// フェーズ別ファイル (存在したものだけ)。
    #[must_use]
    pub const fn new(
        base: Vec<RuleContent>,
        phases: BTreeMap<PhaseId, RuleContent>,
    ) -> MemoryRules {
        MemoryRules { base, phases }
    }

    /// 実行フェーズに応じたルール束を読み順で配信計画に組む。空 (ルール未整備) は空計画で
    /// 正常 (bare run-stage)。
    ///
    /// # Errors
    ///
    /// 分割不能セクション (`UnsplittableSection`) — 呼出側は run-stage の代わりに `error`
    /// directive を出す (02 §10)。
    pub fn plan_for(&self, phase: PhaseId) -> Result<SteeringPlan, UnsplittableSection> {
        let mut files: Vec<RuleContent> = self.base.clone();
        if let Some(rule) = self.phases.get(&phase) {
            files.push(rule.clone());
        }
        SteeringPlan::pack(&files)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn content(path: &str, text: &str) -> RuleContent {
        RuleContent::new(path.to_string(), text.to_string())
    }

    #[test]
    fn the_plan_reads_base_then_the_phase_rule_in_resolution_order() {
        let rules = MemoryRules::new(
            vec![content("memory/org.md", "# Org\n")],
            [(
                PhaseId::Inception,
                content("memory/phases/inception.md", "# Inception\n"),
            )]
            .into_iter()
            .collect(),
        );
        let plan = rules.plan_for(PhaseId::Inception).unwrap();
        let pieces: Vec<_> = plan.chunks().iter().flatten().collect();
        assert_eq!(pieces.len(), 2);
        assert_eq!(pieces.first().unwrap().path(), "memory/org.md");
        assert_eq!(
            pieces.get(1).unwrap().path(),
            "memory/phases/inception.md",
            "フェーズルールは base の後 (strict-additive)"
        );
    }

    #[test]
    fn a_phase_without_a_rule_file_plans_the_base_only() {
        let rules = MemoryRules::new(vec![content("memory/org.md", "# Org\n")], BTreeMap::new());
        let plan = rules.plan_for(PhaseId::Initialization).unwrap();
        assert_eq!(plan.chunks().iter().flatten().count(), 1);
    }

    #[test]
    fn an_empty_memory_layer_plans_nothing() {
        assert!(
            MemoryRules::default()
                .plan_for(PhaseId::Inception)
                .unwrap()
                .is_empty()
        );
    }
}
