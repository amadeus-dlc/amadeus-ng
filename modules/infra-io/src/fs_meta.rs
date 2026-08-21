//! ファイルメタデータの低水準アクセサ — symlink 判定・記述子同一性再検証の材料 (dev/ino)・
//! 書込前 W_OK バリア検査 (upstream `accessSync(path, W_OK)`, 03 §5.6)。

use nix::sys::stat::{FileStat, lstat};
use nix::unistd::{AccessFlags, access};
use std::fs::File;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// パスが (シンボリックリンクを辿らずに) symlink かどうか。
///
/// # Errors
///
/// `lstat` 相当の I/O エラー (パス不在・権限エラー等) を伝播する。
pub fn is_symlink(path: &Path) -> io::Result<bool> {
    let stat: FileStat = lstat(path).map_err(errno_to_io)?;
    // S_IFMT でマスクした値が S_IFLNK と一致するかを見る (`libc::S_IFLNK`)。
    Ok((stat.st_mode & libc::S_IFMT) == libc::S_IFLNK)
}

/// open 済み記述子の `(dev, ino)` — 記述子同一性再検証の材料 (open 前後で一致すれば
/// rename/symlink 差し替えが起きていないと判定できる — upstream `verifyPathStillNamesDescriptor`,
/// 03 §6.7 ガード 5)。
///
/// # Errors
///
/// `fstat` 相当の I/O エラーを伝播する。
pub fn dev_ino(file: &File) -> io::Result<(u64, u64)> {
    let meta = file.metadata()?;
    Ok((meta.dev(), meta.ino()))
}

/// `access(path, W_OK)` 相当 — 書込前 W_OK バリア検査の材料 (upstream `accessSync(path, W_OK)`,
/// 03 §5.6)。read-only ターゲットへの意図的な書込バリアを実装するために使う (rename は
/// read-only ターゲットを貫通するため、事前の W_OK 検査がバリアの実体)。
///
/// # Errors
///
/// `EACCES`/`EROFS` (書込不可) は `Ok(false)` として扱う。それ以外の I/O エラーは伝播する。
pub fn is_writable(path: &Path) -> io::Result<bool> {
    match access(path, AccessFlags::W_OK) {
        Ok(()) => Ok(true),
        Err(nix::errno::Errno::EACCES | nix::errno::Errno::EROFS) => Ok(false),
        Err(e) => Err(errno_to_io(e)),
    }
}

fn errno_to_io(e: nix::errno::Errno) -> io::Error {
    io::Error::from_raw_os_error(e as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    #[test]
    fn is_symlink_distinguishes_links_from_regular_files() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("real.md");
        std::fs::write(&target, b"x").unwrap();
        let link = dir.path().join("link.md");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(!is_symlink(&target).unwrap());
        assert!(is_symlink(&link).unwrap());
    }

    #[test]
    fn dev_ino_is_stable_across_reopens_of_the_same_path() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("f.md");
        std::fs::write(&path, b"x").unwrap();
        let a = dev_ino(&File::open(&path).unwrap()).unwrap();
        let b = dev_ino(&File::open(&path).unwrap()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn is_writable_reports_false_for_read_only_files() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ro.md");
        std::fs::write(&path, b"x").unwrap();
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o444);
        std::fs::set_permissions(&path, perms).unwrap();
        assert!(!is_writable(&path).unwrap());
        // 後始末: tempdir 削除のために書込可能へ戻す。
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o644);
        std::fs::set_permissions(&path, perms).unwrap();
    }

    #[test]
    fn is_writable_propagates_errors_for_a_missing_path() {
        let dir = tempdir().unwrap();
        assert!(is_writable(&dir.path().join("missing")).is_err());
    }

    #[test]
    fn is_writable_reports_true_for_writable_files() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rw.md");
        std::fs::write(&path, b"x").unwrap();
        assert!(is_writable(&path).unwrap());
    }
}
