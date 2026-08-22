//! 統合テスト (a): `write_atomic` が read-only ターゲットで Err を返す
//! (research workspace-state-intent §1.5 — read-only な `aidlc-state.md` は意図的な書込バリア)。
#![allow(clippy::unwrap_used)]

use core_interface_adapter::workspace::FsStateFileStore;
use core_use_case::workspace::{StateFileStore, StateFileWriteError};
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;

#[test]
fn write_atomic_refuses_a_read_only_target() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("aidlc-state.md");
    std::fs::write(&path, "# AI-DLC State Tracking\n").unwrap();

    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o444);
    std::fs::set_permissions(&path, perms).unwrap();

    let mut store = FsStateFileStore::new();
    let result = store.write_atomic(&path, "tampered");
    assert!(matches!(
        result,
        Err(StateFileWriteError::ReadOnlyTarget { .. })
    ));

    // 内容は不変のまま (バリアを貫通していない)。
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o644);
    std::fs::set_permissions(&path, perms).unwrap();
    assert_eq!(store.read(&path).unwrap(), "# AI-DLC State Tracking\n");
}

#[test]
fn write_atomic_succeeds_once_writable_again() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("aidlc-state.md");
    std::fs::write(&path, "old").unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o444);
    std::fs::set_permissions(&path, perms).unwrap();

    let mut store = FsStateFileStore::new();
    assert!(store.write_atomic(&path, "new").is_err());

    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o644);
    std::fs::set_permissions(&path, perms).unwrap();
    store.write_atomic(&path, "new").unwrap();
    assert_eq!(store.read(&path).unwrap(), "new");
}
