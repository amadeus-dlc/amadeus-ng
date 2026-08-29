//! `RuleBundleSource` の実 Gateway — active-space の memory 層をファイルから読む。
//!
//! 読み順は memory 層の解決順 `org → team → project → phases/<phase>` (strict-additive)。
//! ファイルが**無い**のは正常 (ルール未整備・initialization はフェーズルールを持たない)。
//! **在るのに読めない** (権限・UTF-8 破損) のは blocking で `Unreadable` を返す (02 §10)。

use std::path::{Path, PathBuf};

use core_command_domain::workflow_definition::PhaseId;
use core_command_use_case::orchestration::{RuleBundleReadError, RuleBundleSource, RuleFile};

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
            Ok(text) => Ok(Some(RuleFile::new(path.display().to_string(), text))),
            Err(error) => Err(RuleBundleReadError::Unreadable {
                path: path.display().to_string(),
                cause: error.to_string(),
            }),
        }
    }
}

impl RuleBundleSource for RuleBundleSourceImpl {
    fn load(&self, phase: PhaseId) -> Result<Vec<RuleFile>, RuleBundleReadError> {
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
        Ok(files)
    }
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
        let files = source.load(PhaseId::Inception).unwrap();
        assert_eq!(files.len(), 2);
        let first = files.first().unwrap();
        let second = files.get(1).unwrap();
        assert!(first.path().ends_with("org.md"));
        assert!(second.path().ends_with("phases/inception.md"));
        assert_eq!(first.text(), "# Org\n");
    }

    #[test]
    fn initialization_reads_no_phase_rule() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("org.md"), "# Org\n").unwrap();
        let source = RuleBundleSourceImpl::open(dir.path());
        let files = source.load(PhaseId::Initialization).unwrap();
        assert_eq!(files.len(), 1);
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
