//! `PartCount` — 配信計画のパート総数。
//!
//! [`PartIndex`] と隣接しても取り違えられないよう別型で運ぶ (同型プリミティブの隣接は
//! 取り違えの温床)。
//!
//! [`PartIndex`]: super::PartIndex

/// パート総数。
///
/// [`PartIndex`] と隣接しても取り違えられないよう別型で運ぶ。
///
/// [`PartIndex`]: super::PartIndex
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartCount(u32);

impl PartCount {
    /// 数え上げ済みの総数を包む。
    ///
    /// 数えるのは RMU (`read_steering_plan.part_count`) であり、ここへ来るのはその行の値
    /// である — 数え直さない。
    #[must_use]
    pub const fn new(count: u32) -> PartCount {
        PartCount(count)
    }

    /// ワイヤ・表示用の生値。
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}
