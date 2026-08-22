//! プロセス生存判定 — **横断機構の注入シームであって Gateway ではない**
//! (docs/memory/gateway-taxonomy.md。`kill(pid,0)` は永続化でも外部システム呼出でもなく
//! ランタイム機構であり、clean-architecture では Infrastructure に属する)。
//!
//! どのユースケースもこの trait を消費しない。存在理由は `FsWorkspaceLock` の reap 判定
//! (W2: 生きた保持者から奪わない) を、実プロセスの生成に頼らず決定的に検証できるように
//! することだけである (11-workspace §4)。

use std::collections::HashSet;
use std::sync::Mutex;

/// プロセス生存判定の抽象。ロックの reap 判定 (W2: 生きた保持者から奪わない) の材料。
pub trait ProcessProbe {
    /// `pid` が生存しているか。`ESRCH` (対象不在) のときだけ偽で、`EPERM` やその他の
    /// errno は保守的に真 — 判定の誤りが W2 (生きた保持者から奪わない) 違反にならない側へ倒す。
    #[must_use]
    fn is_alive(&self, pid: i32) -> bool;
}

/// OS のプロセステーブルに対する実判定 — infra-io `process_probe::process_alive` に委譲する。
#[derive(Debug, Clone, Copy, Default)]
pub struct OsProcessProbe;

impl OsProcessProbe {
    /// 単位型を作る (状態を持たないので設定項目は無い)。
    #[must_use]
    pub const fn new() -> OsProcessProbe {
        OsProcessProbe
    }
}

impl ProcessProbe for OsProcessProbe {
    fn is_alive(&self, pid: i32) -> bool {
        infra_io::process_probe::process_alive(pid)
    }
}

/// 制御可能な偽プロセス判定。既定はすべて alive、`mark_dead` した pid のみ dead を返す。
#[derive(Debug, Default)]
pub struct FakeProcessProbe {
    dead_pids: Mutex<HashSet<i32>>,
}

impl FakeProcessProbe {
    /// dead 指定が 1 つも無い状態で作る (どの pid も alive と報告する)。
    #[must_use]
    pub fn new() -> FakeProcessProbe {
        FakeProcessProbe::default()
    }

    /// 以後この pid を dead として報告する (poison した `Mutex` は `into_inner` で回復
    /// するため panic しない)。
    pub fn mark_dead(&self, pid: i32) {
        let mut guard = self.dead_pids.lock().unwrap_or_else(|e| e.into_inner());
        guard.insert(pid);
    }
}

impl ProcessProbe for FakeProcessProbe {
    fn is_alive(&self, pid: i32) -> bool {
        let guard = self
            .dead_pids
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        !guard.contains(&pid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_current_process_is_alive() {
        let probe = OsProcessProbe::new();
        assert!(probe.is_alive(std::process::id() as i32));
    }

    #[test]
    fn fake_process_probe_defaults_to_alive_until_marked_dead() {
        let probe = FakeProcessProbe::new();
        assert!(probe.is_alive(4242));
        probe.mark_dead(4242);
        assert!(!probe.is_alive(4242));
        assert!(probe.is_alive(1));
    }
}
