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
///
/// **`pid <= 0` は問答無用で `false` (not alive)**。upstream `isPidAlive`
/// (`aidlc-lib.ts:6899` @ `3c3146cf`) の `if (!Number.isInteger(pid) || pid <= 0) return false;`
/// に対応する (Rust の `i32` では非整数は表現できないので `pid <= 0` の枝だけが残る)。
/// このガードは 2 つの理由で必須:
///
/// - **安全性**: `kill(0, sig)` は呼出プロセスの**プロセスグループ全体**、負値は
///   `pgid = -pid` のグループ宛の送信になる。シグナル 0 なので実害は無いが、
///   「その pid が生きているか」を問うたつもりが常に成功 = alive 誤判定になる。
/// - **可用性**: 破損スタンプ由来の pid `0` / 負値を alive と読むと、そのロック dir が
///   永久に奪えなくなる。upstream 逐語 (`:6894-6895`): *"A missing/invalid pid is treated as
///   'not alive' so an unstamped leaked dir/file is reclaimable on age alone."*
#[must_use]
pub fn process_alive(pid: i32) -> bool {
    if pid <= 0 {
        return false;
    }
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
    fn a_non_positive_pid_is_not_alive_and_never_probes_a_process_group() {
        // upstream isPidAlive (:6899) の `pid <= 0` ガード。`kill(0, 0)` は自プロセスグループ
        // 宛の送信になり必ず成功する (= alive 誤判定) ため、syscall へ渡す前に落とす。
        assert!(
            !process_alive(0),
            "pid 0 はプロセスグループ指定 — not alive"
        );
        assert!(!process_alive(-1), "負値は pgid 指定 — not alive");
        assert!(!process_alive(-(std::process::id() as i32)));
        assert!(!process_alive(i32::MIN));
    }

    #[test]
    fn an_exited_child_process_is_not_alive() {
        let mut child = std::process::Command::new("true").spawn().unwrap();
        let pid = child.id() as i32;
        let _ = child.wait().unwrap();
        assert!(!process_alive(pid));
    }
}
