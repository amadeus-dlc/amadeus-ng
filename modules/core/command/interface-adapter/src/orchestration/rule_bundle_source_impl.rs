//! `RuleBundleSource` の実 Gateway — active-space の memory 層をファイルから読み、
//! 配信計画に組む。
//!
//! 読み順は memory 層の解決順 `org → team → project → phases/<phase>` (strict-additive)。
//! ファイルが**無い**のは正常 (ルール未整備・initialization はフェーズルールを持たない)。
//! **在るのに読めない** (権限・UTF-8 破損) のは blocking で `Unreadable` を返す (02 §10)。
//!
//! 分割とパック (Markdown 見出し境界・過大セクションのコードポイント分割・
//! `STEERING_TEXT_TARGET_BYTES` へのパック — 02 §10) は**形式と輸送の知識**なので本実装が
//! 持ち、ポートへは分割済みの [`SteeringPlan`] を返す (オーナー裁定 2026-08-30)。

use std::path::{Path, PathBuf};

use core_command_domain::orchestration::{RuleContent, SteeringPlan};
use core_command_domain::workflow_definition::PhaseId;
use core_command_use_case::orchestration::{RuleBundleReadError, RuleBundleSource};

/// チャンクのテキスト目標 (`STEERING_TEXT_TARGET_BYTES = 20 * 1024`)。
const STEERING_TEXT_TARGET_BYTES: usize = 20 * 1024;

/// ルールファイル 1 つ (パス + 全文) — 本実装の内部形。
struct RuleFile {
    path: String,
    text: String,
}

/// memory ディレクトリを読む実装。
#[derive(Debug)]
pub struct RuleBundleSourceImpl {
    memory_dir: PathBuf,
}

impl RuleBundleSourceImpl {
    /// active-space の memory ディレクトリ (`aidlc/spaces/<space>/memory`) を指す。
    #[must_use]
    pub fn open(memory_dir: &Path) -> RuleBundleSourceImpl {
        RuleBundleSourceImpl {
            memory_dir: memory_dir.to_path_buf(),
        }
    }

    fn read_if_present(&self, relative: &str) -> Result<Option<RuleFile>, RuleBundleReadError> {
        let path = self.memory_dir.join(relative);
        if !path.exists() {
            return Ok(None);
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(Some(RuleFile {
                path: path.display().to_string(),
                text,
            })),
            Err(error) => Err(RuleBundleReadError::Unreadable {
                path: path.display().to_string(),
                cause: error.to_string(),
            }),
        }
    }
}

impl RuleBundleSource for RuleBundleSourceImpl {
    fn load(&self, phase: PhaseId) -> Result<SteeringPlan, RuleBundleReadError> {
        let mut files = Vec::new();
        for relative in ["org.md", "team.md", "project.md"] {
            if let Some(file) = self.read_if_present(relative)? {
                files.push(file);
            }
        }
        // initialization はフェーズルールファイルを持たない唯一のフェーズ。
        if phase != PhaseId::Initialization {
            let relative = format!("phases/{}.md", phase.as_str());
            if let Some(file) = self.read_if_present(&relative)? {
                files.push(file);
            }
        }
        plan_from_files(&files)
    }
}

