//! `ReadModelUpdater` の投影書込先 2 面。

use std::path::{Path, PathBuf};

/// 投影の書込先 2 面の場所。
///
/// 生の `PathBuf` を 2 本ばらばらに引き回さないための束である — 片方だけ差し替わった
/// 取り合わせ（別 intent の状態ファイルと別 clone のシャード）を構成できなくする。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionTargets {
    state_file: PathBuf,
    audit_shard: PathBuf,
}

impl ProjectionTargets {
    /// 状態ファイルと監査シャードの場所から組む。
    #[must_use]
    pub fn new(
        state_file: impl Into<PathBuf>,
        audit_shard: impl Into<PathBuf>,
    ) -> ProjectionTargets {
        ProjectionTargets {
            state_file: state_file.into(),
            audit_shard: audit_shard.into(),
        }
    }

    /// 状態ファイル（`aidlc-state.md`）の場所。
    #[must_use]
    pub fn state_file(&self) -> &Path {
        &self.state_file
    }

    /// 監査シャード（`<record>/audit/<host>-<clone>.md`）の場所。
    #[must_use]
    pub fn audit_shard(&self) -> &Path {
        &self.audit_shard
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_targets_keep_both_paths_together() {
        let targets = ProjectionTargets::new("/w/aidlc-state.md", "/w/audit/host-abcd1234.md");
        assert_eq!(targets.state_file(), Path::new("/w/aidlc-state.md"));
        assert_eq!(
            targets.audit_shard(),
            Path::new("/w/audit/host-abcd1234.md")
        );
        assert_eq!(
            targets,
            ProjectionTargets::new("/w/aidlc-state.md", "/w/audit/host-abcd1234.md")
        );
    }
}
