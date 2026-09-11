//! `Learning` — 人が残すと決めた 1 件の学び。

use super::learning_candidate_id::LearningCandidateId;
use super::learning_content_hash::LearningContentHash;
use super::learning_provenance::LearningProvenance;
use super::learning_scope::LearningScope;
use super::learning_source::LearningSource;
use super::practice_heading::PracticeHeading;
use crate::workflow_definition::StageSlug;

/// 確定した学び 1 件。**学びは実践である**ので、落ちる先はメモリ層の実践行である
/// （`stage-protocol.md` §13「A confirmed learning IS a practice」）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Learning {
    candidate_id: LearningCandidateId,
    scope: LearningScope,
    heading: PracticeHeading,
    text: String,
    source: LearningSource,
    content_hash: LearningContentHash,
}

impl Learning {
    /// 学びの材料を束ねる完全コンストラクタ。同一性 (`content_hash`) は本文から導く。
    #[must_use]
    pub fn new(
        candidate_id: LearningCandidateId,
        scope: LearningScope,
        heading: PracticeHeading,
        text: impl Into<String>,
        source: LearningSource,
    ) -> Learning {
        let text = text.into();
        let content_hash = LearningContentHash::of_text(&text);
        Learning {
            candidate_id,
            scope,
            heading,
            text,
            source,
            content_hash,
        }
    }

    /// surface のその回の候補番号。
    #[must_use]
    pub const fn candidate_id(&self) -> &LearningCandidateId {
        &self.candidate_id
    }

    /// 書込先の層。
    #[must_use]
    pub const fn scope(&self) -> LearningScope {
        self.scope
    }

    /// 書込先の見出し。
    #[must_use]
    pub const fn heading(&self) -> &PracticeHeading {
        &self.heading
    }

    /// 学びの本文（人が読む逐語）。
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// 学びの出所。
    #[must_use]
    pub const fn source(&self) -> LearningSource {
        self.source
    }

    /// 本文から導いた同一性。
    #[must_use]
    pub const fn content_hash(&self) -> &LearningContentHash {
        &self.content_hash
    }

    /// 実践行に付く重複抑止の印（`<!-- cid:<intent>:<stage>:<hash> -->`）。
    #[must_use]
    pub fn marker(&self, provenance: &LearningProvenance, stage: &StageSlug) -> String {
        format!(
            "<!-- cid:{}:{}:{} -->",
            provenance.intent().as_str(),
            stage.as_str(),
            self.content_hash.as_str()
        )
    }

    /// メモリ層へ足す実践行（末尾の改行込み）。
    #[must_use]
    pub fn practice_line(
        &self,
        provenance: &LearningProvenance,
        stage: &StageSlug,
        learned_on: chrono::NaiveDate,
    ) -> String {
        format!(
            "- {} (learned {}) {}\n",
            self.text,
            learned_on.format("%Y-%m-%d"),
            self.marker(provenance, stage)
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::Learning;
    use crate::orchestration::{
        LearningCandidateId, LearningProvenance, LearningScope, LearningSource, PracticeHeading,
    };
    use crate::workflow_definition::StageSlug;
    use crate::workspace::{IntentDirName, SpaceName};

    /// ゴールデン `tests/golden/upstream-a277af21/learnings/cases.json` の実測。
    const GOLDEN_TEXT: &str = "ALWAYS 採取用の検証結果を記録する。";
    const GOLDEN_HASH: &str = "f543ed24a72a9b57b8fac723a95fa0a2c04240a25c8a6a67229322acb3de9bd3";

    fn golden_learning() -> Learning {
        Learning::new(
            LearningCandidateId::parse("fixture-1").expect("candidate"),
            LearningScope::Project,
            PracticeHeading::from_routed("Corrections"),
            GOLDEN_TEXT,
            LearningSource::UserAddition,
        )
    }

    fn provenance() -> LearningProvenance {
        LearningProvenance::new(
            SpaceName::parse("default").expect("space"),
            IntentDirName::parse("260908-learnings").expect("intent"),
        )
    }

    fn stage() -> StageSlug {
        StageSlug::parse("requirements-analysis").expect("stage")
    }

    #[test]
    fn the_learning_derives_its_identity_from_the_text() {
        assert_eq!(golden_learning().content_hash().as_str(), GOLDEN_HASH);
        assert_eq!(golden_learning().text(), GOLDEN_TEXT);
        assert_eq!(golden_learning().scope(), LearningScope::Project);
        assert_eq!(golden_learning().source(), LearningSource::UserAddition);
        assert_eq!(golden_learning().heading().as_str(), "## Corrections");
        assert_eq!(golden_learning().candidate_id().as_str(), "fixture-1");
    }

    #[test]
    fn the_practice_line_is_the_upstream_verbatim() {
        let line = golden_learning().practice_line(
            &provenance(),
            &stage(),
            chrono::NaiveDate::from_ymd_opt(2026, 9, 8).expect("date"),
        );
        assert_eq!(
            line,
            format!(
                "- {GOLDEN_TEXT} (learned 2026-09-08) <!-- cid:260908-learnings:requirements-analysis:{GOLDEN_HASH} -->\n"
            )
        );
    }

    #[test]
    fn the_marker_binds_the_intent_the_stage_and_the_content() {
        assert_eq!(
            golden_learning().marker(&provenance(), &stage()),
            format!("<!-- cid:260908-learnings:requirements-analysis:{GOLDEN_HASH} -->")
        );
    }
}
