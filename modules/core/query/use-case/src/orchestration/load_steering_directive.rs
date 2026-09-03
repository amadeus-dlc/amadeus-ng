//! `LoadSteeringDirective` — `load-steering` (ルール束の分割配信 1 部、02 §4.1)。
//!
//! クロスフィールド規則 `part <= parts` は**コンストラクタで**強制する
//! (`aidlc-directive.ts:603-611` — validateDirective 相当を型で行う)。
//!
//! `continue_token` は**中身 (型付き [`ContinueToken`])** を運ぶ — HMAC 封緘した base64url
//! 文字列にするのは出力境界 (U7 Presenter) の仕事である。

use super::bundle_digest::BundleDigest;
use super::continue_token::ContinueToken;
use super::part_count::PartCount;
use super::part_index::PartIndex;
use super::rule_content::RuleContent;
use crate::orchestration::StageSlugView;

/// `load-steering` — ルール束の分割配信 1 部 (02 §4.1)。
///
/// クロスフィールド規則 `part <= parts` は**コンストラクタで**強制する
/// (`aidlc-directive.ts:603-611` — validateDirective 相当を型で行う)。
///
/// `continue_token` は**中身 (型付き [`ContinueToken`])** を運ぶ — HMAC 封緘した base64url
/// 文字列にするのは出力境界 (U7 Presenter) の仕事である。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadSteeringDirective {
    stage: StageSlugView,
    bundle: BundleDigest,
    part: PartIndex,
    parts: PartCount,
    rules_content: Vec<RuleContent>,
    continue_token: ContinueToken,
}

impl LoadSteeringDirective {
    /// 1 部を組む (**この型の唯一の構築経路**)。
    ///
    /// 材料はすべて行の写しである — 索引と総数は `read_steering_part.part_index` /
    /// `read_steering_plan.part_count`、中身は `read_steering_part.rules_content` を開いた
    /// ものである。数え直しも切り直しもここでは起きない (分割は RMU がパック時に済ませて
    /// いる — `coding-rules/cqrs-boundaries.md` 規則 6)。
    #[must_use]
    pub const fn new(
        stage: StageSlugView,
        bundle: BundleDigest,
        part: PartIndex,
        parts: PartCount,
        rules_content: Vec<RuleContent>,
        continue_token: ContinueToken,
    ) -> LoadSteeringDirective {
        LoadSteeringDirective {
            stage,
            bundle,
            part,
            parts,
            rules_content,
            continue_token,
        }
    }

    /// 連鎖が属するステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlugView {
        &self.stage
    }

    /// ルール束のダイジェスト。
    #[must_use]
    pub const fn bundle(&self) -> &BundleDigest {
        &self.bundle
    }

    /// この部の索引 (1 始まり)。
    #[must_use]
    pub const fn part(&self) -> PartIndex {
        self.part
    }

    /// パート総数。
    #[must_use]
    pub const fn parts(&self) -> PartCount {
        self.parts
    }

    /// この部が運ぶルール内容 (配列順に適用する)。
    #[must_use]
    pub fn rules_content(&self) -> &[RuleContent] {
        &self.rules_content
    }

    /// 次の `continue` に渡すトークンの中身 (封緘は U7 Presenter)。
    #[must_use]
    pub const fn continue_token(&self) -> &ContinueToken {
        &self.continue_token
    }
}

#[cfg(test)]
mod tests {
    use super::super::bindings::Bindings;
    use super::super::continue_token::ContinueTokenBuilder;
    use super::super::directive::Directive;
    use super::super::directive_digest::DirectiveDigest;
    use super::super::directive_schema::DirectiveKind;
    use super::super::gate_field::GateField;
    use super::super::route_digest::RouteDigest;
    use super::super::steering_plan::SteeringPlan;
    use super::*;
    use crate::orchestration::ScopeSlugView;

    fn slug() -> StageSlugView {
        StageSlugView::parse("requirements-analysis").unwrap()
    }

    fn token(gate: GateField) -> ContinueTokenBuilder {
        ContinueTokenBuilder::new(
            slug(),
            ScopeSlugView::parse("classic").unwrap(),
            PartIndex::FIRST,
            Bindings::new(
                BundleDigest::new("sha256:bbbb"),
                DirectiveDigest::new("d"),
                RouteDigest::new("r"),
                None,
            ),
            gate,
        )
    }

    #[test]
    fn a_load_steering_part_reports_its_faces() {
        let content = RuleContent::new(
            "memory/org.md".to_string(),
            "# Org
"
            .to_string(),
        );
        assert_eq!(content.path(), "memory/org.md");
        assert_eq!(
            content.text(),
            "# Org
"
        );
        let plan = SteeringPlan::new(vec![
            vec![content],
            vec![RuleContent::new(
                "memory/team.md".to_string(),
                "# T
"
                .to_string(),
            )],
        ]);
        let first = plan.first_part().unwrap();
        let pinned = token(GateField::Ungated).build();
        let part = LoadSteeringDirective::new(
            slug(),
            BundleDigest::new("sha256:bbbb"),
            first.index(),
            first.of(),
            first.chunk().to_vec(),
            pinned.clone(),
        );
        assert_eq!(part.stage().as_str(), "requirements-analysis");
        assert_eq!(part.bundle().as_str(), "sha256:bbbb");
        assert_eq!(part.part(), PartIndex::FIRST);
        assert_eq!(part.parts().as_u32(), 2);
        assert_eq!(part.rules_content().len(), 1);
        assert_eq!(
            part.continue_token(),
            &pinned,
            "中身 (型付きトークン) を運ぶ"
        );
        assert_eq!(
            Directive::LoadSteering(part).kind(),
            DirectiveKind::LoadSteering
        );
    }
}
