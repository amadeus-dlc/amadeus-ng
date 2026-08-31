//! `MemoryRulesDao` の実 Gateway — active-space の memory 層をファイルから読む。
//!
//! 読み順は memory 層の解決順 `org → team → project → phases/<phase>` (strict-additive)。
//! ファイルが**無い**のは正常 (ルール未整備) なので束の列に現れないだけで、失敗にはしない。
//! **在るのに読めない** (権限・UTF-8 破損) のは blocking で `Err` を返す (02 §10)。
//!
//! 分割とパック (見出し境界・過大セクションのコードポイント分割・輸送目標へのパック) は
//! ここでは**行わない** — 形式と輸送の知識は配信計画の側にあり、`MemoryRules::plan_for` →
//! `SteeringPlan::pack` が持つ。本実装が返すのは読み終えた全文である。
//!
//! **クエリ側のアダプタが fs を読むのは正当**である — リードモデルを読むのがこの層の仕事で
//! ある (`coding-rules/cqrs-boundaries.md` 規則 6)。

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use core_query_use_case::orchestration::{
    MemoryRules, MemoryRulesDao, MemoryRulesReadError, RuleContent,
};
use core_query_use_case::workflow_view::PhaseView;

/// base ルールの解決順 (strict-additive — 後のものが前のものを特殊化する)。
const BASE_FILES: [&str; 3] = ["org.md", "team.md", "project.md"];

/// memory 層ディレクトリを読む実装。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRulesDaoImpl {
    memory_dir: PathBuf,
}

impl MemoryRulesDaoImpl {
    /// active-space の memory ディレクトリ (`aidlc/spaces/<space>/memory`) を指す。
    ///
    /// 束が運ぶルールのパスは `memory_dir` を前置した形になる — directive の
    /// `rules_in_context` 台帳と `load-steering` の配信パスがこの綴りをそのまま載せるので、
    /// **どの綴りで渡すかは呼び手 (合成ルート) が決める**。ワークスペース相対で渡せば
    /// 台帳もワークスペース相対になる。
    #[must_use]
    pub const fn new(memory_dir: PathBuf) -> MemoryRulesDaoImpl {
        MemoryRulesDaoImpl { memory_dir }
    }

    /// 在れば読み、無ければ `None`。読めないのは失敗である。
    fn read_if_present(&self, relative: &str) -> Result<Option<RuleContent>, MemoryRulesReadError> {
        let path = self.memory_dir.join(relative);
        match fs::read_to_string(&path) {
            Ok(text) => Ok(Some(RuleContent::new(display(&path), text))),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(MemoryRulesReadError::new(display(&path), e.to_string())),
        }
    }
}

impl MemoryRulesDao for MemoryRulesDaoImpl {
    fn find(&self) -> Result<MemoryRules, MemoryRulesReadError> {
        let mut base = Vec::new();
        for relative in BASE_FILES {
            if let Some(rule) = self.read_if_present(relative)? {
                base.push(rule);
            }
        }
        let mut phases = BTreeMap::new();
        for phase in PhaseView::ALL {
            // initialization はブートストラップ専用でフェーズルールファイルを持たない —
            // 置かれていても配信対象にしない (02 §10 / 旧 RuleBundleSource の読み順)。
            if phase == PhaseView::Initialization {
                continue;
            }
            if let Some(rule) = self.read_if_present(&format!("phases/{}.md", phase.as_str()))? {
                phases.insert(phase, rule);
            }
        }
        Ok(MemoryRules::new(base, phases))
    }
}

