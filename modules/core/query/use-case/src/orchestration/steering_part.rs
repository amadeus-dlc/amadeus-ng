//! `SteeringPart` — 配信計画上の 1 部 (索引・総数・その部が運ぶルール内容の借用)。
//!
//! [`SteeringPlan`] のクエリだけが構築する (`index <= of` は構築経路で保証される —
//! 範囲外の部は表現不能)。
//!
//! [`SteeringPlan`]: super::SteeringPlan

use super::part_count::PartCount;
use super::part_index::PartIndex;
use super::rule_content::RuleContent;

/// 計画上の 1 部 — 索引・総数・その部が運ぶルール内容の借用。
///
/// [`SteeringPlan`] のクエリだけが構築する (`index <= of` は構築経路で保証される —
/// 範囲外の部は表現不能)。
///
/// [`SteeringPlan`]: super::SteeringPlan
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SteeringPart<'a> {
    index: PartIndex,
    of: PartCount,
    chunk: &'a [RuleContent],
}

impl<'a> SteeringPart<'a> {
    /// 索引・総数・チャンクを束ねる。
    ///
    /// 範囲内であることを保証できるのは配信計画だけなので `pub(super)` に留める。
    #[must_use]
    pub(super) const fn new(
        index: PartIndex,
        of: PartCount,
        chunk: &'a [RuleContent],
    ) -> SteeringPart<'a> {
        SteeringPart { index, of, chunk }
    }
}

impl SteeringPart<'_> {
    /// この部の索引 (1 始まり)。
    #[must_use]
    pub const fn index(&self) -> PartIndex {
        self.index
    }

    /// パート総数。
    #[must_use]
    pub const fn of(&self) -> PartCount {
        self.of
    }

    /// この部が運ぶルール内容 (配列順に適用する)。
    #[must_use]
    pub const fn chunk(&self) -> &[RuleContent] {
        self.chunk
    }
}
