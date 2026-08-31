//! 実行状態リードモデルの reader — パス解決とファイル読取 (I/O はここに閉じる)。
//!
//! record ディレクトリ直下の `aidlc-state.md` を読み、本文を純 parse
//! ([`parse_execution_state`]) へ渡す。**クエリ側のアダプタが fs を読むのは正当**である —
//! リードモデルを読むのがこの層の仕事である (`coding-rules/cqrs-boundaries.md` 規則 6)。
//!
//! **「存在しない」は失敗ではない** — active-intent がまだ無いワークフロー (誕生分岐 4a) は
//! 正常な観測であり、[`ExecutionStateSource::Missing`] として運ぶ。読めたが壊れている場合
//! だけが失敗である。
//!
//! [`ExecutionStateSource::Missing`]: core_query_use_case::orchestration::ExecutionStateSource

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use core_query_use_case::execution_view::ExecutionStateView;

use super::execution_state_parse::{ExecutionStateParseError, parse_execution_state};

/// 状態ファイルの名前 (record ディレクトリ直下)。
const STATE_FILE: &str = "aidlc-state.md";

/// 状態ファイル読取の失敗 (不在は失敗ではない — [`LoadedExecutionState::Missing`])。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStateReadError {
    /// ファイルは在るが読めない (権限・UTF-8 破損・EISDIR 等)。
    NotReadable {
        /// 読もうとした解決済みパス。
        path: String,
        /// 失敗の理由 (OS 由来)。
        cause: String,
    },
    /// 読めたが状態ファイルとして復号できない。
    Malformed {
        /// 読んだ解決済みパス。
        path: String,
        /// 復号の拒否理由 (材料のみ)。
        cause: ExecutionStateParseError,
    },
}

impl std::fmt::Display for ExecutionStateReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStateReadError::NotReadable { path, cause } => {
                write!(f, "{path}: {cause}")
            }
            ExecutionStateReadError::Malformed { path, cause } => {
                write!(f, "{path}: {cause}")
            }
        }
    }
}

impl std::error::Error for ExecutionStateReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecutionStateReadError::Malformed { cause, .. } => Some(cause),
            ExecutionStateReadError::NotReadable { .. } => None,
        }
    }
}

/// 読取の結果 — 在る / 無い の 2 値 (失敗は `Err` で運ぶ)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadedExecutionState {
    /// 状態ファイルが存在しない (稼働中ワークフローなし — 誕生分岐へ)。
    Missing,
    /// 読めたリードモデル。
    Loaded(ExecutionStateView),
}

/// record ディレクトリ直下の `aidlc-state.md` を解決する。
#[must_use]
pub fn state_file_path(record_dir: &Path) -> PathBuf {
    record_dir.join(STATE_FILE)
}

/// 状態ファイルを読み、クエリモデルを組み立てて返す。
///
/// 呼出のたびに読み直す (キャッシュ戦略は観測不能なので実装の自由)。
///
/// # Errors
///
/// ファイルは在るのに読めない (`NotReadable`)、読めたが復号できない (`Malformed`)。
/// **不在はエラーにしない** — [`LoadedExecutionState::Missing`] で返す。
pub fn load_execution_state(
    record_dir: &Path,
) -> Result<LoadedExecutionState, ExecutionStateReadError> {
    let path = state_file_path(record_dir);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Ok(LoadedExecutionState::Missing);
        }
        Err(e) => {
            return Err(ExecutionStateReadError::NotReadable {
                path: path.display().to_string(),
                cause: e.to_string(),
            });
        }
    };
    parse_execution_state(&text)
        .map(LoadedExecutionState::Loaded)
        .map_err(|cause| ExecutionStateReadError::Malformed {
            path: path.display().to_string(),
            cause,
        })
}

#[cfg(test)]
mod tests {
    // panic! は想定外バリアントの即時失敗という検証用途で使っており、テスト失敗のシグナル
    // として妥当なため許容する。
    #![allow(clippy::panic)]

    use super::*;
    use core_query_use_case::execution_view::ExecutionStatus;
    use tempfile::tempdir;

    fn state_file() -> String {
        [
            "# AI-DLC State Tracking",
            "- **Scope**: classic",
            "",
            "## Stage Progress",
            "### INITIALIZATION PHASE",
            "- [-] state-init — EXECUTE",
            "",
            "## Current Status",
            "- **Current Stage**: state-init",
            "- **Status**: Running",
            "- **Last Updated**: 2026-08-29T16:36:24Z",
            "",
        ]
        .join("\n")
    }

    #[test]
    fn the_state_file_sits_directly_under_the_record_dir() {
        let dir = tempdir().unwrap();
        assert_eq!(
            state_file_path(dir.path()),
            dir.path().join("aidlc-state.md")
        );
    }

    #[test]
    fn a_readable_state_file_becomes_the_query_model() {
        let dir = tempdir().unwrap();
        fs::write(state_file_path(dir.path()), state_file()).unwrap();
        let loaded = load_execution_state(dir.path()).unwrap();
        let LoadedExecutionState::Loaded(view) = loaded else {
            panic!("読めたはず")
        };
        assert_eq!(view.scope().as_str(), "classic");
        assert_eq!(view.status(), ExecutionStatus::Running);
        assert_eq!(view.stage_count(), 1);
    }

    #[test]
    fn an_absent_state_file_is_a_normal_observation() {
        let dir = tempdir().unwrap();
        assert_eq!(
            load_execution_state(dir.path()).unwrap(),
            LoadedExecutionState::Missing
        );
    }

    #[test]
    fn an_unreadable_target_is_reported_with_its_path() {
        let dir = tempdir().unwrap();
        // 状態ファイルの位置にディレクトリを置く — read_to_string は EISDIR で失敗する。
        fs::create_dir(state_file_path(dir.path())).unwrap();
        let error = load_execution_state(dir.path()).unwrap_err();
        assert!(
            matches!(error, ExecutionStateReadError::NotReadable { .. }),
            "{error:?}"
        );
        assert!(error.to_string().contains("aidlc-state.md"));
        let boxed: Box<dyn std::error::Error> = Box::new(error);
        assert!(boxed.source().is_none());
    }

    #[test]
    fn a_malformed_state_file_is_reported_with_its_cause() {
        let dir = tempdir().unwrap();
        fs::write(
            state_file_path(dir.path()),
            state_file().replace("- **Status**: Running", ""),
        )
        .unwrap();
        let error = load_execution_state(dir.path()).unwrap_err();
        let ExecutionStateReadError::Malformed { ref cause, .. } = error else {
            panic!("復号の拒否のはず")
        };
        assert_eq!(
            *cause,
            ExecutionStateParseError::MissingField {
                field: "Status".to_string()
            }
        );
        assert!(error.to_string().contains("missing field \"Status\""));
        let boxed: Box<dyn std::error::Error> = Box::new(error);
        assert!(boxed.source().is_some(), "内側の拒否は source で辿れる");
    }
}
