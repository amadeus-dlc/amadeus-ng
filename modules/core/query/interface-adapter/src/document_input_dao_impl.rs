//! `DocumentInputDao` の実 Gateway — 転送ファイルと、そこが名指す 1 ファイルを生バイトで読む。
//!
//! 媒体はファイルであり、SQLite の `read_*` 表とは別の面である — したがって
//! [`super::ReadModelDaos`] (1 要求 1 接続) の住人ではなく、2 つの所在 (プロジェクトルートと
//! 活動記録) だけを握る ([`super::ProjectDescriptionDaoImpl`] と同型)。
//!
//! # ここが閉じる攻撃面
//!
//! 名指されたパスは**顧客が書いたバイト**である。したがって
//!
//! - シェルへは渡さない (この層はコマンドを起動しない)。
//! - プロジェクトルートの外へは出さない — 字句解決の後に接頭辞で確かめる。
//! - 経路のどの成分もシンボリックリンクなら拒む — 途中のディレクトリを差し替える形の
//!   逃げ道を、末尾だけの検査では塞げないからである。
//! - 最終成分は `O_NOFOLLOW` で開く — 検査と `open` の間の差し替え (TOCTOU) を閉じる。
//! - 通常ファイルでなければ読む前に拒む — FIFO・デバイス・ディレクトリは `read` が
//!   永遠に返らないことがある。
//! - 上限を超えたら読み切らずに拒む — 上限の判定のために全量を確保しない。

use std::io::Read as _;
use std::path::{Component, Path, PathBuf};

use core_query_use_case::orchestration::{
    DocumentInputBytes, DocumentInputDao, DocumentInputReadError,
};

/// 活動記録直下の転送ファイル名 (upstream `DOCUMENT_INPUT_REQUEST_FILE` 逐語)。
///
/// 名前は**この層の内部詳細**である — ポート面は「転送ファイル」としか語らない。利用者へ
/// 見せる綴りは提示側 (`wording`) が別に持つ。
const DOCUMENT_INPUT_REQUEST_FILE: &str = ".aidlc-document-input-path";

/// 転送ファイルのバイト上限 (upstream `requestFileByteCap` 逐語)。
///
/// 1 行のパスしか載らない面なので、文書側の上限ではなく**パス長の上限**を当てる。上限が
/// 無いと、疎な巨大ファイルが 1 行検査の前に全量確保・復号されて落ちる。4096 は対応する
/// どのプラットフォームの `PATH_MAX` も覆い、末尾改行の分も含む。
const REQUEST_FILE_BYTE_CAP: u64 = 4096;

/// 文書側のバイト上限 (upstream `documentInputByteCap` = 文字上限 × 4 逐語)。
///
/// UTF-8 の 1 文字は最大 4 バイトなので、文字上限を超える入力はこのバイト上限も必ず超える。
/// バイト側で先に切ることで、文字数を数えるために全量を復号せずに済む。
const DOCUMENT_BYTE_CAP: u64 =
    core_query_use_case::orchestration::EXTRACT_OUTPUT_CHAR_CAP as u64 * 4;

/// 直接入力の 2 面を読む実装。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentInputDaoImpl {
    project_dir: PathBuf,
    request_file: PathBuf,
}

impl DocumentInputDaoImpl {
    /// プロジェクトルートと転送ファイルの所在を受け取る (**この型の唯一の構築経路**)。
    #[must_use]
    pub fn new(project_dir: &Path, record_dir: &Path) -> DocumentInputDaoImpl {
        DocumentInputDaoImpl {
            project_dir: project_dir.to_path_buf(),
            request_file: record_dir.join(DOCUMENT_INPUT_REQUEST_FILE),
        }
    }
}

impl DocumentInputDao for DocumentInputDaoImpl {
    fn find_request(&self) -> Result<Vec<u8>, DocumentInputReadError> {
        read_regular_no_follow(
            &self.request_file,
            DOCUMENT_INPUT_REQUEST_FILE,
            REQUEST_FILE_BYTE_CAP,
        )
    }

