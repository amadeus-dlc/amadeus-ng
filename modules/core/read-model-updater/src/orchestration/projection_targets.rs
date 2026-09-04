//! `ReadModelUpdater` の投影書込先 4 面。

use std::path::{Path, PathBuf};

/// 投影の書込先 4 面の場所。
///
/// 生の `PathBuf` をばらばらに引き回さないための束である — 片方だけ差し替わった
/// 取り合わせ（別 intent の状態ファイルと別 clone のシャード）を構成できなくする。
///
/// # メモリ層の 2 ファイル（b49）
///
/// `team.md` / `project.md` は**人が編集する正本**でもあるが、`PracticesAffirmed` の投影が
/// 節を置き換え・規則行を足す面でもある（設計 §5）。パスは active-space の memory
/// ディレクトリから導くので、束を組む側は**ディレクトリ 1 本**を渡す — 2 本を別々に渡せる
/// ようにすると、別の space の team.md と project.md を取り合わせられてしまう。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionTargets {
    state_file: PathBuf,
    audit_shard: PathBuf,
    team_md: PathBuf,
    project_md: PathBuf,
}

impl ProjectionTargets {
    /// 状態ファイル・監査シャード・memory ディレクトリから組む（**唯一の構築経路**）。
    #[must_use]
    pub fn new(
        state_file: impl Into<PathBuf>,
        audit_shard: impl Into<PathBuf>,
        memory_dir: impl Into<PathBuf>,
    ) -> ProjectionTargets {
        let memory_dir = memory_dir.into();
        ProjectionTargets {
            state_file: state_file.into(),
            audit_shard: audit_shard.into(),
            team_md: memory_dir.join("team.md"),
            project_md: memory_dir.join("project.md"),
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

    /// メモリ層のチーム規則（`<memory>/team.md`）の場所。
    #[must_use]
    pub fn team_md(&self) -> &Path {
        &self.team_md
    }

    /// メモリ層のプロジェクト規則（`<memory>/project.md`）の場所。
    #[must_use]
    pub fn project_md(&self) -> &Path {
        &self.project_md
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_targets_keep_every_path_together() {
        let targets = ProjectionTargets::new(
            "/w/aidlc-state.md",
            "/w/audit/host-abcd1234.md",
            "/w/memory",
        );
        assert_eq!(targets.state_file(), Path::new("/w/aidlc-state.md"));
        assert_eq!(
            targets.audit_shard(),
            Path::new("/w/audit/host-abcd1234.md")
        );
        assert_eq!(targets.team_md(), Path::new("/w/memory/team.md"));
        assert_eq!(targets.project_md(), Path::new("/w/memory/project.md"));
        assert_eq!(
            targets,
            ProjectionTargets::new(
                "/w/aidlc-state.md",
                "/w/audit/host-abcd1234.md",
                "/w/memory",
            )
        );
    }
}