/// パスの綴り (`Path::display` の写し — 台帳に載る文字列はここで決まる)。
fn display(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    // panic! は想定外バリアントの即時失敗という検証用途で使っており、テスト失敗のシグナル
    // として妥当なため許容する。
    #![allow(clippy::panic)]

    use super::*;
    use tempfile::{TempDir, tempdir};

    /// `memory_dir` 直下・`phases/` 配下にファイルを置いた一時ディレクトリ。
    fn memory_layer(files: &[(&str, &str)]) -> TempDir {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("phases")).unwrap();
        for (relative, text) in files {
            fs::write(dir.path().join(relative), text).unwrap();
        }
        dir
    }

    /// 束が運ぶパスの末尾 (前置される一時ディレクトリを落として比較する)。
    fn suffixes(rules: &MemoryRules, phase: PhaseView) -> Vec<String> {
        rules
            .plan_for(phase)
            .unwrap()
            .chunks()
            .iter()
            .flatten()
            .map(|piece| {
                let path = piece.path().replace('\\', "/");
                let (_, tail) = path.split_once("/memory-fixture/").unwrap_or(("", &path));
                tail.to_string()
            })
            .collect()
    }

    #[test]
    fn the_bundle_reads_in_resolution_order_and_skips_missing_files() {
        // team.md は無い — 正常スキップ。
        let dir = memory_layer(&[
            ("org.md", "# Org\n"),
            ("project.md", "# Project\n"),
            ("phases/inception.md", "# Inception\n"),
        ]);
        let rules = MemoryRulesDaoImpl::new(dir.path().to_path_buf())
            .find()
            .unwrap();
        let paths: Vec<String> = rules
            .plan_for(PhaseView::Inception)
            .unwrap()
            .delivered_paths();
        assert_eq!(paths.len(), 3, "org → project → phases/inception");
        assert!(paths.first().unwrap().ends_with("org.md"), "{paths:?}");
        assert!(paths.get(1).unwrap().ends_with("project.md"), "{paths:?}");
        assert!(
            paths.get(2).unwrap().ends_with("phases/inception.md"),
            "フェーズルールは base の後 (strict-additive) — {paths:?}"
        );
    }

    #[test]
    fn a_phase_rule_of_another_phase_is_not_delivered() {
        let dir = memory_layer(&[
            ("org.md", "# Org\n"),
            ("phases/inception.md", "# Inception\n"),
            ("phases/construction.md", "# Construction\n"),
        ]);
        let rules = MemoryRulesDaoImpl::new(dir.path().to_path_buf())
            .find()
            .unwrap();
        let paths = rules
            .plan_for(PhaseView::Construction)
            .unwrap()
            .delivered_paths();
        assert_eq!(paths.len(), 2, "org + construction だけ — {paths:?}");
        assert!(
            paths.get(1).unwrap().ends_with("phases/construction.md"),
            "{paths:?}"
        );
    }

    #[test]
    fn initialization_reads_no_phase_rule_even_when_the_file_exists() {
        // ブートストラップ専用フェーズはルールファイルを持たない — 置いても配信しない。
        let dir = memory_layer(&[
            ("org.md", "# Org\n"),
            ("phases/initialization.md", "# Initialization\n"),
        ]);
        let rules = MemoryRulesDaoImpl::new(dir.path().to_path_buf())
            .find()
            .unwrap();
        let paths = rules
            .plan_for(PhaseView::Initialization)
            .unwrap()
            .delivered_paths();
        assert_eq!(paths.len(), 1, "org.md だけ — {paths:?}");
        assert!(paths.first().unwrap().ends_with("org.md"), "{paths:?}");
    }

    #[test]
    fn all_four_rule_bearing_phases_are_read() {
        let dir = memory_layer(&[
            ("phases/ideation.md", "# Ideation\n"),
            ("phases/inception.md", "# Inception\n"),
            ("phases/construction.md", "# Construction\n"),
            ("phases/operation.md", "# Operation\n"),
        ]);
        let rules = MemoryRulesDaoImpl::new(dir.path().to_path_buf())
            .find()
            .unwrap();
        for phase in [
            PhaseView::Ideation,
            PhaseView::Inception,
            PhaseView::Construction,
            PhaseView::Operation,
        ] {
            let paths = rules.plan_for(phase).unwrap().delivered_paths();
            assert_eq!(paths.len(), 1, "{phase:?}");
            assert!(
                paths
                    .first()
                    .unwrap()
                    .ends_with(&format!("phases/{}.md", phase.as_str())),
                "{paths:?}"
            );
        }
    }

    #[test]
    fn an_empty_memory_layer_is_a_normal_observation() {
        let dir = tempdir().unwrap();
        let rules = MemoryRulesDaoImpl::new(dir.path().to_path_buf())
            .find()
            .unwrap();
        assert!(
            rules.plan_for(PhaseView::Inception).unwrap().is_empty(),
            "ルール未整備は空計画 = bare run-stage"
        );
    }

    #[test]
    fn a_memory_dir_that_does_not_exist_is_a_normal_observation() {
        let dir = tempdir().unwrap();
        let rules = MemoryRulesDaoImpl::new(dir.path().join("no-such-space/memory"))
            .find()
            .unwrap();
        assert!(rules.plan_for(PhaseView::Inception).unwrap().is_empty());
    }

    #[test]
    fn a_base_file_that_is_present_but_unreadable_is_blocking() {
        let dir = tempdir().unwrap();
        // ルールファイルの位置にディレクトリを置く — read_to_string は EISDIR で失敗する。
        fs::create_dir(dir.path().join("team.md")).unwrap();
        let error = MemoryRulesDaoImpl::new(dir.path().to_path_buf())
            .find()
            .unwrap_err();
        assert!(error.path().ends_with("team.md"), "{error:?}");
        assert!(!error.cause().is_empty(), "OS 由来の理由を運ぶ");
    }

    #[test]
    fn a_phase_file_that_is_present_but_unreadable_is_blocking() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("phases/operation.md")).unwrap();
        let error = MemoryRulesDaoImpl::new(dir.path().to_path_buf())
            .find()
            .unwrap_err();
        assert!(error.path().ends_with("phases/operation.md"), "{error:?}");
    }

    #[test]
    fn the_delivered_path_is_spelled_from_the_memory_dir_the_caller_supplied() {
        // 呼び手が渡した綴りがそのまま台帳に載る (相対で渡せば相対で出る)。
        let dir = tempdir().unwrap();
        let memory = dir.path().join("memory-fixture");
        fs::create_dir_all(memory.join("phases")).unwrap();
        fs::write(memory.join("org.md"), "# Org\n").unwrap();
        let rules = MemoryRulesDaoImpl::new(memory).find().unwrap();
        assert_eq!(suffixes(&rules, PhaseView::Inception), ["org.md"]);
    }
}
