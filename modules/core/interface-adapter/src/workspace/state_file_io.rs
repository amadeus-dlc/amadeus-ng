//! `WorkflowExecutionRepository` (B-2) 実装の内部部品 — ポートではない
//! (docs/memory/gateway-taxonomy.md)。
//!
//! 状態ファイル (`aidlc-state.md`) の読取と、read-only ターゲットへの W_OK 書込バリア付き
//! アトミック書込 (upstream `readStateFile` / `writeStateFile`, 03 §5.6, 11-workspace §4)。
//! 対象が不在なら親ディレクトリを mkdir -p する。
//!
//! 「状態ファイルという格納形式」の知識はここに閉じ込める。集約 `WorkflowExecution` の
//! 永続化面は B-2 の `WorkflowExecutionRepository` が担い、その実装が本モジュールを内部で
//! 呼ぶ。trait は持たない — 差し替え点はポート側 (Repository) にあり、ここに二重の抽象を
//! 置かない。
//!
//! 現時点でプロダクトコードからの消費者は存在せず (B-2 未着手)、唯一の呼出は下の
//! ユニットテストである。upstream 逐語の契約 (not-found 文言・W_OK バリア・mkdir -p・
//! アトミック書込) を先に固定しておくためのモジュールなので、B-2 が配線するまで
//! `dead_code` を許可する。
#![allow(dead_code)]

use std::fs;
use std::io;
use std::path::Path;

/// 状態ファイル読取の失敗 (upstream `readStateFile` — 不在時 `State file not found: <path>`)。
///
/// 逐語文言 (`message_catalog::state::file_not_found` 等) を包んで運ぶだけの型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StateFileReadError {
    message: String,
}

impl StateFileReadError {
    /// 逐語文言を包んで持ち上げる。
    pub(crate) fn new(message: impl Into<String>) -> StateFileReadError {
        StateFileReadError {
            message: message.into(),
        }
    }

    /// 保持している逐語文言。upstream 出力と 1 文字も違ってはならない。
    pub(crate) fn message(&self) -> &str {
        &self.message
    }
}

/// 状態ファイル書込の失敗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StateFileWriteError {
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

/// 状態ファイルを読み取る。不在は逐語の not-found 文言、それ以外は OS の I/O 文言。
pub(crate) fn read(path: &Path) -> Result<String, StateFileReadError> {
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

/// tmp+rename でアトミックに書き込む。対象が存在すれば書込前に W_OK バリアを検査し、
/// 不在なら親ディレクトリを mkdir -p する。
pub(crate) fn write_atomic(path: &Path, content: &str) -> Result<(), StateFileWriteError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    #[test]
    fn read_reports_the_verbatim_not_found_message() {
        let dir = tempdir().unwrap();
        let err = read(&dir.path().join("aidlc-state.md")).unwrap_err();
        assert!(err.message().starts_with("State file not found: "));
    }

    #[test]
    fn write_atomic_creates_missing_parent_directories() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("spaces/default/intents/x/aidlc-state.md");
        write_atomic(&path, "# AI-DLC State Tracking\n").unwrap();
        assert_eq!(read(&path).unwrap(), "# AI-DLC State Tracking\n");
    }

    #[test]
    fn read_maps_non_not_found_errors_to_their_io_message() {
        let dir = tempdir().unwrap();
        // ディレクトリの read_to_string は NotFound 以外 (EISDIR) で失敗する
        let err = read(dir.path()).unwrap_err();
        assert!(!err.message().starts_with("State file not found: "));
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
