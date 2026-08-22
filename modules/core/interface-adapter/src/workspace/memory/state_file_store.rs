//! `StateFileStore` の in-memory 実装 (テスト用)。W_OK バリアは `mark_read_only` で明示的に
//! シミュレートする。

use core_use_case::workspace::{StateFileReadError, StateFileStore, StateFileWriteError};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// プロセス内 `HashMap` を裏に持つ `StateFileStore`。`write_atomic` の tmp+rename は
/// 意味を持たないため単なる差し替えで、read-only バリアだけを `mark_read_only` で再現する。
#[derive(Debug, Default)]
pub struct InMemoryStateFileStore {
    files: HashMap<PathBuf, String>,
    read_only: HashSet<PathBuf>,
}

impl InMemoryStateFileStore {
    /// ファイルも read-only 指定も無い空の状態で作る。
    #[must_use]
    pub fn new() -> InMemoryStateFileStore {
        InMemoryStateFileStore::default()
    }

    /// 初期内容を設定する (mkdir -p 相当の準備なしに read が成功する状態を作る)。
    pub fn seed(&mut self, path: &Path, content: &str) {
        self.files.insert(path.to_path_buf(), content.to_string());
    }

    /// 以後この path への `write_atomic` を W_OK バリアで拒否する。
    pub fn mark_read_only(&mut self, path: &Path) {
        self.read_only.insert(path.to_path_buf());
    }
}

impl StateFileStore for InMemoryStateFileStore {
    fn read(&self, path: &Path) -> Result<String, StateFileReadError> {
        self.files.get(path).cloned().ok_or_else(|| {
            StateFileReadError::new(message_catalog::state::file_not_found(
                &path.display().to_string(),
            ))
        })
    }

    fn write_atomic(&mut self, path: &Path, content: &str) -> Result<(), StateFileWriteError> {
        if self.files.contains_key(path) && self.read_only.contains(path) {
            return Err(StateFileWriteError::ReadOnlyTarget {
                message: format!("state file is read-only: {}", path.display()),
            });
        }
        self.files.insert(path.to_path_buf(), content.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_reports_not_found_for_unseeded_paths() {
        let store = InMemoryStateFileStore::new();
        let err = store.read(Path::new("/nope")).unwrap_err();
        assert!(err.message().starts_with("State file not found: "));
    }

    #[test]
    fn write_then_read_round_trips() {
        let mut store = InMemoryStateFileStore::new();
        let path = PathBuf::from("/space/intent/aidlc-state.md");
        store.write_atomic(&path, "content").unwrap();
        assert_eq!(store.read(&path).unwrap(), "content");
    }

    #[test]
    fn mark_read_only_blocks_further_writes() {
        let mut store = InMemoryStateFileStore::new();
        let path = PathBuf::from("/space/intent/aidlc-state.md");
        store.seed(&path, "original");
        store.mark_read_only(&path);
        assert!(store.write_atomic(&path, "changed").is_err());
        assert_eq!(store.read(&path).unwrap(), "original");
    }
}
