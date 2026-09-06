//! `PracticesPromotion` — 昇格 1 回が書き写す内容 (5 節の本文と、印を付けた規則行)。

use std::collections::BTreeSet;

use chrono::NaiveDate;

use super::markdown_sections::extract_section;
use super::promoted_section::PromotedSection;
use super::promoted_sections::PromotedSections;
use super::promotion_plan_error::PromotionPlanError;
use super::rule_lines::RuleLines;

/// team.md が持つ 5 節 (upstream `TEAM_SECTIONS` `:3622-3628` の順序と綴り)。
const TEAM_SECTIONS: [&str; 5] = [
    "## Way of Working",
    "## Walking Skeleton",
    "## Testing Posture",
    "## Deployment",
    "## Code Style",
];

/// 追記先の見出し 2 つ (upstream `:3659` / `:3663`)。
const MANDATED_HEADING: &str = "## Mandated";
const FORBIDDEN_HEADING: &str = "## Forbidden";

/// 昇格 1 回が書き写す内容。
///
/// **判断ではなく計算の結果**である — ドラフト 2 本と正本 2 本と日付から決まる純粋な値で、
/// 誰が承認したか・受け取ってよいかは集約 (`IntentExecution::affirm_practices`) が決める
/// (設計 §1)。空の昇格 ([`PracticesPromotion::default`]) も正規の値である — upstream は
/// 節も規則も無い昇格を `Sections Written: `（空）と 0 / 0 で受理する。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PracticesPromotion {
    sections: PromotedSections,
    mandated: RuleLines,
    forbidden: RuleLines,
}

impl PracticesPromotion {
    /// ドラフト 2 本と正本 2 本から昇格の内容を計算する (upstream Step 4a / 4b の写し)。
    ///
    /// - 5 節を順に見て、ドラフトに**本文のある**節があればそれを採る (不在・空は据え置き —
    ///   一部の実践だけを直す再実行のため)。正本 team.md にその見出しが無ければ拒否する。
    /// - 規則はドラフトの `## Mandated` / `## Forbidden` を行ごとに trim し、空行・
    ///   `<!--` 始まり・`#` 始まりを捨てる。各行に `(affirmed <today>)` の印を付け、
    ///   **正本に同じ印付き行が既にあれば除く** (at-least-once の再実行で重複しない)。
    ///   同じ実行内の重複も 1 つにする。
    /// - 追記する規則が 1 つも無い見出しは検査しない (upstream も `appendUnderHeading` を
    ///   呼ばないので throw しない)。
    ///
    /// # Errors
    ///
    /// 正本 team.md に置換先の見出しが無い ([`PromotionPlanError::TeamHeadingMissing`])、
    /// 正本 project.md に追記先の見出しが無い ([`PromotionPlanError::ProjectHeadingMissing`])、
    /// 置き換える節の見出しが重複した ([`PromotionPlanError::DuplicateSection`] — 見出しは固定
    /// 5 種を順に 1 度ずつ見るので構成不能だが、[`PromotedSections`] の構築検査を握り潰さない)。
    pub fn plan(
        team_practices_draft: &str,
        discovered_rules_draft: &str,
        team_md: &str,
        project_md: &str,
        today: NaiveDate,
    ) -> Result<PracticesPromotion, PromotionPlanError> {
        let mut sections = Vec::new();
        for heading in TEAM_SECTIONS {
            let Some(body) =
                extract_section(team_practices_draft, heading).filter(|body| !body.is_empty())
            else {
                continue;
            };
            if extract_section(team_md, heading).is_none() {
                return Err(PromotionPlanError::TeamHeadingMissing(heading.to_string()));
            }
            // 見出し名は `## ` を除いた裸の名前 (upstream `heading.slice(3)`)。
            let name = heading.get(3..).unwrap_or(heading);
            sections.push(PromotedSection::new(name, body));
        }

        // 既存行の集合は**両見出しで共有**する — upstream の `existingGuardrailLines` は
        // 1 つの Set であり、Mandated で足した行は Forbidden の重複判定にも効く。
        let mut existing: BTreeSet<String> = project_md
            .split('\n')
            .map(|line| line.trim().to_string())
            .collect();
        let mandated = stamp_rules(
            discovered_rules_draft,
            MANDATED_HEADING,
            today,
            &mut existing,
        );
        let forbidden = stamp_rules(
            discovered_rules_draft,
            FORBIDDEN_HEADING,
            today,
            &mut existing,
        );
        for (heading, rules) in [
            (MANDATED_HEADING, &mandated),
            (FORBIDDEN_HEADING, &forbidden),
        ] {
            if !rules.is_empty() && extract_section(project_md, heading).is_none() {
                return Err(PromotionPlanError::ProjectHeadingMissing(
                    heading.to_string(),
                ));
            }
        }

        Ok(PracticesPromotion {
            sections: PromotedSections::new(sections)?,
            mandated: RuleLines::new(mandated),
            forbidden: RuleLines::new(forbidden),
        })
    }

