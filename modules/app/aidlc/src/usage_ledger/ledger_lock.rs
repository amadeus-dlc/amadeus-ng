//! 台帳の read-modify-write を直列化するプロセス間ロック。
//!
//! 本家 `withUsageLedgerLock` と同じく workspace の外（OS の一時領域）に置き、`aidlc/`
//! 配下へ余分な生成物を残さない。解放時にロックファイルも消すので一時領域に溜まらない。
//! 消したファイルを待っていた側が古い inode を掴んでしまわないよう、取得後にパスの
//! inode が自分の開いたものと同じであることを確かめ、違えば開き直す。
use core_infrastructure::ExclusiveFileLock;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// 保持している間だけ台帳を占有する。
#[derive(Debug)]
pub(crate) struct LedgerLock {
    lock: ExclusiveFileLock,
    path: PathBuf,
}

impl LedgerLock {
    /// workspace ごとのロックを、`wait` まで待って取る。
    ///
    /// # Errors
    /// ロックファイルを開けない、または `wait` 内に取れない（`WouldBlock`）。
    pub(crate) fn acquire(project: &Path, wait: Duration) -> std::io::Result<Self> {
        let path = lock_path(project);
        let deadline = std::time::Instant::now() + wait;
        loop {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&path)?;
            let opened = inode(&file.metadata()?);
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            let lock = ExclusiveFileLock::acquire(file, remaining)?;
            // 待っている間に先行者がファイルを消して別の inode になっていたら開き直す。
            if std::fs::metadata(&path).is_ok_and(|current| inode(&current) == opened) {
                return Ok(Self { lock, path });
            }
            drop(lock);
            if std::time::Instant::now() >= deadline {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WouldBlock,
                    "usage ledger lock was replaced while waiting",
                ));
            }
        }
    }
}

/// ファイルの同一性。unix では inode、それ以外では区別しない（常に同じと見なす）。
#[cfg(unix)]
fn inode(metadata: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt as _;
    metadata.ino()
}

#[cfg(not(unix))]
const fn inode(_metadata: &std::fs::Metadata) -> u64 {
    0
}

impl Drop for LedgerLock {
    fn drop(&mut self) {
        // ロックを握ったまま消す（`lock` はこの後に落ちる）。
        let _ = std::fs::remove_file(&self.path);
        let _ = &self.lock;
    }
}

/// workspace の絶対パスから決まる一時領域のロックファイル。
fn lock_path(project: &Path) -> PathBuf {
    let digest = core_infrastructure::hash::sha256_hex(project.to_string_lossy().as_bytes());
    std::env::temp_dir().join(format!(
        ".aidlc-usage-{}.lock",
        digest.get(..8).unwrap_or_default()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lock_file_exists_while_held_and_is_removed_on_release() {
        let project = tempfile::tempdir().expect("一時ディレクトリ");
        let path = lock_path(project.path());
        let held = LedgerLock::acquire(project.path(), Duration::from_secs(1)).expect("取れる");
        assert!(path.exists(), "保持中はファイルがある");
        drop(held);
        assert!(!path.exists(), "解放でファイルも消える");
    }

    #[test]
    fn a_second_acquirer_waits_for_the_first_and_then_succeeds() {
        let project = tempfile::tempdir().expect("一時ディレクトリ");
        let first = LedgerLock::acquire(project.path(), Duration::from_secs(1)).expect("取れる");
        let root = project.path().to_path_buf();
        let started = std::time::Instant::now();
        let waiter = std::thread::spawn(move || {
            LedgerLock::acquire(&root, Duration::from_secs(5)).map(|lock| {
                drop(lock);
                started.elapsed()
            })
        });
        std::thread::sleep(Duration::from_millis(150));
        drop(first);
        let waited = waiter.join().expect("スレッド").expect("待った後に取れる");
        assert!(
            waited >= Duration::from_millis(150),
            "先行者の解放を待つ: {waited:?}"
        );
        assert!(!lock_path(project.path()).exists());
    }

    #[test]
    fn a_held_lock_times_out_the_impatient() {
        let project = tempfile::tempdir().expect("一時ディレクトリ");
        let _first = LedgerLock::acquire(project.path(), Duration::from_secs(1)).expect("取れる");
        let error =
            LedgerLock::acquire(project.path(), Duration::ZERO).expect_err("同時には取れない");
        assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock);
    }
}
