//! プロセス生存判定 — `kill(pid, 0)` の ESRCH 判定 (reap 用, upstream `process.kill(pid, 0)`,
//! 03 §6.8)。`unsafe` を使わず `nix::sys::signal::kill` の safe wrapper 経由で行う
//! (`#![forbid(unsafe_code)]`)。

use nix::errno::Errno;
use nix::sys::signal::kill;
use nix::unistd::Pid;

/// プロセス生存判定。シグナル 0 相当 (`None`) を送り、`ESRCH` (対象不在) のみ `false` を返す。
/// 送信成功、および `EPERM` (対象は実在するが権限がない) は `true`。それ以外の errno は
/// 保守的に `true` (reap しない側) として扱う — 判定の誤りが「生きた保持者からの横取り」
/// (W2 違反) にならないようにするため。
#[must_use]
pub fn process_alive(pid: i32) -> bool {
    !matches!(kill(Pid::from_raw(pid), None), Err(Errno::ESRCH))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_current_process_is_alive() {
        assert!(process_alive(std::process::id() as i32));
    }

    #[test]
    fn an_exited_child_process_is_not_alive() {
        let mut child = std::process::Command::new("true").spawn().unwrap();
        let pid = child.id() as i32;
        let _ = child.wait().unwrap();
        assert!(!process_alive(pid));
    }
}