/// ルール束を分割・パックして配信計画に組む。束が空なら空計画 (bare run-stage)。
fn plan_from_files(files: &[RuleFile]) -> Result<SteeringPlan, RuleBundleReadError> {
    let mut pieces = Vec::new();
    for file in files {
        for section in split_at_headings(&file.text) {
            if section.len() <= STEERING_TEXT_TARGET_BYTES {
                pieces.push(RuleContent::new(file.path.clone(), section));
                continue;
            }
            let slices =
                split_by_codepoints(&section).ok_or_else(|| RuleBundleReadError::Unsplittable {
                    path: file.path.clone(),
                })?;
            for slice in slices {
                pieces.push(RuleContent::new(file.path.clone(), slice));
            }
        }
    }
    let mut chunks: Vec<Vec<RuleContent>> = Vec::new();
    let mut current: Vec<RuleContent> = Vec::new();
    let mut current_bytes = 0usize;
    for piece in pieces {
        let piece_bytes = piece.text().len() + piece.path().len();
        if !current.is_empty() && current_bytes + piece_bytes > STEERING_TEXT_TARGET_BYTES {
            chunks.push(std::mem::take(&mut current));
            current_bytes = 0;
        }
        current_bytes += piece_bytes;
        current.push(piece);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    Ok(SteeringPlan::new(chunks))
}

/// Markdown 見出し境界 (`#` 始まりの行) で分割する。見出しの無いファイルは丸ごと 1 piece。
fn split_at_headings(text: &str) -> Vec<String> {
    let mut sections: Vec<String> = Vec::new();
    let mut current = String::new();
    for line in text.lines() {
        if line.starts_with('#') && !current.trim().is_empty() {
            sections.push(std::mem::take(&mut current));
        }
        current.push_str(line);
        current.push('\n');
    }
    if !current.trim().is_empty() {
        sections.push(current);
    }
    sections
}

/// 過大セクションをコードポイント境界でターゲット以下へ分割する。分割不能は `None`。
fn split_by_codepoints(section: &str) -> Option<Vec<String>> {
    let mut slices = Vec::new();
    let mut current = String::new();
    for c in section.chars() {
        if current.len() + c.len_utf8() > STEERING_TEXT_TARGET_BYTES {
            if current.is_empty() {
                // 1 コードポイントが上限を超える — 分割不能 (防御的)。
                return None;
            }
            slices.push(std::mem::take(&mut current));
        }
        current.push(c);
    }
    if !current.is_empty() {
        slices.push(current);
    }
    Some(slices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundle_reads_in_resolution_order_and_skips_missing_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("org.md"), "# Org\n").unwrap();
        std::fs::create_dir_all(dir.path().join("phases")).unwrap();
        std::fs::write(dir.path().join("phases/inception.md"), "# Inception\n").unwrap();
        // team.md / project.md は無い — 正常スキップ。
        let source = RuleBundleSourceImpl::open(dir.path());
        let plan = source.load(PhaseId::Inception).unwrap();
        let pieces: Vec<_> = plan.chunks().iter().flatten().collect();
        assert_eq!(pieces.len(), 2);
        let first = pieces.first().unwrap();
        let second = pieces.get(1).unwrap();
        assert!(first.path().ends_with("org.md"));
        assert!(second.path().ends_with("phases/inception.md"));
        assert_eq!(first.text(), "# Org\n");
    }

    #[test]
    fn initialization_reads_no_phase_rule() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("org.md"), "# Org\n").unwrap();
        let source = RuleBundleSourceImpl::open(dir.path());
        let plan = source.load(PhaseId::Initialization).unwrap();
        let pieces: Vec<_> = plan.chunks().iter().flatten().collect();
        assert_eq!(pieces.len(), 1);
    }

    #[test]
    fn an_empty_memory_dir_plans_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let source = RuleBundleSourceImpl::open(dir.path());
        assert!(source.load(PhaseId::Inception).unwrap().is_empty());
    }

    #[test]
    fn sections_split_at_heading_boundaries_and_pack_to_the_target() {
        // 12KiB のセクション 3 つ → 20KiB ターゲットで 3 部。
        let dir = tempfile::tempdir().unwrap();
        let big = "x".repeat(12 * 1024);
        std::fs::write(
            dir.path().join("org.md"),
            format!("# A\n{big}\n# B\n{big}\n# C\n{big}\n"),
        )
        .unwrap();
        let source = RuleBundleSourceImpl::open(dir.path());
        let plan = source.load(PhaseId::Inception).unwrap();
        assert_eq!(
            plan.part_count().as_u32(),
            3,
            "12KiB×2 は 20KiB を超えるので 1 piece 1 チャンク"
        );
    }

    #[test]
    fn an_oversize_section_splits_losslessly_at_codepoint_boundaries() {
        let dir = tempfile::tempdir().unwrap();
        let huge = "あ".repeat(9 * 1024); // 27KiB (3 bytes × 9K) — 1 セクションでターゲット超
        let body = format!("# Huge\n{huge}\n");
        std::fs::write(dir.path().join("org.md"), &body).unwrap();
        let source = RuleBundleSourceImpl::open(dir.path());
        let plan = source.load(PhaseId::Inception).unwrap();
        assert!(plan.part_count().as_u32() >= 2);
        let rebuilt: String = plan
            .chunks()
            .iter()
            .flatten()
            .map(core_command_domain::orchestration::RuleContent::text)
            .collect();
        assert_eq!(rebuilt, body, "分割は無損失");
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_file_is_blocking() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("org.md");
        std::fs::write(&path, "# Org\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
        let source = RuleBundleSourceImpl::open(dir.path());
        let error = source.load(PhaseId::Inception).unwrap_err();
        assert!(matches!(
            error,
            RuleBundleReadError::Unreadable { path, .. } if path.ends_with("org.md")
        ));
    }
}
