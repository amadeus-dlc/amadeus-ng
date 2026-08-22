//! `StateFileStore` ポート — 11-workspace §3 `StateFileService` の最小面。読取／audit-first
//! 遷移書込を供給する。read-only ファイルは意図的な書込バリア (rename は read-only ターゲット
//! を貫通するため W_OK 事前チェックがバリアの実装 — research workspace-state-intent §1.5)。
//! 実装は `core-interface-adapter` の Gateway (`fs_state_file_store` / `memory`)。

use std::path::Path;

/// 状態ファイル読取の失敗 (upstream `readStateFile` — 不在時 `State file not found: <path>`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateFileReadError {
    message: String,
}

impl StateFileReadError {
    /// 逐語文言 (`message_catalog::state::file_not_found` 等) を包んで持ち上げる。
    /// 文言の組み立ては Gateway 側の責務で、この型はそれを運ぶだけ。
    #[must_use]
    pub fn new(message: impl Into<String>) -> StateFileReadError {
        StateFileReadError {
            message: message.into(),
        }
    }

    /// 保持している逐語文言。upstream 出力と 1 文字も違ってはならない。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// 状態ファイル書込の失敗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateFileWriteError {
    /// 対象が存在し W_OK バリアに引っかかった (意図的な書込バリア)。
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

/// 状態ファイルの読取／アトミック書込を供給するポート。
pub trait StateFileStore {
    /// 状態ファイルを読み取る。
    ///
    /// # Errors
    ///
    /// ファイル不在・I/O エラーを `StateFileReadError` として返す。
    fn read(&self, path: &Path) -> Result<String, StateFileReadError>;

    /// tmp+rename でアトミックに書き込む。対象が存在すれば書込前に W_OK バリアを検査する。
    /// 不在なら親ディレクトリを mkdir -p する。
    ///
    /// # Errors
    ///
    /// W_OK バリア (`StateFileWriteError::ReadOnlyTarget`) と I/O エラーを返す。
    fn write_atomic(&mut self, path: &Path, content: &str) -> Result<(), StateFileWriteError>;
}
