//! `StateFileStore` の実 Gateway — read-only ターゲットへの W_OK 書込バリア + infra-io の
//! アトミック書込 (upstream `readStateFile` / `writeStateFile`, 03 §5.6, 11-workspace §4)。
//! 不在なら親ディレクトリを mkdir -p する。

use core_use_case::workspace::state_file_store::{
    StateFileReadError, StateFileStore, StateFileWriteError,
};
use std::fs;
use std::io;
use std::path::Path;

/// 実ファイルシステム上の `StateFileStore` 実装。内部状態を持たない (I/O はすべて呼出のたび
/// にオペレーティングシステムへ委譲する)。
#[derive(Debug, Clone, Copy, Default)]
pub struct FsStateFileStore;

impl FsStateFileStore {
    #[must_use]
    pub const fn new() -> FsStateFileStore {
        FsStateFileStore
    }
}

impl StateFileStore for FsStateFileStore {
    fn read(&self, path: &Path) -> Result<String, StateFileReadError> {
        fs::read_to_string(path).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                StateFileReadError::new(message_catalog::state::file_not_found(
                    &path.display().to_string(),
                ))
            } else {
                StateFileReadError::new(e.to_string())
            }
        })
    }

    fn write_atomic(&mut self, path: &Path, content: &str) -> Result<(), StateFileWriteError> {
        if path.exists() {
            match infra_io::fs_meta::is_writable(path) {
                Ok(true) => {}
                Ok(false) => {
                    return Err(StateFileWriteError::ReadOnlyTarget {
                        message: format!("state file is read-only: {}", path.display()),
                    });
                }
                Err(e) => {
                    return Err(StateFileWriteError::Io {
                        message: e.to_string(),
                    });
                }
            }
        } else if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| StateFileWriteError::Io {
                message: e.to_string(),
            })?;
        }
        infra_io::atomic::write_file_atomic(path, content.as_bytes()).map_err(|e| {
            StateFileWriteError::Io {
                message: e.to_string(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn read_reports_the_verbatim_not_found_message() {
        let dir = tempdir().unwrap();
        let store = FsStateFileStore::new();
        let err = store.read(&dir.path().join("aidlc-state.md")).unwrap_err();
        assert!(err.message().starts_with("State file not found: "));
    }

    #[test]
    fn write_atomic_creates_missing_parent_directories() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("spaces/default/intents/x/aidlc-state.md");
        let mut store = FsStateFileStore::new();
        store
            .write_atomic(&path, "# AI-DLC State Tracking\n")
            .unwrap();
        assert_eq!(store.read(&path).unwrap(), "# AI-DLC State Tracking\n");
    }

    #[test]
    fn read_maps_non_not_found_errors_to_their_io_message() {
        let dir = tempdir().unwrap();
        let store = FsStateFileStore::new();
        // ディレクトリの read_to_string は NotFound 以外 (EISDIR) で失敗する
        let err = store.read(dir.path()).unwrap_err();
        assert!(!err.message().starts_with("State file not found: "));
    }

    #[test]
    fn write_atomic_maps_parent_creation_failure_to_io() {
        let dir = tempdir().unwrap();
        let occupied = dir.path().join("occupied");
        fs::write(&occupied, b"x").unwrap();
        let mut store = FsStateFileStore::new();
        // 親の位置に regular file がいるので mkdir -p は ENOTDIR で失敗する
        let err = store
            .write_atomic(&occupied.join("child/aidlc-state.md"), "c")
            .unwrap_err();
        assert!(matches!(err, StateFileWriteError::Io { .. }));
    }

    #[test]
    fn write_atomic_maps_atomic_write_failure_to_io() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("occupied-dir");
        fs::create_dir(&target).unwrap();
        let mut store = FsStateFileStore::new();
        // 対象が既存ディレクトリ: W_OK バリアは通るが rename (file → dir) が失敗する
        let err = store.write_atomic(&target, "c").unwrap_err();
        assert!(matches!(err, StateFileWriteError::Io { .. }));
    }

    #[test]
    fn write_atomic_round_trips_through_read() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("aidlc-state.md");
        let mut store = FsStateFileStore::new();
        store.write_atomic(&path, "one").unwrap();
        store.write_atomic(&path, "two").unwrap();
        assert_eq!(store.read(&path).unwrap(), "two");
    }
}
