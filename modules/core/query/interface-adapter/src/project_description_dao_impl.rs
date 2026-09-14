//! `ProjectDescriptionDao` の実 Gateway — record 直下の依頼原文サイドカーを生テキストで読む。
//!
//! 媒体はファイルであり、SQLite の `read_*` 表とは別の面である — したがって
//! [`super::ReadModelDaos`] (1 要求 1 接続) の住人ではなく、サイドカーの所在だけを握る
//! ([`super::StateFileDaoImpl`] と同型)。

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use core_query_use_case::orchestration::{ProjectDescriptionDao, ReadModelReadError};

/// 依頼原文サイドカーを 1 面読む実装。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDescriptionDaoImpl {
    sidecar: PathBuf,
}

impl ProjectDescriptionDaoImpl {
    /// サイドカーの所在を受け取る (**この型の唯一の構築経路**)。
    #[must_use]
    pub fn new(sidecar: &Path) -> ProjectDescriptionDaoImpl {
        ProjectDescriptionDaoImpl {
            sidecar: sidecar.to_path_buf(),
        }
    }

    fn failure(&self, kind: ErrorKind) -> ReadModelReadError {
        ReadModelReadError::new(kind, Some(self.sidecar.clone()))
    }
}

impl ProjectDescriptionDao for ProjectDescriptionDaoImpl {
    fn find(&self) -> Result<Option<String>, ReadModelReadError> {
        // 追従しない検査を先に置く — upstream は `readRegularFileNoFollowOrThrow` で
        // シンボリックリンク・FIFO・ディレクトリを読む前に拒む。依頼原文は record の外の
        // バイトを引き込ませてよい面ではないので、その安全性をここでも保つ。
        let metadata = match std::fs::symlink_metadata(&self.sidecar) {
            Ok(metadata) => metadata,
            // 不在は失敗ではない — legacy record はサイドカーを持たない。
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(self.failure(error.kind())),
        };
        if !metadata.is_file() {
            return Err(self.failure(ErrorKind::InvalidInput));
        }
        std::fs::read_to_string(&self.sidecar)
            .map(Some)
            .map_err(|error| self.failure(error.kind()))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn an_absent_sidecar_is_not_a_failure() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let dao = ProjectDescriptionDaoImpl::new(&root.path().join("project-description.json"));
        assert_eq!(dao.find(), Ok(None));
    }

    #[test]
    fn the_bytes_are_returned_verbatim() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let path = root.path().join("project-description.json");
        std::fs::write(&path, "\"依頼原文\\nの 2 行目\"").expect("サイドカー");
        assert_eq!(
            ProjectDescriptionDaoImpl::new(&path).find(),
            Ok(Some("\"依頼原文\\nの 2 行目\"".to_string()))
        );
    }

    #[test]
    fn a_directory_in_the_sidecar_position_is_a_failure() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let path = root.path().join("project-description.json");
        std::fs::create_dir(&path).expect("ディレクトリ");
        let error = ProjectDescriptionDaoImpl::new(&path)
            .find()
            .expect_err("ディレクトリは読めない");
        assert_eq!(error.path(), Some(path.as_path()));
    }

    /// シンボリックリンクは**追従せず**拒む — record の外のバイトを依頼原文として引き込ませない。
    #[cfg(unix)]
    #[test]
    fn a_symlinked_sidecar_is_refused_rather_than_followed() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let target = root.path().join("elsewhere.json");
        std::fs::write(&target, "\"外のバイト\"").expect("リンク先");
        let path = root.path().join("project-description.json");
        std::os::unix::fs::symlink(&target, &path).expect("シンボリックリンク");

        let error = ProjectDescriptionDaoImpl::new(&path)
            .find()
            .expect_err("追従しない");

        assert_eq!(error.kind(), ErrorKind::InvalidInput);
    }
}
