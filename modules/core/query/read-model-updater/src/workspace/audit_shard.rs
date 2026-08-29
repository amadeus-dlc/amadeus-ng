//! 監査シャードの投影ライタ — 台帳への追記（03 §6.1 / 11-workspace §4）。
//!
//! シャードは**追記専用**である。既存の行は読まないし、書き換えもしない。空のファイル
//! （サイズ 0）への最初の書込のときだけヘッダ行 `# AI-DLC Audit Log\n` を先に置く
//! （upstream `appendAuditBlockAtPath`）。
//!
//! 追記のオープンは `core-infrastructure` の追記専用プリミティブを通す — シンボリックリンクを
//! 辿らない `O_APPEND | O_NOFOLLOW` 相当であり、リンクをすり替えて別ファイルへ書かせる経路を
//! 塞ぐ。原子性の根拠は単一 syscall ではなく `O_APPEND` である。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::audit_block::SHARD_HEADER;

/// 監査シャードへの追記の失敗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditShardWriteError {
    /// I/O の失敗（分類だけを運ぶ — 文言はアダプタ層）。
    Io {
        /// OS 由来の分類。
        kind: io::ErrorKind,
    },
}

impl core::fmt::Display for AuditShardWriteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AuditShardWriteError::Io { kind } => write!(f, "io: {kind:?}"),
        }
    }
}

impl std::error::Error for AuditShardWriteError {}

fn io_error(error: &io::Error) -> AuditShardWriteError {
    AuditShardWriteError::Io { kind: error.kind() }
}

/// 描き終えたブロック列をシャードへ追記する。
///
/// `blocks` が空なら**ファイルに触らない** — 書くものが無いのにシャードを生やさない。
///
/// # Errors
///
/// 親ディレクトリを作れない、シャードを開けない、書けない（`Io`）。
pub fn append(path: &Path, blocks: &str) -> Result<(), AuditShardWriteError> {
    if blocks.is_empty() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| io_error(&error))?;
    }
    let needs_header = match fs::metadata(path) {
        Ok(meta) => meta.len() == 0,
        Err(error) if error.kind() == io::ErrorKind::NotFound => true,
        Err(error) => return Err(io_error(&error)),
    };
    let mut file = core_infrastructure::append_only::open_append_only(path)
        .map_err(|error| io_error(&error))?;
    if needs_header {
        core_infrastructure::append_only::append_all(&mut file, SHARD_HEADER.as_bytes())
            .map_err(|error| io_error(&error))?;
    }
    core_infrastructure::append_only::append_all(&mut file, blocks.as_bytes())
        .map_err(|error| io_error(&error))
}

