//! `RuleScopeView` — `rules_in_context[].scope` の閉集合 (upstream 08 §110-119)。

use super::unknown_value::UnknownValue;

/// ルール行がどの層のものか。並びは広い層から狭い層への 4 層チェーン。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuleScopeView {
    /// `memory/org.md` 由来。全ステージへ無条件に載る。
    Org,
    /// `memory/team.md` 由来。同じく無条件。
    Team,
    /// `memory/project.md` 由来。同じく無条件。
    Project,
    /// `memory/phases/<phase>.md` 由来。ステージの `phase` と一致するときだけ載る。
    Phase,
}

impl RuleScopeView {
    /// 宣言順の全値 (4 値の網羅走査の正本)。並びは広い層から狭い層への 4 層チェーン。
    pub const ALL: [RuleScopeView; 4] = [
        RuleScopeView::Org,
        RuleScopeView::Team,
        RuleScopeView::Project,
        RuleScopeView::Phase,
    ];

    /// # Errors
    ///
    /// 4 値以外は [`UnknownValue`] で拒否する。
    pub fn parse(s: &str) -> Result<RuleScopeView, UnknownValue> {
        Ok(match s {
            "org" => RuleScopeView::Org,
            "team" => RuleScopeView::Team,
            "project" => RuleScopeView::Project,
            "phase" => RuleScopeView::Phase,
            other => return Err(UnknownValue::new(other)),
        })
    }

    /// `stage-graph.json` 上の正準綴り (`parse` の逆写像)。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            RuleScopeView::Org => "org",
            RuleScopeView::Team => "team",
            RuleScopeView::Project => "project",
            RuleScopeView::Phase => "phase",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_layers_round_trip_and_unknown_is_rejected() {
        for r in RuleScopeView::ALL {
            assert_eq!(RuleScopeView::parse(r.as_str()).unwrap(), r);
        }
        let rejected = RuleScopeView::parse("space").unwrap_err();
        assert_eq!(rejected.as_str(), "space");
    }
}
