//! 開いたファイルに対する、解放漏れのない排他ロック。
use std::fs::File;
use std::time::Duration;
/// 保持している間だけOSの排他ロックを所有する。プロトコルやパス政策は持たない。
#[derive(Debug)]
pub struct ExclusiveFileLock {
    _file: File,
}
impl ExclusiveFileLock {
    /// 開いたファイルの排他ロックを、指定時間まで待って取得する。
    /// # Errors
    /// 待ち時間内に取れない場合はWouldBlock、その他はOSの失敗。
    pub fn acquire(file: File, wait: Duration) -> Result<Self, std::io::Error> {
        let started = std::time::Instant::now();
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(Self { _file: file }),
                Err(std::fs::TryLockError::Error(error)) => return Err(error),
                Err(std::fs::TryLockError::WouldBlock) => {
                    let elapsed = started.elapsed();
                    if elapsed >= wait {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::WouldBlock,
                            "exclusive file lock timed out",
                        ));
                    }
                    std::thread::sleep((wait - elapsed).min(Duration::from_millis(10)));
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_second_handle_is_blocked_until_the_owner_is_dropped() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let first = ExclusiveFileLock::acquire(file.reopen().unwrap(), Duration::ZERO).unwrap();
        let error = ExclusiveFileLock::acquire(file.reopen().unwrap(), Duration::ZERO)
            .expect_err("同時には取得できない");
        assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock);
        drop(first);
        ExclusiveFileLock::acquire(file.reopen().unwrap(), Duration::ZERO).unwrap();
    }
}