/// シャードディレクトリの全 `*.md` を**ファイル名順**に読み、`\n` で連結する。
///
/// これが [`find_all_events`] へ渡る連結バッファである。ファイル名順で連結するからこそ、
/// 同一秒のタイをバッファ位置で破るという規則が「シャード名順 × シャード内追記順」を意味する
/// （03 §6.3 / §6.4）。
///
/// **消えた・読めないシャードは黙って飛ばす**。読取中に台帳が育つのも失敗ではない — 生きている
/// 台帳を 1 つの欠落でまるごと読めなくしないためである（upstream 逐語:
/// *"growth during the read is explicitly not a failure"*）。ディレクトリ自体が無ければ空を返す。
///
/// [`find_all_events`]: core_command_domain::workspace::find_all_events
#[must_use]
pub fn read_all(dir: &Path) -> String {
    let Ok(entries) = fs::read_dir(dir) else {
        return String::new();
    };
    let mut shards: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect();
    shards.sort();
    shards
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const BLOCK: &str =
        "\n## Human Turn\n**Timestamp**: 2026-08-21T09:14:07Z\n**Event**: HUMAN_TURN\n\n---\n";

    #[test]
    fn the_first_write_to_a_missing_shard_lays_the_header_first() {
        let dir = tempdir().expect("一時 dir");
        let path = dir.path().join("audit/host-abcd1234.md");
        append(&path, BLOCK).expect("追記");
        assert_eq!(
            fs::read_to_string(&path).expect("読める"),
            format!("{SHARD_HEADER}{BLOCK}")
        );
    }

    #[test]
    fn an_empty_existing_shard_also_gets_the_header() {
        let dir = tempdir().expect("一時 dir");
        let path = dir.path().join("shard.md");
        fs::write(&path, b"").expect("空ファイルを置く");
        append(&path, BLOCK).expect("追記");
        assert_eq!(
            fs::read_to_string(&path).expect("読める"),
            format!("{SHARD_HEADER}{BLOCK}")
        );
    }

    #[test]
    fn a_later_write_appends_without_a_second_header() {
        let dir = tempdir().expect("一時 dir");
        let path = dir.path().join("shard.md");
        append(&path, BLOCK).expect("1 回目");
        append(&path, BLOCK).expect("2 回目");
        let content = fs::read_to_string(&path).expect("読める");
        assert_eq!(content, format!("{SHARD_HEADER}{BLOCK}{BLOCK}"));
        assert_eq!(
            content.matches(SHARD_HEADER).count(),
            1,
            "ヘッダは 1 本だけ"
        );
    }

    #[test]
    fn writing_nothing_does_not_create_the_shard() {
        let dir = tempdir().expect("一時 dir");
        let path = dir.path().join("shard.md");
        append(&path, "").expect("何も書かない");
        assert!(!path.exists(), "書くものが無いのにファイルを生やさない");
    }

    #[test]
    fn reading_all_shards_concatenates_them_in_file_name_order() {
        let dir = tempdir().expect("一時 dir");
        fs::write(dir.path().join("zeta-00000002.md"), "Z").expect("置く");
        fs::write(dir.path().join("alpha-00000001.md"), "A").expect("置く");
        // `.md` 以外は台帳ではない。
        fs::write(dir.path().join("notes.txt"), "X").expect("置く");
        assert_eq!(read_all(dir.path()), "A\nZ");
    }

    #[test]
    fn a_missing_shard_directory_reads_as_empty() {
        let dir = tempdir().expect("一時 dir");
        assert_eq!(read_all(&dir.path().join("no-such-dir")), "");
    }

    #[test]
    fn an_unreadable_shard_is_skipped_rather_than_failing_the_whole_read() {
        // ディレクトリを `.md` の位置に置くと `read_to_string` が失敗する（読めないシャード
        // の代役）。生きている台帳が 1 つの欠落で読めなくならないことを見る。
        let dir = tempdir().expect("一時 dir");
        fs::write(dir.path().join("a-00000001.md"), "A").expect("置く");
        fs::create_dir(dir.path().join("b-00000002.md")).expect("読めない項目を置く");
        fs::write(dir.path().join("c-00000003.md"), "C").expect("置く");
        assert_eq!(read_all(dir.path()), "A\nC");
    }

    #[test]
    fn a_shard_whose_metadata_cannot_be_read_fails_rather_than_guessing() {
        // 「ヘッダが要るか」はサイズで決める。サイズが読めないなら決められないので、
        // 勝手に「新規だからヘッダを書く」と推測せずに止める。
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().expect("一時 dir");
        let parent = dir.path().join("audit");
        fs::create_dir(&parent).expect("親を作る");
        let path = parent.join("host-abcd1234.md");
        fs::write(&path, b"x").expect("シャードを置く");

        let mut perms = fs::metadata(&parent).expect("親の情報").permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&parent, perms).expect("親を閉じる");

        let error = append(&path, BLOCK);

        // 後始末（TempDir の drop が親を消せるようにする）。
        let mut perms = fs::metadata(&parent).expect("親の情報").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&parent, perms).expect("親を開け直す");

        assert_eq!(
            error,
            Err(AuditShardWriteError::Io {
                kind: io::ErrorKind::PermissionDenied
            })
        );
    }

    #[test]
    fn a_failure_to_open_carries_its_kind() {
        let dir = tempdir().expect("一時 dir");
        // 親の位置に regular file がいるので mkdir -p が ENOTDIR で失敗する。
        let occupied = dir.path().join("occupied");
        fs::write(&occupied, b"x").expect("置く");
        let error = append(&occupied.join("child/shard.md"), BLOCK).expect_err("失敗する");
        assert!(
            matches!(error, AuditShardWriteError::Io { .. }),
            "実際: {error}"
        );
        assert!(error.to_string().starts_with("io: "), "実際: {error}");
    }
}
