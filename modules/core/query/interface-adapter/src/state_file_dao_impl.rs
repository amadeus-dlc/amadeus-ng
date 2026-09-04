//! `StateFileDao` の実 Gateway — record の `aidlc-state.md` を生テキストで読む。

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use core_query_use_case::orchestration::{ReadModelReadError, StateFileDao};

/// upstream 互換の人間可読リードモデルを 1 面読む実装。
///
/// 媒体はファイルであり、SQLite の `read_*` 表とは別の面である — したがって
/// [`super::ReadModelDaos`] (1 要求 1 接続) の住人ではなく、状態ファイルの所在だけを握る。
/// どの record を見るかは合成ルートが結線する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateFileDaoImpl {
    state_file: PathBuf,
}

impl StateFileDaoImpl {
    /// 状態ファイルの所在を受け取る (**この型の唯一の構築経路**)。
    #[must_use]
    pub fn new(state_file: &Path) -> StateFileDaoImpl {
        StateFileDaoImpl {
            state_file: state_file.to_path_buf(),
        }
    }
}

impl StateFileDao for StateFileDaoImpl {
    fn find(&self) -> Result<Option<String>, ReadModelReadError> {
        match std::fs::read_to_string(&self.state_file) {
            Ok(content) => Ok(Some(content)),
            // 不在は失敗ではない — record がまだ無い / 状態ファイルがまだ書かれていない。
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(ReadModelReadError::new(
                error.kind(),
                Some(self.state_file.clone()),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn an_absent_state_file_is_not_a_failure() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let dao = StateFileDaoImpl::new(&root.path().join("aidlc-state.md"));
        assert_eq!(dao.find(), Ok(None));
    }

    #[test]
    fn a_zero_byte_state_file_is_present_and_empty() {
        // upstream の `loadStateFileIfPresent` は `!== null` で判定する — 0 バイトは
        // 「不在」ではなく「版が読めない状態ファイル」である (ピン `:5479-5481`)。
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let path = root.path().join("aidlc-state.md");
        std::fs::write(&path, "").expect("空ファイル");
        assert_eq!(StateFileDaoImpl::new(&path).find(), Ok(Some(String::new())));
    }

    #[test]
    fn the_text_is_returned_verbatim() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let path = root.path().join("aidlc-state.md");
        std::fs::write(&path, "- **State Version**: 8\n").expect("状態ファイル");
        assert_eq!(
            StateFileDaoImpl::new(&path).find(),
            Ok(Some("- **State Version**: 8\n".to_string()))
        );
    }

    #[test]
    fn an_unreadable_state_file_carries_its_place_and_classification() {
        // ディレクトリを状態ファイルの位置に置くと、読取は不在ではない失敗になる。
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let path = root.path().join("aidlc-state.md");
        std::fs::create_dir(&path).expect("ディレクトリ");
        let error = StateFileDaoImpl::new(&path)
            .find()
            .expect_err("ディレクトリは文字列として読めない");
        assert_eq!(error.path(), Some(path.as_path()));
    }
}
