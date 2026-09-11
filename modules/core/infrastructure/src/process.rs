//! 親PIDと、signal 0によるプロセス存在確認。ワークフロー語彙を持たない。
/// 現在のプロセスの親PID。対応外環境では不明。
#[must_use]
pub fn parent_id() -> Option<u32> {
    #[cfg(unix)]
    {
        u32::try_from(nix::unistd::getppid().as_raw()).ok()
    }
    #[cfg(not(unix))]
    {
        None
    }
}
/// signalを送らず存在を確認する。EPERMは存在するプロセスとして扱う。
#[must_use]
pub fn is_alive(pid: u32) -> bool {
    if pid <= 1 {
        return false;
    }
    #[cfg(unix)]
    {
        let Ok(pid) = i32::try_from(pid) else {
            return false;
        };
        matches!(
            nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None),
            Ok(()) | Err(nix::errno::Errno::EPERM)
        )
    }
    #[cfg(not(unix))]
    {
        false
    }
}
#[cfg(all(test, unix))]
mod tests {
    #[test]
    fn the_current_process_and_its_parent_are_alive_without_signalling_them() {
        assert!(super::is_alive(std::process::id()));
        assert!(super::parent_id().is_some_and(super::is_alive));
    }
    #[test]
    fn invalid_process_identifiers_are_refused() {
        assert!(!super::is_alive(0));
        assert!(!super::is_alive(1));
        assert!(!super::is_alive(u32::MAX));
    }
}
