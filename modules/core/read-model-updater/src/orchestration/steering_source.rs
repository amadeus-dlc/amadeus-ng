//! `SteeringSource` — 参照入力 (active-space の memory 層) の読取先。

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use core_command_domain::workflow_definition::PhaseId;

use crate::read_tables::{MemoryRules, RuleContent};

use super::catch_up_error::CatchUpError;

/// base 規則の解決順 (strict-additive — 後のものが前のものを特殊化する)。
const BASE_FILES: [&str; 3] = ["org.md", "team.md", "project.md"];

/// 取得ループが steering の材料を読む先 — active-space の memory ディレクトリ。
///
/// # `ProjectionTargets` とは別の型である
///
/// [`ProjectionTargets`] は**書込先**、本型は**読取入力**である。同じ「場所を持つ型」でも、
/// 一方は投影が描くファイルを指し、他方は人が編集するファイルを指す。1 つに束ねると、
/// 書込先を差し替えたつもりで読取先まで動く。
///
/// # 読み順
///
/// `org.md` → `team.md` → `project.md` → `phases/<phase>.md` (memory 層の解決順)。
/// ファイルが**無い**のは正常 (規則未整備) なので束の列に現れないだけで、失敗にはしない。
/// **在るのに読めない** (権限・UTF-8 破損) のは blocking である — 規則を静かに落として
/// 進むと、届く steering が痩せたまま気づかれない。
///
/// initialization はブートストラップ専用でフェーズ規則ファイルを持たないので、
/// `phases/initialization.md` は置かれていても読まない (02 §10)。base は全フェーズへ届く
/// ので、initialization の束が空になるわけではない。
///
/// [`ProjectionTargets`]: super::ProjectionTargets
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteeringSource {
    memory_dir: PathBuf,
}

impl SteeringSource {
    /// active-space の memory ディレクトリ (`aidlc/spaces/<space>/memory`) を指す。
    ///
    /// 束が運ぶ規則のパスは `memory_dir` を前置した形になる — 配信済みパスの台帳が
    /// この綴りをそのまま載せるので、**どの綴りで渡すかは呼び手 (合成ルート) が決める**。
    #[must_use]
    pub const fn new(memory_dir: PathBuf) -> SteeringSource {
        SteeringSource { memory_dir }
    }

    /// 読む先のディレクトリ。
    #[must_use]
    pub fn memory_dir(&self) -> &Path {
        &self.memory_dir
    }

    /// memory 層を読み順どおりに読む。
    ///
    /// # Errors
    ///
    /// 在るのに読めない規則ファイル ([`CatchUpError::SteeringRead`])。取得ループの失敗を
    /// そのまま返すのは、この読取が取得ループの一部だからである — 純粋投影核
    /// ([`SteeringTables::pack`]) はこの型を知らない (二層構造)。
    ///
    /// [`SteeringTables::pack`]: crate::read_tables::SteeringTables::pack
    pub fn read(&self) -> Result<MemoryRules, CatchUpError> {
        let mut base = Vec::new();
        for relative in BASE_FILES {
            if let Some(rule) = self.read_if_present(relative)? {
                base.push(rule);
            }
        }
        let mut phases = BTreeMap::new();
        for phase in all_phases() {
            if phase == PhaseId::Initialization {
                continue;
            }
            if let Some(rule) = self.read_if_present(&format!("phases/{}.md", phase.as_str()))? {
                phases.insert(phase, rule);
            }
        }
        Ok(MemoryRules::new(base, phases))
    }

    /// 在れば読み、無ければ `None`。読めないのは失敗である。
    fn read_if_present(&self, relative: &str) -> Result<Option<RuleContent>, CatchUpError> {
        let path = self.memory_dir.join(relative);
        match fs::read_to_string(&path) {
            Ok(text) => Ok(Some(RuleContent::new(display(&path), text))),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(CatchUpError::SteeringRead {
                path: display(&path),
                kind: error.kind(),
            }),
        }
    }
}

/// フェーズの全列挙 (番号順)。
fn all_phases() -> impl Iterator<Item = PhaseId> {
    (0..=4_u32).filter_map(PhaseId::from_index)
}

/// パスの綴り (`Path::display` の写し — 台帳に載る文字列はここで決まる)。
fn display(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    // 想定外ケースの即時失敗はテストの検証手段である (house style)。
    #![allow(clippy::panic)]

    use super::*;
    use tempfile::{TempDir, tempdir};

    /// `memory_dir` 直下・`phases/` 配下にファイルを置いた一時ディレクトリ。
    fn memory_layer(files: &[(&str, &str)]) -> TempDir {
        let dir = tempdir().expect("一時ディレクトリ");
        fs::create_dir_all(dir.path().join("phases")).expect("phases を作る");
        for (relative, text) in files {
            fs::write(dir.path().join(relative), text).expect("規則を置く");
        }
        dir
    }

    /// 束が運ぶパス (前置される一時ディレクトリの綴りは末尾一致で見る)。
    fn paths(rules: &MemoryRules, phase: PhaseId) -> Vec<String> {
        rules
            .files_for(phase)
            .iter()
            .map(|piece| piece.path().replace('\\', "/"))
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
        let rules = SteeringSource::new(dir.path().to_path_buf())
            .read()
            .expect("読める");
        let paths = paths(&rules, PhaseId::Inception);
        assert_eq!(paths.len(), 3);
        assert!(
            paths.first().is_some_and(|p| p.ends_with("org.md")),
            "{paths:?}"
        );
        assert!(
            paths.get(1).is_some_and(|p| p.ends_with("project.md")),
            "{paths:?}"
        );
        assert!(
            paths
                .get(2)
                .is_some_and(|p| p.ends_with("phases/inception.md")),
            "フェーズ規則は base の後 (strict-additive) — {paths:?}"
        );
    }

    #[test]
    fn the_initialization_phase_rule_is_never_read_even_when_it_exists() {
        let dir = memory_layer(&[
            ("org.md", "# Org\n"),
            ("phases/initialization.md", "# Never delivered\n"),
        ]);
        let rules = SteeringSource::new(dir.path().to_path_buf())
            .read()
            .expect("読める");
        assert_eq!(
            rules.files_for(PhaseId::Initialization).len(),
            1,
            "initialization には base だけが届く"
        );
        // どのフェーズの束にも initialization の規則は載らない。
        for phase in all_phases() {
            assert!(
                !rules
                    .files_for(phase)
                    .iter()
                    .any(|piece| piece.path().contains("initialization")),
                "{phase:?}"
            );
        }
    }

    #[test]
    fn an_absent_memory_directory_reads_as_an_empty_bundle() {
        let dir = tempdir().expect("一時ディレクトリ");
        let source = SteeringSource::new(dir.path().join("absent"));
        assert_eq!(source.read().expect("欠損は正常"), MemoryRules::default());
        assert!(source.memory_dir().ends_with("absent"));
    }

    #[test]
    fn a_file_that_exists_but_is_not_utf8_is_a_blocking_failure() {
        let dir = memory_layer(&[("org.md", "# Org\n")]);
        fs::write(dir.path().join("team.md"), [0x80_u8, 0x81]).expect("不正なバイト");
        let error = SteeringSource::new(dir.path().to_path_buf())
            .read()
            .expect_err("読めない規則は止める");
        match error {
            CatchUpError::SteeringRead { path, kind } => {
                assert!(path.ends_with("team.md"), "実際: {path}");
                assert_eq!(kind, io::ErrorKind::InvalidData);
            }
            other => panic!("読取の失敗として上がる (実際: {other:?})"),
        }
    }
}
