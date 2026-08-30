//! `SnapshotStrategy` — いつスナップショット (ある時点の集約) を書き直すかの設定。
//!
//! Repository 実装の**内部設定**であり、ポート面には現れない — 呼出側は `store` に任せる
//! だけである (オーナー裁定 2026-08-30、本家 example `user_account_repository.rs` の形)。
//! 初回 (genesis) は本ストラテジに関係なく必ずスナップショットを書く (本家 v3 の新規作成
//! 規約 — これを外すとリプレイの基底が無くなる)。

use std::num::NonZeroUsize;

/// N イベントごとにスナップショットを書き直す設定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotStrategy {
    interval: NonZeroUsize,
}

impl SnapshotStrategy {
    /// `interval` イベントごとに書き直す。
    #[must_use]
    pub const fn every(interval: NonZeroUsize) -> SnapshotStrategy {
        SnapshotStrategy { interval }
    }

    /// この通番でスナップショットを書き直すか (genesis の必須スナップショットは呼出側の分岐)。
    #[must_use]
    pub const fn wants_snapshot(&self, seq_nr: usize) -> bool {
        seq_nr.is_multiple_of(self.interval.get())
    }
}

impl Default for SnapshotStrategy {
    /// 本家 example と同じ 10 イベントごと。実際の値は合成ルート (U7) が確定する。
    fn default() -> SnapshotStrategy {
        SnapshotStrategy::every(NonZeroUsize::new(10).unwrap_or(NonZeroUsize::MIN))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_wants_a_snapshot_every_ten_events() {
        let strategy = SnapshotStrategy::default();
        assert!(!strategy.wants_snapshot(1));
        assert!(!strategy.wants_snapshot(9));
        assert!(strategy.wants_snapshot(10));
        assert!(!strategy.wants_snapshot(11));
        assert!(strategy.wants_snapshot(20));
    }

    #[test]
    fn a_custom_interval_is_respected() {
        let strategy = SnapshotStrategy::every(NonZeroUsize::new(3).unwrap());
        assert!(!strategy.wants_snapshot(2));
        assert!(strategy.wants_snapshot(3));
        assert!(strategy.wants_snapshot(6));
    }
}
