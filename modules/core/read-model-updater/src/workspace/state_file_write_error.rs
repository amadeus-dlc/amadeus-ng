//! 状態ファイルの書込 — リードモデル `aidlc-state.md` への書込
//! (upstream `writeStateFile`, 03 §5.6, 11-workspace §4)。
//!
//! read-only ターゲットへの W_OK 書込バリア付きアトミック書込 (tmp+rename)。対象が不在なら
//! 親ディレクトリを mkdir -p する。trait は持たない — 差し替え点は取得ループ側にあり、
//! ここに二重の抽象を置かない。
//!
//! **利用制約**: W_OK 検査→rename の間の TOCTOU 窓は upstream (`accessSync` → write) と
//! 同一の観測挙動である。書込の直列化は mkdir ロックではなく SQLite の書込トランザクションと
//! 楽観バージョンが担う (ADR-007 でロック機構は退役した)。

use std::fs;
use std::path::Path;

/// 状態ファイル書込の失敗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateFileWriteError {
    /// 対象が存在し W_OK バリアに引っかかった (意図的な書込バリア —
    /// research workspace-state-intent §1.5。rename は read-only ターゲットを貫通するため
    /// W_OK 事前チェックがバリアの実装)。
    ReadOnlyTarget {
        /// バリアに引っかかった対象の説明。
        message: String,
    },
    /// その他の I/O エラー。
    Io {
        /// 失敗した I/O 操作の説明。
        message: String,
    },
}

/// tmp+rename でアトミックに書き込む。対象が存在すれば書込前に W_OK バリアを検査し、
/// 不在なら親ディレクトリを mkdir -p する。
///
/// # Errors
///
/// read-only ターゲット (`ReadOnlyTarget`)、その他の I/O 失敗 (`Io`)。
pub fn write_atomic(path: &Path, content: &str) -> Result<(), StateFileWriteError> {
    if path.exists() {
        match core_infrastructure::fs_meta::is_writable(path) {
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
    core_infrastructure::atomic::write_file_atomic(path, content.as_bytes()).map_err(|e| {
        StateFileWriteError::Io {
            message: e.to_string(),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    use super::super::state_file_read_error::read;

    #[test]
    fn write_atomic_creates_missing_parent_directories() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("spaces/default/intents/x/aidlc-state.md");
        write_atomic(&path, "# AI-DLC State Tracking\n").unwrap();
        assert_eq!(read(&path).unwrap(), "# AI-DLC State Tracking\n");
    }

    #[test]
    fn write_atomic_maps_parent_creation_failure_to_io() {
        let dir = tempdir().unwrap();
        let occupied = dir.path().join("occupied");
        fs::write(&occupied, b"x").unwrap();
        // 親の位置に regular file がいるので mkdir -p は ENOTDIR で失敗する
        let err = write_atomic(&occupied.join("child/aidlc-state.md"), "c").unwrap_err();
        assert!(matches!(err, StateFileWriteError::Io { .. }));
    }

    #[test]
    fn write_atomic_maps_atomic_write_failure_to_io() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("occupied-dir");
        fs::create_dir(&target).unwrap();
        // 対象が既存ディレクトリ: W_OK バリアは通るが rename (file → dir) が失敗する
        let err = write_atomic(&target, "c").unwrap_err();
        assert!(matches!(err, StateFileWriteError::Io { .. }));
    }

    #[test]
    fn write_atomic_round_trips_through_read() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("aidlc-state.md");
        write_atomic(&path, "one").unwrap();
        write_atomic(&path, "two").unwrap();
        assert_eq!(read(&path).unwrap(), "two");
    }

    /// 旧 `tests/fs_state_file_store_test.rs` (a) からの移設 — read-only な `aidlc-state.md`
    /// は意図的な書込バリア (research workspace-state-intent §1.5)。
    #[test]
    fn write_atomic_refuses_a_read_only_target() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("aidlc-state.md");
        fs::write(&path, "# AI-DLC State Tracking\n").unwrap();

        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o444);
        fs::set_permissions(&path, perms).unwrap();

        let result = write_atomic(&path, "tampered");
        assert!(matches!(
            result,
            Err(StateFileWriteError::ReadOnlyTarget { .. })
        ));

        // 内容は不変のまま (バリアを貫通していない)。
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&path, perms).unwrap();
        assert_eq!(read(&path).unwrap(), "# AI-DLC State Tracking\n");
    }

    /// 旧 `tests/fs_state_file_store_test.rs` からの移設 — バリアは permission の状態にのみ
    /// 依存し、一度拒否したパスを恒久的に閉ざさない。
    #[test]
    fn write_atomic_succeeds_once_writable_again() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("aidlc-state.md");
        fs::write(&path, "old").unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o444);
        fs::set_permissions(&path, perms).unwrap();

        assert!(write_atomic(&path, "new").is_err());

        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&path, perms).unwrap();
        write_atomic(&path, "new").unwrap();
        assert_eq!(read(&path).unwrap(), "new");
    }
}
