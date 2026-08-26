//! 時計 — **横断機構の注入シームであって Gateway ではない** (clean-architecture: 時計は
//! Infrastructure が所有する機構。aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md)。
//!
//! どのユースケースもこの trait を消費しない。存在理由は、時刻に依存する Gateway の挙動
//! (イベント記録時刻の押印など) を、実時間の経過に頼らず決定的に検証できるようにすること
//! だけである。したがってアプリ境界のポートとして use-case 層には置かず、実装と同じ
//! アダプタ層に閉じ込める。
//!
//! 単位は `chrono::DateTime<Utc>` — ドメインイベントの `occurred_at` と集約の
//! `last_updated_at` が本家 event-store-adapter-rs の契約でこの型だからである (ADR-010)。
//! 自前の epoch ミリ秒と ISO 8601 整形はここで役目を終えた (NFR4.1 の再検討)。

use std::cell::Cell;

use chrono::{DateTime, TimeDelta, Utc};

/// 現在時刻の抽象。テストで fake を注入するための唯一の時刻源。
pub trait Clock {
    /// 現在の UTC 時刻。記録時刻の押印と経過時間の算出はこの値で行う。
    #[must_use]
    fn now(&self) -> DateTime<Utc>;
}

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

/// 制御可能な偽時計。テストから `advance` / `set` で時刻を進める。
///
/// `Cell` を `&self` の裏に置くのは [interior-mutability] の既定 (内部可変性は禁止) に
/// 対する例外である。理由は [`Clock::now`] が `&self` であり、注入した時計を握ったまま
/// 進める操作をテストから呼べる必要があること、そしてこの型が**テスト専用の実装に
/// 閉じている**ことである。ロックではなく `Cell` を選ぶのは、施錠の失敗という
/// panic 経路を作らないためである (NFR4.3)。
///
/// [interior-mutability]: https://github.com/amadeus-dlc/amadeus-ng/blob/main/aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/interior-mutability.md
#[derive(Debug)]
pub struct FakeClock {
    now: Cell<DateTime<Utc>>,
}

impl FakeClock {
    /// 初期時刻を指定して作る。以後この時計は `set` / `advance` でしか動かない。
    #[must_use]
    pub const fn new(now: DateTime<Utc>) -> FakeClock {
        FakeClock {
            now: Cell::new(now),
        }
    }

    /// 時刻を絶対値で置く。巻き戻し (現在値より前の時刻) も許す。
    pub fn set(&self, now: DateTime<Utc>) {
        self.now.set(now);
    }

    /// 時刻を `delta` だけ進める。時刻に依存する分岐をテストで作るための操作。
    pub fn advance(&self, delta: TimeDelta) {
        self.now.set(self.now.get() + delta);
    }
}

impl Clock for FakeClock {
    fn now(&self) -> DateTime<Utc> {
        self.now.get()
    }
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn fake_clock_advances_and_sets() {
        let clock = FakeClock::new(epoch(100));
        assert_eq!(clock.now(), epoch(100));
        clock.advance(TimeDelta::seconds(50));
        assert_eq!(clock.now(), epoch(150));
        clock.set(DateTime::UNIX_EPOCH);
        assert_eq!(clock.now(), DateTime::UNIX_EPOCH);
    }

    #[test]
    fn fake_clock_accepts_a_backward_step() {
        // 巻き戻しは許す — 時刻に依存する分岐をテストで作るための操作だからである。
        let clock = FakeClock::new(epoch(100));
        clock.advance(TimeDelta::seconds(-40));
        assert_eq!(clock.now(), epoch(60));
    }
}
