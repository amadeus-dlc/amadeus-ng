//! 追記専用 open — `O_RDWR|O_APPEND|O_CREAT|O_NOFOLLOW|O_NONBLOCK` mode 0o666 (upstream
//! `appendAuditBlockAtPath`, 03 §6.7)。symlink は `O_NOFOLLOW` で拒否し、fstat で regular
//! file を検査する。**nlink != 1 は拒否しない** — rsync `--link-dest` / `cp -al` スナップショット
//! を拒否すると以後の全 gate/hook 追記が文鎮化した upstream の実績による非対称 (11-workspace §4)。

use libc::{O_APPEND, O_CREAT, O_NOFOLLOW, O_NONBLOCK};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

/// 追記専用 open。存在しなければ mode 0o666 で新規作成する。symlink は `O_NOFOLLOW` により
/// open 自体が `ELOOP` で失敗する。open 成功後は fstat で regular file を検査し、非 regular
/// (FIFO・デバイス等) は Err で拒否する。**多重リンク (`nlink != 1`) は拒否しない** — 意図的
/// 非対称 (11-workspace §4)。
///
/// # Errors
///
/// open 自体の I/O エラー (symlink・権限エラー等)、および regular file でなかった場合の
/// `InvalidInput` を返す。
pub fn open_append_only(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(O_APPEND | O_CREAT | O_NOFOLLOW | O_NONBLOCK)
        .mode(0o666)
        .open(path)?;

    let meta = file.metadata()?;
    if !meta.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("refusing non-regular file: {}", path.display()),
        ));
    }
    Ok(file)
}

/// 部分書込ループ付き追記。0 バイト進捗を Err で検出する
/// (upstream `writeAll` — "Audit append made no write progress" 相当, 03 §6.7 ガード 6)。
///
/// # Errors
///
/// 書込 I/O エラー、または 0 バイト進捗が観測された場合の `WriteZero` エラーを返す。
pub fn append_all(file: &mut File, bytes: &[u8]) -> io::Result<()> {
    let mut written = 0usize;
    while written < bytes.len() {
        // ループ不変条件 (`written < bytes.len()`) により範囲外にはならない — else 分岐には
        // 到達しない。
        let Some(remaining) = bytes.get(written..) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "append_all: written index out of bounds",
            ));
        };
        match file.write(remaining) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "append made no write progress",
                ));
            }
            Ok(n) => written += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::os::unix::fs::symlink;
    use tempfile::tempdir;

    #[test]
    fn open_append_only_creates_a_regular_file_when_absent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("shard.md");
        let mut file = open_append_only(&path).unwrap();
        append_all(&mut file, b"hello").unwrap();
        drop(file);
        let mut content = String::new();
        File::open(&path)
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert_eq!(content, "hello");
    }

    #[test]
    fn open_append_only_appends_across_multiple_opens() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("shard.md");
        {
            let mut file = open_append_only(&path).unwrap();
            append_all(&mut file, b"first;").unwrap();
        }
        {
            let mut file = open_append_only(&path).unwrap();
            append_all(&mut file, b"second;").unwrap();
        }
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "first;second;");
    }

    #[test]
    fn open_append_only_refuses_symlinks() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("real.md");
        std::fs::write(&target, b"x").unwrap();
        let link = dir.path().join("link.md");
        symlink(&target, &link).unwrap();
        assert!(open_append_only(&link).is_err());
    }

    #[test]
    fn open_append_only_refuses_a_fifo() {
        use nix::sys::stat::Mode;
        let dir = tempdir().unwrap();
        let path = dir.path().join("fifo");
        nix::unistd::mkfifo(&path, Mode::from_bits_truncate(0o644)).unwrap();
        // O_NONBLOCK により open はブロックせず成功し、fstat の regular file 検査で拒否される
        assert!(open_append_only(&path).is_err());
    }

    #[test]
    fn append_all_writes_every_byte() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("shard.md");
        let mut file = open_append_only(&path).unwrap();
        let payload = vec![b'x'; 8192];
        append_all(&mut file, &payload).unwrap();
        drop(file);
        assert_eq!(std::fs::metadata(&path).unwrap().len(), 8192);
    }
}
