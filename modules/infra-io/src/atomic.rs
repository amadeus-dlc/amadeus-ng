//! アトミック書込プリミティブ — 同一ディレクトリの一時ファイルへ書いて rename する。
//! ポリシーを持たない低水準 I/O (W_OK バリア判断などの規律は Gateway 側 — 11-workspace §4)。
//! upstream `writeFileAtomic` 相当 (03 §5.6 / research workspace-state-intent §1.5)。

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// tmp+rename でアトミックにファイルを書き込む。一時ファイルは対象と同一ディレクトリに
/// `<basename>.tmp-<pid>-<nanos>` として作成し、write → flush → fsync → rename の順で行う。
/// クラッシュしても並行リーダに破断ファイルを見せない。
///
/// # Errors
///
/// 親ディレクトリの解決失敗・一時ファイルの作成/書込/fsync 失敗・rename 失敗を伝播する
/// (失敗時は一時ファイルの削除を試みる — ベストエフォート)。
pub fn write_file_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "path has no parent directory")
        })?;
    let tmp_path = tmp_path_in(parent, path)?;

    let write_result = write_and_sync(&tmp_path, bytes);
    if write_result.is_err() {
        let _ = fs::remove_file(&tmp_path);
        return write_result;
    }

    if let Err(e) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(e);
    }
    Ok(())
}

fn write_and_sync(tmp_path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(tmp_path)?;
    file.write_all(bytes)?;
    file.flush()?;
    file.sync_all()
}

fn tmp_path_in(parent: &Path, target: &Path) -> io::Result<PathBuf> {
    let file_name = target
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?
        .to_string_lossy()
        .into_owned();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    Ok(parent.join(format!("{file_name}.tmp-{}-{nanos}", std::process::id())))
}

/// `O_EXCL` 新規作成 (排他 open) — 既に存在すれば `AlreadyExists` で拒否する。
///
/// # Errors
///
/// 既存パスへの作成 (`ErrorKind::AlreadyExists`) やその他の I/O エラーを伝播する。
pub fn open_exclusive_new(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn write_file_atomic_creates_the_file_with_the_given_bytes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("out.txt");
        write_file_atomic(&path, b"hello").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"hello");
    }

    #[test]
    fn write_file_atomic_overwrites_existing_content_without_leaving_a_tmp_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("out.txt");
        write_file_atomic(&path, b"first").unwrap();
        write_file_atomic(&path, b"second").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second");
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn write_file_atomic_never_leaves_a_partially_named_target_missing_parent() {
        let dir = tempdir().unwrap();
        let missing_parent = dir.path().join("nested/does-not-exist/out.txt");
        assert!(write_file_atomic(&missing_parent, b"x").is_err());
    }

    #[test]
    fn write_file_atomic_rejects_a_path_without_a_parent_directory() {
        assert!(write_file_atomic(Path::new("no-parent.txt"), b"x").is_err());
    }

    #[test]
    fn write_file_atomic_cleans_up_the_tmp_file_when_rename_fails() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("occupied");
        fs::create_dir(&target).unwrap(); // rename (file → 既存ディレクトリ) は失敗する
        assert!(write_file_atomic(&target, b"x").is_err());
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(leftovers.is_empty(), "失敗時も一時ファイルを残さない");
    }

    #[test]
    fn open_exclusive_new_refuses_an_existing_path() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("exclusive.txt");
        open_exclusive_new(&path).unwrap();
        let err = open_exclusive_new(&path).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
    }
}