    fn find_document(&self, requested: &str) -> Result<DocumentInputBytes, DocumentInputReadError> {
        let outside = || DocumentInputReadError::OutsideProject {
            requested: requested.to_string(),
        };
        let root = std::fs::canonicalize(&self.project_dir).map_err(|_| outside())?;
        let candidate = Path::new(requested);
        let absolute = if candidate.is_absolute() {
            normalize(candidate)
        } else {
            normalize(&root.join(candidate))
        };
        let relative = absolute.strip_prefix(&root).map_err(|_| outside())?;
        // `rel === ""` (ルート自身) も upstream は拒む。
        let mut parts = Vec::new();
        for part in relative.components() {
            match part {
                Component::Normal(part) => parts.push(part.to_str().ok_or_else(outside)?),
                // 字句解決の後に `..` や接頭辞が残るのは、ルートの外を指した綴りである。
                _ => return Err(outside()),
            }
        }
        if parts.is_empty() {
            return Err(outside());
        }
        let portable = parts.join("/");
        // 途中の成分がシンボリックリンクなら、差し替えられたディレクトリ経由の逃げ道に
        // なるので、末尾だけでなく経路全体を見る。
        let mut guarded = root;
        for part in &parts {
            guarded.push(part);
            let metadata = std::fs::symlink_metadata(&guarded)
                .map_err(|error| unreadable(&portable, &error))?;
            if metadata.is_symlink() {
                return Err(DocumentInputReadError::Unreadable {
                    what: portable,
                    cause: format!(
                        "{} is a symlink, and no path component may be a symlink here",
                        guarded.display()
                    ),
                });
            }
        }
        let bytes = read_regular_no_follow(&guarded, &portable, DOCUMENT_BYTE_CAP)?;
        Ok(DocumentInputBytes::new(portable, bytes))
    }
}

fn unreadable(what: &str, error: &std::io::Error) -> DocumentInputReadError {
    DocumentInputReadError::Unreadable {
        what: what.to_string(),
        cause: error.to_string(),
    }
}

/// 通常ファイルを非追従で開き、上限まで読む。上限を超えたら**読み切らずに**拒む。
fn read_regular_no_follow(
    path: &Path,
    what: &str,
    cap: u64,
) -> Result<Vec<u8>, DocumentInputReadError> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        // `O_NONBLOCK` は `O_NOFOLLOW` と同じだけ効く — FIFO は writer が現れるまで
        // `open` の中で待つので、これが無いと種別を検査する前に止まる。
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let mut file = options
        .open(path)
        .map_err(|error| unreadable(what, &error))?;
    let metadata = file.metadata().map_err(|error| unreadable(what, &error))?;
    if !metadata.is_file() {
        return Err(DocumentInputReadError::Unreadable {
            what: what.to_string(),
            cause: format!(
                "not a regular file ({})",
                if metadata.is_dir() {
                    "a directory"
                } else {
                    "a special file"
                }
            ),
        });
    }
    let mut bytes = Vec::new();
    // 上限 + 1 バイトだけ読む — 超過の判定に全量を確保しない。
    file.by_ref()
        .take(cap + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| unreadable(what, &error))?;
    if bytes.len() as u64 > cap {
        return Err(DocumentInputReadError::Unreadable {
            what: what.to_string(),
            cause: format!("exceeds the {cap}-byte limit"),
        });
    }
    Ok(bytes)
}

