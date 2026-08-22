//! 時計 — **横断機構の注入シームであって Gateway ではない** (clean-architecture: 時計は
//! Infrastructure が所有する機構。docs/memory/gateway-taxonomy.md)。
//!
//! どのユースケースもこの trait を消費しない。存在理由は `FsWorkspaceLock` の stale 判定と
//! owner stamp 押印を、実時間の経過に頼らず決定的に検証できるようにすることだけである
//! (11-workspace §4「clock と process-probe は trait で注入可能に」)。したがってアプリ境界の
//! ポートとして use-case 層には置かず、実装と同じアダプタ層に閉じ込める。

use std::sync::atomic::{AtomicU64, Ordering};

/// 現在時刻の抽象 (ミリ秒, Unix epoch 起点)。テストで fake を注入するための唯一の時刻源。
pub trait Clock {
    /// Unix epoch 起点の経過ミリ秒。stale 判定 (年齢 = この値 − owner stamp) と
    /// owner stamp の押印はどちらもこの単位で行う。
    #[must_use]
    fn now_ms(&self) -> u64;
}

/// `SystemTime::now()` に基づく実時計。
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl SystemClock {
    /// 単位型を作る (状態を持たないので設定項目は無い)。
    #[must_use]
    pub const fn new() -> SystemClock {
        SystemClock
    }
}

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
            .unwrap_or(0)
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_ms_is_monotonically_non_decreasing_across_two_reads() {
        let clock = SystemClock::new();
        let a = clock.now_ms();
        let b = clock.now_ms();
        assert!(b >= a);
    }

    #[test]
    fn fake_clock_advances_and_sets() {
        let clock = FakeClock::new(100);
        assert_eq!(clock.now_ms(), 100);
        clock.advance(50);
        assert_eq!(clock.now_ms(), 150);
        clock.set(0);
        assert_eq!(clock.now_ms(), 0);
    }
}
