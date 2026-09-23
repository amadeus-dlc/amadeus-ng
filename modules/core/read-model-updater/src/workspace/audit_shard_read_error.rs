//! 監査シャードの読取 — 台帳全体を 1 本の連結バッファにする（03 §6.3 / 11-workspace §4）。
//!
//! この読取は人間の返答ガード（承認・差し戻し・autonomous への昇格の
//! `humanActedSinceGate`）の材料になる。読めなかったシャードを「空だった」と読むと、
//! 人間の turn が在るのか無いのかを誰も読んでいない台帳から「人が居た」と答えてしまう
//! （fail-open）。したがって**消えたシャードだけを飛ばし、それ以外の読取失敗は失敗として
//! 返す**（upstream 2.8.2 `aidlc-lib.ts:7092-7110`）。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// 在るのに読めない監査シャード（材料だけを運ぶ — 文言はアダプタ層）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditShardReadError {
    /// 読取の I/O 失敗（不在以外）。
    Io {
        /// OS 由来の分類。
        kind: io::ErrorKind,
    },
    /// シャードの位置にシンボリックリンクが置かれている（upstream は辿らずに読取失敗とする）。
    Symlinked,
}

impl core::fmt::Display for AuditShardReadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AuditShardReadError::Io { kind } => write!(f, "io: {kind:?}"),
            AuditShardReadError::Symlinked => f.write_str("symlinked audit shard"),
        }
    }
}

impl std::error::Error for AuditShardReadError {}

/// シャードディレクトリの全 `*.md` を**ファイル名順**に読み、`\n` で連結する。
///
/// これが [`OrderedAuditEvents::find_in`] へ渡る連結バッファである。ファイル名順で連結するからこそ、
/// 同一秒のタイをバッファ位置で破るという規則が「シャード名順 × シャード内追記順」を意味する
/// （03 §6.3 / §6.4）。
///
/// - ディレクトリ自体が無い・列挙できないなら空を返す（upstream `auditShards` も列挙できない
///   場所を飛ばす）。
/// - 列挙と読取の間に**消えた**シャードは飛ばす。読取中に台帳が育つのも失敗ではない
///   （upstream 逐語: *"growth during the read is explicitly not a failure"*）。
/// - UTF-8 として不正なバイトは置換文字で読む（upstream の `toString("utf-8")` と同じ）。
///   黙って飛ばすと、そのシャードの `HUMAN_TURN` も解決行も無かったことになる。
///
/// # Errors
///
/// 在るシャードが読めない（`Io`）、またはシンボリックリンクである（`Symlinked`）。
///
/// [`OrderedAuditEvents::find_in`]: core_command_domain::workspace::OrderedAuditEvents::find_in
pub fn read_all(dir: &Path) -> Result<String, AuditShardReadError> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(String::new());
    };
    let mut shards: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect();
    shards.sort();
    let mut parts = Vec::new();
    for path in &shards {
        match fs::symlink_metadata(path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(AuditShardReadError::Symlinked);
            }
            Ok(_) => (),
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(AuditShardReadError::Io { kind: error.kind() }),
        }
        match fs::read(path) {
            Ok(bytes) => parts.push(String::from_utf8_lossy(&bytes).into_owned()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            Err(error) => return Err(AuditShardReadError::Io { kind: error.kind() }),
        }
    }
    Ok(parts.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn reading_all_shards_concatenates_them_in_file_name_order() {
        let dir = tempdir().expect("一時 dir");
        fs::write(dir.path().join("zeta-00000002.md"), "Z").expect("置く");
        fs::write(dir.path().join("alpha-00000001.md"), "A").expect("置く");
        // `.md` 以外は台帳ではない。
        fs::write(dir.path().join("notes.txt"), "X").expect("置く");
        assert_eq!(read_all(dir.path()), Ok("A\nZ".to_string()));
    }

    #[test]
    fn a_missing_shard_directory_reads_as_empty() {
        let dir = tempdir().expect("一時 dir");
        assert_eq!(read_all(&dir.path().join("no-such-dir")), Ok(String::new()));
    }

    #[test]
    fn an_unreadable_shard_fails_the_read_rather_than_reading_as_empty() {
        // ディレクトリを `.md` の位置に置くと読取が不在以外の理由で失敗する（読めない
        // シャードの代役）。読めなかったことを「空だった」に丸めない。
        let dir = tempdir().expect("一時 dir");
        fs::write(dir.path().join("a-00000001.md"), "A").expect("置く");
        fs::create_dir(dir.path().join("b-00000002.md")).expect("読めない項目を置く");
        fs::write(dir.path().join("c-00000003.md"), "C").expect("置く");
        assert!(matches!(
            read_all(dir.path()),
            Err(AuditShardReadError::Io { .. })
        ));
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_shard_is_not_followed() {
        let dir = tempdir().expect("一時 dir");
        let elsewhere = dir.path().join("elsewhere.txt");
        fs::write(&elsewhere, "X").expect("置く");
        std::os::unix::fs::symlink(&elsewhere, dir.path().join("a-00000001.md"))
            .expect("リンクを置く");
        assert_eq!(read_all(dir.path()), Err(AuditShardReadError::Symlinked));
    }

    #[test]
    fn invalid_utf8_is_read_with_replacement_rather_than_skipped() {
        let dir = tempdir().expect("一時 dir");
        fs::write(dir.path().join("a-00000001.md"), b"A\xffB").expect("置く");
        assert_eq!(read_all(dir.path()), Ok("A\u{fffd}B".to_string()));
    }
}
