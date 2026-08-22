//! `Clock` / `ProcessProbe` のテスト用 fake 実装 — `fs_workspace_lock` の reap 判定
//! (dead owner / stale age) を実プロセスの生成や実時間の経過に頼らず決定的に検証するための
//! 注入点 (11-workspace §4「clock と process-probe は trait で注入可能に」)。

use core_use_case::workspace::clock::Clock;
use core_use_case::workspace::process_probe::ProcessProbe;
use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// 制御可能な偽時計。`Arc` で共有し、テストから `advance`/`set` で時刻を進める。
#[derive(Debug)]
pub struct FakeClock {
    now_ms: AtomicU64,
}

impl FakeClock {
    /// 初期時刻 (Unix epoch 起点のミリ秒) を指定して作る。以後この時計は
    /// `set` / `advance` でしか動かない。
    #[must_use]
    pub const fn new(now_ms: u64) -> FakeClock {
        FakeClock {
            now_ms: AtomicU64::new(now_ms),
        }
    }

    /// 時刻を絶対値で置く。巻き戻し (現在値より小さい値) も許す。
    pub fn set(&self, now_ms: u64) {
        self.now_ms.store(now_ms, Ordering::SeqCst);
    }

    /// 時刻を `delta_ms` だけ進める。stale 閾値の跨ぎをテストで作るための操作。
    pub fn advance(&self, delta_ms: u64) {
        self.now_ms.fetch_add(delta_ms, Ordering::SeqCst);
    }
}

impl Clock for FakeClock {
    fn now_ms(&self) -> u64 {
        self.now_ms.load(Ordering::SeqCst)
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

    /// 以後この pid を dead として報告する。
    ///
    /// # Panics
    ///
    /// 内部 `Mutex` が poison していた場合 (テスト用途につき想定外異常のみ)。
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
    fn fake_clock_advances_and_sets() {
        let clock = FakeClock::new(100);
        assert_eq!(clock.now_ms(), 100);
        clock.advance(50);
        assert_eq!(clock.now_ms(), 150);
        clock.set(0);
        assert_eq!(clock.now_ms(), 0);
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
