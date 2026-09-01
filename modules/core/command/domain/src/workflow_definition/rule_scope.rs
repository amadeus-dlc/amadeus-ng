//! `RuleScope` — `rules_in_context[].scope` の閉集合。

use super::unknown_rule_scope::UnknownRuleScope;

/// `rules_in_context[].scope` の閉集合 (upstream 08 §110-119)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuleScope {
    /// `memory/org.md` 由来。全ステージへ無条件に載る (upstream 08 §3.2)。
    Org,
    /// `memory/team.md` 由来。同じく無条件。
    Team,
    /// `memory/project.md` 由来。同じく無条件。
    Project,
    /// `memory/phases/<phase>.md` 由来。ステージの `phase` と一致するときだけ載る。
    Phase,
}

impl RuleScope {
    /// 宣言順の全値 (4 値の網羅走査の正本)。並びは広い層から狭い層への 4 層チェーン。
    pub const ALL: [RuleScope; 4] = [
        RuleScope::Org,
        RuleScope::Team,
        RuleScope::Project,
        RuleScope::Phase,
    ];

    /// # Errors
    ///
    /// 4 値以外は `UnknownRuleScope` で拒否する。
    pub fn parse(s: &str) -> Result<RuleScope, UnknownRuleScope> {
        Ok(match s {
            "org" => RuleScope::Org,
            "team" => RuleScope::Team,
            "project" => RuleScope::Project,
            "phase" => RuleScope::Phase,
            other => return Err(UnknownRuleScope::new(other)),
        })
    }

    /// `stage-graph.json` 上の正準綴り (`parse` の逆写像)。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            RuleScope::Org => "org",
            RuleScope::Team => "team",
            RuleScope::Project => "project",
            RuleScope::Phase => "phase",
        }
    }
}
