//! `SystemClock` — [`Clock`](crate::Clock) の壁時計実装。

use chrono::{DateTime, Utc};

use crate::clock::Clock;

/// `Utc::now()` に基づく実時計。
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
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;

    use super::*;

    fn epoch(seconds: i64) -> DateTime<Utc> {
        DateTime::UNIX_EPOCH + TimeDelta::seconds(seconds)
    }

    #[test]
    fn system_clock_reports_a_wall_clock_time() {
        // 単調性は壁時計が保証しない (時刻調整で後退しうる) ため主張しない。
        // 2020-01-01 以降であることだけを検査する。
        assert!(SystemClock::new().now() > epoch(1_577_836_800));
    }
}
