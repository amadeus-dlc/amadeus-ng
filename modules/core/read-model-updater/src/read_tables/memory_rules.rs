//! `MemoryRules` — 読み終えた memory 層の規則束 (参照入力の写し)。

use std::collections::BTreeMap;

use core_command_domain::workflow_definition::PhaseId;

use super::digest;
use super::rule_content::RuleContent;

/// 読み終えた memory 層の規則束 — base 3 ファイル (解決順) とフェーズ別ファイル。
///
/// これは**参照入力の写し**であってジャーナル由来の行ではない。読取 (I/O) は取得ループの
/// [`SteeringSource`] が行い、本型はその結果だけを運ぶ。ファイルが**無い**のは正常
/// (規則未整備・initialization はフェーズ規則を持たない) なので、無いファイルは単に列に
/// 現れない — 本型に失敗の表現は無い (Always Valid)。「在るのに読めない」は読み手が
/// [`CatchUpError::SteeringRead`] で止める。
///
/// [`SteeringSource`]: crate::orchestration::SteeringSource
/// [`CatchUpError::SteeringRead`]: crate::orchestration::CatchUpError::SteeringRead
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemoryRules {
    base: Vec<RuleContent>,
    phases: BTreeMap<PhaseId, RuleContent>,
}

impl MemoryRules {
    /// 読み終えた規則束を組む (**この型の唯一の構築経路**)。
    ///
    /// `base` は解決順 (`org → team → project`)、`phases` は存在したフェーズ規則だけ。
    #[must_use]
    pub const fn new(
        base: Vec<RuleContent>,
        phases: BTreeMap<PhaseId, RuleContent>,
    ) -> MemoryRules {
        MemoryRules { base, phases }
    }

    /// そのフェーズへ配る規則ファイルの列 (読み順 — base の後にフェーズ規則)。
    ///
    /// strict-additive なので後置である。フェーズ規則を持たないフェーズ (initialization、
    /// および規則ファイルが置かれていないフェーズ) は base だけになる。
    #[must_use]
    pub fn files_for(&self, phase: PhaseId) -> Vec<RuleContent> {
        let mut files = self.base.clone();
        if let Some(rule) = self.phases.get(&phase) {
            files.push(rule.clone());
        }
        files
    }

    /// 参照入力そのもののダイジェスト — 全ファイル (path + text) を**読み順に 1 度ずつ**。
    ///
    /// 取得ループはこの値を保存済みの値と比べ、違うときだけ再パックする。フェーズ束は
    /// base を共有するので、束ごとに数えると同じファイルを 5 回数えることになる —
    /// 数えるのは**読んだファイル**である。
    #[must_use]
    pub fn source_digest(&self) -> String {
        digest::source(self.base.iter().chain(self.phases.values()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read_tables::RuleContent;
    use core_command_domain::workflow_definition::PhaseId;

    fn content(path: &str, text: &str) -> RuleContent {
        RuleContent::new(path.to_string(), text.to_string())
    }

    #[test]
    fn the_files_for_a_phase_are_the_base_in_resolution_order_then_the_phase_rule() {
        let rules = MemoryRules::new(
            vec![
                content("org.md", "# Org\n"),
                content("team.md", "# Team\n"),
                content("project.md", "# Project\n"),
            ],
            [(PhaseId::Inception, content("phases/inception.md", "# I\n"))]
                .into_iter()
                .collect(),
        );
        let files = rules.files_for(PhaseId::Inception);
        let paths: Vec<&str> = files.iter().map(RuleContent::path).collect();
        assert_eq!(
            paths,
            ["org.md", "team.md", "project.md", "phases/inception.md"]
        );
    }

    #[test]
    fn a_phase_without_a_rule_file_reads_the_base_only() {
        let rules = MemoryRules::new(vec![content("org.md", "# Org\n")], BTreeMap::new());
        assert_eq!(rules.files_for(PhaseId::Initialization).len(), 1);
    }

    #[test]
    fn the_source_digest_covers_every_file_once_in_reading_order() {
        let base = vec![content("org.md", "# Org\n")];
        let phases: BTreeMap<PhaseId, RuleContent> =
            [(PhaseId::Inception, content("phases/inception.md", "# I\n"))]
                .into_iter()
                .collect();
        let rules = MemoryRules::new(base.clone(), phases.clone());
        assert_eq!(rules.source_digest(), rules.source_digest(), "決定的である");

        // 本文が 1 バイト変われば別のダイジェストになる (再投影の引き金)。
        let edited = MemoryRules::new(vec![content("org.md", "# Org!\n")], phases.clone());
        assert_ne!(rules.source_digest(), edited.source_digest());

        // ファイルが増えても別のダイジェストになる。
        let mut more = base;
        more.push(content("team.md", "# Team\n"));
        assert_ne!(
            rules.source_digest(),
            MemoryRules::new(more, phases).source_digest()
        );
    }

    #[test]
    fn an_empty_memory_layer_still_has_a_digest() {
        let empty = MemoryRules::default();
        assert_eq!(empty.source_digest().len(), 64, "生 hex 64 桁");
    }
}