/// ファイルを開かずに `.` と `..` を畳む字句解決 (upstream `resolve`)。
fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    struct Workspace {
        root: tempfile::TempDir,
    }

    impl Workspace {
        fn new() -> Workspace {
            let root = tempfile::tempdir().expect("一時ディレクトリ");
            std::fs::create_dir_all(root.path().join("record")).expect("record");
            Workspace { root }
        }

        fn dao(&self) -> DocumentInputDaoImpl {
            DocumentInputDaoImpl::new(self.root.path(), &self.root.path().join("record"))
        }

        fn write(&self, relative: &str, content: &[u8]) {
            let path = self.root.path().join(relative);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("親");
            }
            std::fs::write(path, content).expect("書込");
        }
    }

    #[test]
    fn an_absent_request_file_is_a_refusal_not_an_empty_observation() {
        let workspace = Workspace::new();
        assert!(matches!(
            workspace.dao().find_request(),
            Err(DocumentInputReadError::Unreadable { .. })
        ));
    }

    #[test]
    fn the_request_file_is_read_verbatim() {
        let workspace = Workspace::new();
        workspace.write("record/.aidlc-document-input-path", b"docs/request.md\n");
        assert_eq!(
            workspace.dao().find_request(),
            Ok(b"docs/request.md\n".to_vec())
        );
    }

    #[test]
    fn a_request_file_over_the_path_cap_is_refused_before_it_is_decoded() {
        let workspace = Workspace::new();
        workspace.write(
            "record/.aidlc-document-input-path",
            &vec![b'a'; REQUEST_FILE_BYTE_CAP as usize + 1],
        );
        assert!(matches!(
            workspace.dao().find_request(),
            Err(DocumentInputReadError::Unreadable { cause, .. }) if cause.contains("4096-byte limit")
        ));
    }

    #[test]
    fn a_contained_regular_file_is_read_with_its_portable_path() {
        let workspace = Workspace::new();
        workspace.write("docs/request.md", "本文".as_bytes());
        let found = workspace
            .dao()
            .find_document("docs/request.md")
            .expect("読取");
        assert_eq!(found.path(), "docs/request.md");
        assert_eq!(found.bytes(), "本文".as_bytes());
    }

    /// 同じ綴りでも、解決先がルートの外なら受理しない。
    #[test]
    fn a_path_that_climbs_out_of_the_project_is_refused() {
        let workspace = Workspace::new();
        for requested in [
            "../outside/request.md",
            "/etc/hosts",
            "docs/../../request.md",
        ] {
            assert_eq!(
                workspace.dao().find_document(requested),
                Err(DocumentInputReadError::OutsideProject {
                    requested: requested.to_string(),
                }),
                "{requested}"
            );
        }
    }

    /// ルート自身を指す綴りは 1 ファイルの名指しではない。
    #[test]
    fn a_path_that_resolves_to_the_root_itself_is_refused() {
        let workspace = Workspace::new();
        assert_eq!(
            workspace.dao().find_document("."),
            Err(DocumentInputReadError::OutsideProject {
                requested: ".".to_string(),
            })
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_is_refused_whether_it_is_the_last_component_or_a_parent() {
        let workspace = Workspace::new();
        workspace.write("docs/request.md", b"inside");
        std::os::unix::fs::symlink(
            workspace.root.path().join("docs/request.md"),
            workspace.root.path().join("docs/alias.md"),
        )
        .expect("末尾リンク");
        std::os::unix::fs::symlink(
            workspace.root.path().join("docs"),
            workspace.root.path().join("linkdir"),
        )
        .expect("親リンク");
        for requested in ["docs/alias.md", "linkdir/request.md"] {
            assert!(
                matches!(
                    workspace.dao().find_document(requested),
                    Err(DocumentInputReadError::Unreadable { cause, .. }) if cause.contains("symlink")
                ),
                "{requested}"
            );
        }
    }

    #[test]
    fn a_directory_is_refused_before_any_read() {
        let workspace = Workspace::new();
        workspace.write("docs/request.md", b"inside");
        assert!(matches!(
            workspace.dao().find_document("docs"),
            Err(DocumentInputReadError::Unreadable { cause, .. }) if cause.contains("a directory")
        ));
    }

    #[test]
    fn a_document_over_the_byte_cap_is_refused() {
        let workspace = Workspace::new();
        workspace.write(
            "docs/request.md",
            &vec![b'a'; DOCUMENT_BYTE_CAP as usize + 1],
        );
        assert!(matches!(
            workspace.dao().find_document("docs/request.md"),
            Err(DocumentInputReadError::Unreadable { cause, .. }) if cause.contains("-byte limit")
        ));
    }
}
