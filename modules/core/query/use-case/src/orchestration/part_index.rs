//! `PartIndex` — 配信部の索引 (1 始まり)。
//!
//! 算術は公開しない — 進めるのは `next()` だけ。範囲判定はクエリ側では行わない (RMU が
//! 投影した行の写し [`SteeringPlanView`] を読むだけ — 取り違え防止)。
//!
//! [`SteeringPlanView`]: super::SteeringPlanView

/// 配信部の索引 (1 始まり)。
///
/// 算術は公開しない — 進めるのは `next()` だけ。範囲判定はクエリ側では行わない (RMU が
/// 投影した行の写し [`SteeringPlanView`] を読むだけ — 取り違え防止)。
///
/// [`SteeringPlanView`]: super::SteeringPlanView
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PartIndex(u32);

impl PartIndex {
    /// 第 1 部。
    pub const FIRST: PartIndex = PartIndex(1);

    /// 次の部。
    #[must_use]
    pub const fn next(self) -> PartIndex {
        PartIndex(self.0.saturating_add(1))
    }

    /// ワイヤ生値から復元する (0 は索引として不正 — 1 始まり)。
    #[must_use]
    pub const fn from_raw(raw: u32) -> Option<PartIndex> {
        if raw == 0 { None } else { Some(PartIndex(raw)) }
    }

    /// ワイヤ・表示用の生値。
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_part_index_starts_at_one_and_rejects_zero() {
        assert_eq!(PartIndex::from_raw(0), None);
        assert_eq!(PartIndex::from_raw(2).map(PartIndex::as_u32), Some(2));
        assert_eq!(PartIndex::FIRST.next().as_u32(), 2);
    }
}
