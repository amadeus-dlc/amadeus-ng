//! 状態ファイルの読取 — リードモデル `aidlc-state.md` の読取
//! (upstream `readStateFile`, 03 §5.6, 11-workspace §4)。
//!
//! 「状態ファイルという格納形式」の知識はここに閉じ込める。状態ファイルは**リードモデル**で
//! あり (ADR-004)、書くのは投影だけである — コマンド側は集約から最新状態を得るので、この
//! モジュールを見ない (`coding-rules/cqrs-boundaries.md` 規則 4)。

use std::fs;
use std::io;
use std::path::Path;

/// 状態ファイル読取の失敗 (upstream `readStateFile` — 不在時 `State file not found: <path>`)。
///
/// 逐語文言 (`super::wording::file_not_found_message` 等) を包んで運ぶだけの型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateFileReadError {
    message: String,
}

impl StateFileReadError {
    /// 逐語文言を包んで持ち上げる。
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

/// 状態ファイルを読み取る。不在は逐語の not-found 文言、それ以外は OS の I/O 文言。
///
/// # Errors
///
/// 読めなければ逐語文言を包んだ `StateFileReadError`。
pub fn read(path: &Path) -> Result<String, StateFileReadError> {
    fs::read_to_string(path).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            StateFileReadError::new(super::wording::file_not_found_message(
                &path.display().to_string(),
            ))
        } else {
            StateFileReadError::new(e.to_string())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn read_reports_the_verbatim_not_found_message() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("aidlc-state.md");
        let err = read(&path).unwrap_err();
        // 逐語契約なので完全一致で pin する (接頭辞比較では末尾・パス部の変化を見逃す)
        assert_eq!(
            err.message(),
            format!("State file not found: {}", path.display())
        );
    }

    #[test]
    fn read_maps_non_not_found_errors_to_their_io_message() {
        let dir = tempdir().unwrap();
        // ディレクトリの read_to_string は NotFound 以外 (EISDIR) で失敗する
        let err = read(dir.path()).unwrap_err();
        assert!(!err.message().starts_with("State file not found: "));
    }
}
