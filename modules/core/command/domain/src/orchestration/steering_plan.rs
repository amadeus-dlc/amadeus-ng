//! `SteeringPlan` — 「何をどの順で届けるか」の配信計画 (02 §10)。
//!
//! 計画の**概念** (順序付きの部・部索引・配信済みパス台帳) だけをドメインが持つ。
//! Markdown の分割・輸送上限へのパックという**形式と輸送の知識**はアダプタ層
//! (`RuleBundleSource` 実装) が持ち、分割済みのチャンク列で本型を組む。

use super::directive::RuleContent;

/// 配信部の索引 (1 始まり)。
///
/// 算術は公開しない — 進めるのは `next()` だけ、範囲判定は [`SteeringPlan`] のクエリだけが
/// 行う (取り違え防止)。
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

/// パート総数。
///
/// [`PartIndex`] と隣接しても取り違えられないよう別型で運ぶ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartCount(u32);

impl PartCount {
    /// ワイヤ・表示用の生値。
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

/// 計画上の 1 部 — 索引・総数・その部が運ぶルール内容の借用。
///
/// [`SteeringPlan`] のクエリだけが構築する (`index <= of` は構築経路で保証される —
/// 範囲外の部は表現不能)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SteeringPart<'a> {
    index: PartIndex,
    of: PartCount,
    chunk: &'a [RuleContent],
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

/// 配信計画 — piece をパック済みのチャンク列 (アダプタ層が組む)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteeringPlan {
    chunks: Vec<Vec<RuleContent>>,
}

impl SteeringPlan {
    /// 分割済みチャンク列から計画を組む。空チャンクは部として意味を持たないので落とす。
    #[must_use]
    pub fn new(chunks: Vec<Vec<RuleContent>>) -> SteeringPlan {
        SteeringPlan {
            chunks: chunks.into_iter().filter(|c| !c.is_empty()).collect(),
        }
    }

    /// チャンク列 (読み順)。
    #[must_use]
    pub fn chunks(&self) -> &[Vec<RuleContent>] {
        &self.chunks
    }

    /// パート総数。
    #[must_use]
    pub fn part_count(&self) -> PartCount {
        PartCount(u32::try_from(self.chunks.len()).unwrap_or(u32::MAX))
    }

    /// 束が空か (bare run-stage でよい)。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// 第 1 部 (空計画なら `None`)。
    #[must_use]
    pub fn first_part(&self) -> Option<SteeringPart<'_>> {
        self.part_at(PartIndex::FIRST)
    }

    /// `delivered` まで配信済みのとき、次に届ける部 (もう無ければ `None`)。
    #[must_use]
    pub fn part_after(&self, delivered: PartIndex) -> Option<SteeringPart<'_>> {
        self.part_at(delivered.next())
    }

    /// `index` まで配信済みなら計画は完了 (終端 run-stage) か。
    #[must_use]
    pub fn is_delivered_through(&self, index: PartIndex) -> bool {
        index.as_u32() == self.part_count().as_u32()
    }

    /// 配信済みルールのパス台帳 (読み順・重複除去)。
    #[must_use]
    pub fn delivered_paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = Vec::new();
        for chunk in &self.chunks {
            for piece in chunk {
                if !paths.iter().any(|path| path == piece.path()) {
                    paths.push(piece.path().to_string());
                }
            }
        }
        paths
    }

    fn part_at(&self, index: PartIndex) -> Option<SteeringPart<'_>> {
        let position = usize::try_from(index.as_u32()).ok()?.checked_sub(1)?;
        self.chunks.get(position).map(|chunk| SteeringPart {
            index,
            of: self.part_count(),
            chunk,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(path: &str, text: &str) -> RuleContent {
        RuleContent::new(path.to_string(), text.to_string())
    }

    #[test]
    fn an_empty_plan_has_no_parts() {
        let plan = SteeringPlan::new(vec![]);
        assert!(plan.is_empty());
        assert_eq!(plan.part_count().as_u32(), 0);
        assert!(plan.first_part().is_none());
        assert!(plan.delivered_paths().is_empty());
    }

    #[test]
    fn empty_chunks_are_dropped_at_construction() {
        let plan = SteeringPlan::new(vec![vec![], vec![piece("a.md", "1")]]);
        assert_eq!(plan.part_count().as_u32(), 1);
    }

    #[test]
    fn parts_walk_in_order_until_delivered_through() {
        let plan = SteeringPlan::new(vec![vec![piece("a.md", "1")], vec![piece("b.md", "2")]]);
        let first = plan.first_part().unwrap();
        assert_eq!(first.index(), PartIndex::FIRST);
        assert_eq!(first.of().as_u32(), 2);
        assert_eq!(first.chunk().first().map(RuleContent::path), Some("a.md"));
        assert!(!plan.is_delivered_through(first.index()));
        let second = plan.part_after(first.index()).unwrap();
        assert_eq!(second.index().as_u32(), 2);
        assert!(plan.is_delivered_through(second.index()));
        assert!(plan.part_after(second.index()).is_none());
    }

    #[test]
    fn the_delivered_paths_ledger_deduplicates_in_reading_order() {
        let plan = SteeringPlan::new(vec![
            vec![piece("a.md", "1"), piece("a.md", "2")],
            vec![piece("b.md", "3")],
        ]);
        assert_eq!(plan.delivered_paths(), ["a.md", "b.md"]);
    }

    #[test]
    fn the_part_index_starts_at_one_and_rejects_zero() {
        assert_eq!(PartIndex::from_raw(0), None);
        assert_eq!(PartIndex::from_raw(2).map(PartIndex::as_u32), Some(2));
        assert_eq!(PartIndex::FIRST.next().as_u32(), 2);
    }
}