    /// 置き換える節 (team.md の書込順)。
    #[must_use]
    pub const fn sections(&self) -> &PromotedSections {
        &self.sections
    }

    /// `## Mandated` へ足す印付きの規則行。
    #[must_use]
    pub const fn mandated(&self) -> &RuleLines {
        &self.mandated
    }

    /// `## Forbidden` へ足す印付きの規則行。
    #[must_use]
    pub const fn forbidden(&self) -> &RuleLines {
        &self.forbidden
    }

    /// 書き替えた節の見出し名の列 (stdout の `sections_written` / 監査行の材料)。
    #[must_use]
    pub fn sections_written(&self) -> Vec<&str> {
        self.sections
            .fold_left(Vec::new(), |mut headings, section| {
                headings.push(section.heading());
                headings
            })
    }

    /// `## Mandated` へ足した規則の件数。
    #[must_use]
    pub fn mandated_appended(&self) -> u32 {
        count(self.mandated.len())
    }

    /// `## Forbidden` へ足した規則の件数。
    #[must_use]
    pub fn forbidden_appended(&self) -> u32 {
        count(self.forbidden.len())
    }
}

/// 件数を数える (実運用で溢れない規模だが、境界を暗黙の切り捨てにしない — NFR4.3)。
fn count(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}

/// ドラフトの 1 節から規則行を拾い、印を付け、既出を除く。
fn stamp_rules(
    draft: &str,
    heading: &str,
    today: NaiveDate,
    existing: &mut BTreeSet<String>,
) -> Vec<String> {
    let stamp = today.format("%Y-%m-%d").to_string();
    let Some(section) = extract_section(draft, heading) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in section.split('\n') {
        let rule = line.trim();
        if rule.is_empty() || rule.starts_with("<!--") || rule.starts_with('#') {
            continue;
        }
        let stamped = format!("{rule} (affirmed {stamp})");
        if existing.insert(stamped.clone()) {
            out.push(stamped);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 5).expect("固定の日付")
    }

    const TEAM_MD: &str = "\
# Team

## Way of Working
old.

## Walking Skeleton
old.

## Testing Posture
old.

## Deployment
old.

## Code Style
old.
";

    const PROJECT_MD: &str = "\
# Project

## Mandated

## Forbidden

## Corrections
";

    #[test]
    fn only_the_sections_the_draft_carries_are_written() {
        let draft = "## Way of Working\nnew.\n\n## Code Style\nrustfmt.\n";
        let promotion = PracticesPromotion::plan(draft, "", TEAM_MD, PROJECT_MD, today()).unwrap();
        assert_eq!(
            promotion.sections_written(),
            vec!["Way of Working", "Code Style"]
        );
        assert_eq!(
            promotion.sections().at(0).map(PromotedSection::body),
            Some("new.\n\n")
        );
    }

    /// 空の節はドラフトに「無い」のと同じ扱いである (正本を触らない)。
    #[test]
    fn an_empty_draft_section_leaves_the_live_section_alone() {
        let draft = "## Way of Working\n## Code Style\nrustfmt.\n";
        let promotion = PracticesPromotion::plan(draft, "", TEAM_MD, PROJECT_MD, today()).unwrap();
        assert_eq!(promotion.sections_written(), vec!["Code Style"]);
    }

    /// 節も規則も無い昇格は受理する (upstream は 0 / 0 で emit する)。
    #[test]
    fn an_empty_promotion_is_accepted() {
        let promotion = PracticesPromotion::plan("", "", TEAM_MD, PROJECT_MD, today()).unwrap();
        assert_eq!(promotion, PracticesPromotion::default());
        assert!(promotion.sections_written().is_empty());
        assert_eq!(promotion.mandated_appended(), 0);
        assert_eq!(promotion.forbidden_appended(), 0);
    }

    #[test]
    fn rules_are_trimmed_stamped_and_stripped_of_comments_and_headings() {
        let draft = "\
## Mandated
  ALWAYS ship tests.  
<!-- a comment -->
### not a rule

ALWAYS review.

## Forbidden
NEVER force-push.
";
        let promotion = PracticesPromotion::plan("", draft, TEAM_MD, PROJECT_MD, today()).unwrap();
        assert_eq!(
            promotion.mandated(),
            &RuleLines::new(vec![
                "ALWAYS ship tests. (affirmed 2026-09-05)".to_string(),
                "ALWAYS review. (affirmed 2026-09-05)".to_string(),
            ])
        );
        assert_eq!(
            promotion.forbidden(),
            &RuleLines::new(vec!["NEVER force-push. (affirmed 2026-09-05)".to_string()])
        );
        assert_eq!(promotion.mandated_appended(), 2);
        assert_eq!(promotion.forbidden_appended(), 1);
    }

    /// 正本に同じ印付き行が既にあれば足さない (再実行が重複を積まない)。
    #[test]
    fn a_rule_already_stamped_in_the_live_file_is_not_appended_again() {
        let draft = "## Mandated\nALWAYS review.\n";
        let live =
            "# Project\n\n## Mandated\nALWAYS review. (affirmed 2026-09-05)\n\n## Forbidden\n";
        let promotion = PracticesPromotion::plan("", draft, TEAM_MD, live, today()).unwrap();
        assert!(promotion.mandated().is_empty());
        assert_eq!(promotion.mandated_appended(), 0);
    }

    /// 同じドラフト内の重複も 1 つにする。
    #[test]
    fn a_rule_repeated_inside_one_draft_is_appended_once() {
        let draft = "## Mandated\nALWAYS review.\nALWAYS review.\n";
        let promotion = PracticesPromotion::plan("", draft, TEAM_MD, PROJECT_MD, today()).unwrap();
        assert_eq!(
            promotion.mandated(),
            &RuleLines::new(vec!["ALWAYS review. (affirmed 2026-09-05)".to_string()])
        );
    }

    #[test]
    fn a_missing_team_heading_is_refused_with_its_spelling() {
        let draft = "## Deployment\nnone.\n";
        let live = "# Team\n\n## Way of Working\nold.\n";
        assert_eq!(
            PracticesPromotion::plan(draft, "", live, PROJECT_MD, today()).unwrap_err(),
            PromotionPlanError::TeamHeadingMissing("## Deployment".to_string())
        );
    }

    #[test]
    fn a_missing_project_heading_is_refused_only_when_there_is_a_rule_to_append() {
        let live = "# Project\n\n## Corrections\n";
        assert_eq!(
            PracticesPromotion::plan("", "## Mandated\nALWAYS x.\n", TEAM_MD, live, today())
                .unwrap_err(),
            PromotionPlanError::ProjectHeadingMissing("## Mandated".to_string())
        );
        assert_eq!(
            PracticesPromotion::plan("", "## Forbidden\nNEVER y.\n", TEAM_MD, live, today())
                .unwrap_err(),
            PromotionPlanError::ProjectHeadingMissing("## Forbidden".to_string())
        );
        // 足す規則が無ければ見出しの不在は問わない。
        assert!(PracticesPromotion::plan("", "", TEAM_MD, live, today()).is_ok());
    }
}
