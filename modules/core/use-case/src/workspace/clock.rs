//! `Clock` ポート — 11-workspace §3 に列挙された供給ポートの 1 つ。ロックの stale 判定・
//! owner stamp の時刻材料。実装は `core-interface-adapter`
//! (`SystemClock` が実時計、`testing::FakeClock` がテスト用の注入可能な偽時計)。

/// 現在時刻の抽象 (ミリ秒, Unix epoch 起点)。テストで fake を注入するための唯一の時刻源。
pub trait Clock {
    /// Unix epoch 起点の経過ミリ秒。stale 判定 (年齢 = この値 − owner stamp) と
    /// owner stamp の押印はどちらもこの単位で行う。
    #[must_use]
    fn now_ms(&self) -> u64;
}
