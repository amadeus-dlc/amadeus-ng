//! `GlobalSeqNr` — 全集約横断のジャーナル通番 (entities.md GlobalSeqNr、C6 journal.global_seq_nr)。

use std::fmt;

/// 全集約横断のジャーナル通番 (C6 `journal.global_seq_nr` の値)。
///
/// 投影 (U4) のチェックポイントはこの単位で進む。`0` は「まだ何も読んでいない」を表す
/// 番兵で ([`GlobalSeqNr::ZERO`])、実在するジャーナル行は 1 以上である
/// (C6 の `INTEGER PRIMARY KEY AUTOINCREMENT` が 1 から採番するため)。
///
/// 順序は素の通番の大小そのもの — 差分読取 (`events_after`) とチェックポイントの単調性が
/// この順序に乗る。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlobalSeqNr(u64);

impl GlobalSeqNr {
    /// 「まだ何も読んでいない」位置。未登録の投影のチェックポイントはこの値になる (BR1.4)。
    pub const ZERO: GlobalSeqNr = GlobalSeqNr(0);

    /// 通番を包む。ジャーナル行の採番はストアの責務なので、ここでは値域を狭めない。
    #[must_use]
    pub const fn new(value: u64) -> GlobalSeqNr {
        GlobalSeqNr(value)
    }

    /// 生の通番。
    #[must_use]
    pub const fn to_u64(self) -> u64 {
        self.0
    }
}

impl From<u64> for GlobalSeqNr {
    fn from(value: u64) -> GlobalSeqNr {
        GlobalSeqNr(value)
    }
}

impl fmt::Display for GlobalSeqNr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_the_value_that_means_nothing_has_been_read_yet() {
        assert_eq!(GlobalSeqNr::ZERO.to_u64(), 0);
        assert_eq!(GlobalSeqNr::ZERO, GlobalSeqNr::new(0));
    }

    #[test]
    fn a_counter_value_round_trips_through_the_newtype() {
        assert_eq!(GlobalSeqNr::new(7).to_u64(), 7);
        assert_eq!(GlobalSeqNr::from(7_u64).to_u64(), 7);
    }

    #[test]
    fn the_order_follows_the_underlying_counter() {
        assert!(GlobalSeqNr::ZERO < GlobalSeqNr::new(1));
        assert!(GlobalSeqNr::new(2) > GlobalSeqNr::new(1));
        let mut sorted = vec![GlobalSeqNr::new(3), GlobalSeqNr::ZERO, GlobalSeqNr::new(1)];
        sorted.sort();
        assert_eq!(
            sorted,
            [GlobalSeqNr::ZERO, GlobalSeqNr::new(1), GlobalSeqNr::new(3)]
        );
    }

    #[test]
    fn the_display_is_the_bare_counter() {
        assert_eq!(GlobalSeqNr::new(42).to_string(), "42");
        assert_eq!(GlobalSeqNr::ZERO.to_string(), "0");
    }

    #[test]
    fn values_compare_by_value() {
        assert_eq!(GlobalSeqNr::new(5), GlobalSeqNr::new(5));
        assert_ne!(GlobalSeqNr::new(5), GlobalSeqNr::new(6));
    }

    #[test]
    fn the_counter_saturates_at_the_u64_ceiling_instead_of_wrapping() {
        assert_eq!(GlobalSeqNr::new(u64::MAX).to_u64(), u64::MAX);
        assert!(GlobalSeqNr::new(u64::MAX) > GlobalSeqNr::new(u64::MAX - 1));
    }
}
