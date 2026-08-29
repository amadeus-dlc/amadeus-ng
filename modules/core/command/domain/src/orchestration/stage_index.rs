//! `StageIndex` — 集約が所有するステージ位置 (BR5.1)。

use std::fmt;

/// 文書順のステージ位置。
///
/// **構築できるのは集約だけ** (`IntentExecution::stage_index(usize) -> Option<StageIndex>`)
/// で、その集約の `stage_count` 未満であることが構築時に保証される。生の `usize` を集約 API・
/// イベント・`NextDecision`・状態の写し (memento) に露出させないための E1 型であり、範囲外は
/// `None` で表して panic しない (BR5.1)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StageIndex(usize);

impl StageIndex {
    /// 集約 (と同一クレート内の再水和経路) だけが使う構築子。
    ///
    /// 範囲の保証は呼出側の責務であり、公開経路は `IntentExecution::stage_index` と
    /// `IntentExecution::from_snapshot` の検証を必ず通る。
    pub(crate) const fn new(value: usize) -> StageIndex {
        StageIndex(value)
    }

    /// 文書順の位置 (0 始まり)。
    #[must_use]
    pub const fn to_usize(self) -> usize {
        self.0
    }
}

impl fmt::Display for StageIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn the_index_carries_its_position() {
        assert_eq!(StageIndex::new(0).to_usize(), 0);
        assert_eq!(StageIndex::new(7).to_usize(), 7);
    }

    #[test]
    fn ordering_follows_the_position() {
        let mut sorted = [StageIndex::new(3), StageIndex::new(1), StageIndex::new(2)];
        sorted.sort();
        let raw: Vec<usize> = sorted.iter().map(|s| s.to_usize()).collect();
        assert_eq!(raw, [1, 2, 3]);
    }

    #[test]
    fn equality_is_the_position() {
        assert_eq!(StageIndex::new(4), StageIndex::new(4));
        assert_ne!(StageIndex::new(4), StageIndex::new(5));
    }

    #[test]
    fn display_writes_the_bare_position() {
        assert_eq!(StageIndex::new(12).to_string(), "12");
    }

    #[test]
    fn the_index_works_as_a_set_key() {
        let set: BTreeSet<StageIndex> =
            [StageIndex::new(1), StageIndex::new(1), StageIndex::new(2)]
                .into_iter()
                .collect();
        assert_eq!(set.len(), 2);
    }
}
