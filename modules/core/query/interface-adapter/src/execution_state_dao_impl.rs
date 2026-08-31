//! `ExecutionStateDao` の実 Gateway — 実行状態リードモデルのパス解決とファイル読取
//! (I/O はここに閉じる)。
//!
//! record ディレクトリ直下の `aidlc-state.md` を読み、本文を純 parse
//! ([`parse_execution_state`]) へ渡す。**クエリ側のアダプタが fs を読むのは正当**である —
//! リードモデルを読むのがこの層の仕事である (`coding-rules/cqrs-boundaries.md` 規則 6)。
//!
//! **「存在しない」は失敗ではない** — active-intent がまだ無いワークフロー (誕生分岐 4a) は
//! 正常な観測であり、`Ok(None)` として運ぶ。読めたが壊れている場合だけが失敗である。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use core_query_use_case::orchestration::{
    ExecutionStateDao, ExecutionStateReadError, ExecutionStateView,
};

use super::execution_state_parse::parse_execution_state;

/// 状態ファイルの名前 (record ディレクトリ直下)。
const STATE_FILE: &str = "aidlc-state.md";

/// record ディレクトリ配下の状態ファイルを読む実装。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionStateDaoImpl {
    record_dir: PathBuf,
}

impl ExecutionStateDaoImpl {
    /// intent の record ディレクトリ (`aidlc/spaces/<space>/intents/<slug>-<id8>`) を指す。
    #[must_use]
    pub const fn new(record_dir: PathBuf) -> ExecutionStateDaoImpl {
        ExecutionStateDaoImpl { record_dir }
    }

    /// record ディレクトリ直下の `aidlc-state.md`。
    fn state_file_path(&self) -> PathBuf {
        self.record_dir.join(STATE_FILE)
    }
}

impl ExecutionStateDao for ExecutionStateDaoImpl {
    fn find(&self) -> Result<Option<ExecutionStateView>, ExecutionStateReadError> {
        let path = self.state_file_path();
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => {
                return Err(ExecutionStateReadError::NotReadable {
                    path: display(&path),
                    cause: e.to_string(),
                });
            }
        };
        parse_execution_state(&text)
            .map(Some)
            .map_err(|cause| ExecutionStateReadError::Malformed {
                path: display(&path),
                // 復号器の型はこの層の持ち物なので、ポート面へは描写だけを渡す
                // (`coding-rules/domain-persistence-neutrality.md` と同じ趣旨 —
                // 契約に内部実装を出さない)。
                cause: cause.to_string(),
            })
    }
}

/// パスの綴り (`Path::display` の写し)。
fn display(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    // panic! は想定外バリアントの即時失敗という検証用途で使っており、テスト失敗のシグナル
    // として妥当なため許容する。
    #![allow(clippy::panic)]

    use super::*;
    use core_query_use_case::orchestration::ExecutionStatus;
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
    fn a_readable_state_file_becomes_the_query_model() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("aidlc-state.md"), state_file()).unwrap();
        let found = ExecutionStateDaoImpl::new(dir.path().to_path_buf())
            .find()
            .unwrap();
        let Some(view) = found else {
            panic!("読めたはず")
        };
        assert_eq!(view.scope().as_str(), "classic");
        assert_eq!(view.status(), ExecutionStatus::Running);
        assert_eq!(view.stage_count(), 1);
    }

    #[test]
    fn the_state_file_sits_directly_under_the_record_dir() {
        let dir = tempdir().unwrap();
        assert_eq!(
            ExecutionStateDaoImpl::new(dir.path().to_path_buf()).state_file_path(),
            dir.path().join("aidlc-state.md")
        );
    }

    #[test]
    fn an_absent_state_file_is_a_normal_observation() {
        let dir = tempdir().unwrap();
        assert_eq!(
            ExecutionStateDaoImpl::new(dir.path().to_path_buf())
                .find()
                .unwrap(),
            None
        );
    }

    #[test]
    fn an_unreadable_target_is_reported_with_its_path() {
        let dir = tempdir().unwrap();
        // 状態ファイルの位置にディレクトリを置く — read_to_string は EISDIR で失敗する。
        fs::create_dir(dir.path().join("aidlc-state.md")).unwrap();
        let error = ExecutionStateDaoImpl::new(dir.path().to_path_buf())
            .find()
            .unwrap_err();
        assert!(
            matches!(error, ExecutionStateReadError::NotReadable { .. }),
            "{error:?}"
        );
        assert!(error.to_string().contains("aidlc-state.md"));
    }

    #[test]
    fn a_malformed_state_file_is_reported_with_its_cause() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("aidlc-state.md"),
            state_file().replace("- **Status**: Running", ""),
        )
        .unwrap();
        let error = ExecutionStateDaoImpl::new(dir.path().to_path_buf())
            .find()
            .unwrap_err();
        let ExecutionStateReadError::Malformed { ref cause, .. } = error else {
            panic!("復号の拒否のはず")
        };
        assert_eq!(
            cause, "missing field \"Status\"",
            "復号の拒否理由は描写だけを渡す"
        );
        assert!(error.to_string().contains("aidlc-state.md"));
    }
}
