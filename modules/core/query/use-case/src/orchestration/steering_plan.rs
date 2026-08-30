//! `SteeringPlan` — 「何をどの順で届けるか」の配信計画 (02 §10)。
//!
//! 計画の概念 (順序付きの部・部索引・配信済みパス台帳) に加えて、**分割とパック**
//! (Markdown 見出し境界・過大セクションのコードポイント分割・輸送目標へのパック — 02 §10)
//! も本型のファクトリ [`SteeringPlan::pack`] が持つ。CPU とメモリだけの計算であり、構築規則は
//! 型が所有する (`coding-rules/domain-services.md` / `factory-naming.md`)。ファイルの読取
//! (I/O) は読み手 (アダプタ層・合成ルート) が行い、読み順どおりの [`RuleContent`] 列で渡す。

use super::directive::RuleContent;

/// チャンクのテキスト目標 (`STEERING_TEXT_TARGET_BYTES = 20 * 1024` — 02 §10)。
const STEERING_TEXT_TARGET_BYTES: usize = 20 * 1024;

/// セクションを輸送目標未満へ分割できない (防御的 — 1 コードポイントが目標を超える場合のみ)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsplittableSection {
    path: String,
}

impl UnsplittableSection {
    /// 該当セクションを含むルールファイルのパス。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl std::fmt::Display for UnsplittableSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unsplittable section in {}", self.path)
    }
}

impl std::error::Error for UnsplittableSection {}

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

/// 配信計画 — piece をパック済みのチャンク列。
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

    /// ルールファイル列 (読み順の path + 全文) を分割・パックして計画に組む (本型のファクトリ)。
    /// 束が空なら空計画 (bare run-stage)。
    ///
    /// Markdown 見出し境界 (`#` 始まりの行) で分割し、目標を超えるセクションは
    /// コードポイント境界で無損失に刻み、読み順のまま輸送目標へパックする (02 §10)。
    ///
    /// # Errors
    ///
    /// 1 コードポイントが輸送目標を超えるセクションは分割不能 (`UnsplittableSection` —
    /// 防御的)。
    pub fn pack(files: &[RuleContent]) -> Result<SteeringPlan, UnsplittableSection> {
        let mut pieces = Vec::new();
        for file in files {
            // 分割予算はパック予算と同じ帳簿で数える — piece は text + path のバイト数で
            // パックされるので、分割の閾値からも path のぶんを差し引く。帳簿が食い違うと、
            // 目標ちょうどのセクションが分割されず 1 チャンクが目標を path 長ぶん超える。
            let budget = STEERING_TEXT_TARGET_BYTES.saturating_sub(file.path().len());
            for section in split_at_headings(file.text()) {
                if section.len() <= budget {
                    pieces.push(RuleContent::new(file.path().to_string(), section));
                    continue;
                }
                let slices =
                    split_by_codepoints(&section, budget).ok_or_else(|| UnsplittableSection {
                        path: file.path().to_string(),
                    })?;
                for slice in slices {
                    pieces.push(RuleContent::new(file.path().to_string(), slice));
                }
            }
        }
        let mut chunks: Vec<Vec<RuleContent>> = Vec::new();
        let mut current: Vec<RuleContent> = Vec::new();
        let mut current_bytes = 0usize;
        for piece in pieces {
            let piece_bytes = piece.text().len() + piece.path().len();
            if !current.is_empty() && current_bytes + piece_bytes > STEERING_TEXT_TARGET_BYTES {
                chunks.push(std::mem::take(&mut current));
                current_bytes = 0;
            }
            current_bytes += piece_bytes;
            current.push(piece);
        }
        if !current.is_empty() {
            chunks.push(current);
        }
        Ok(SteeringPlan::new(chunks))
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

/// Markdown 見出し境界 (`#` 始まりの行) で分割する。見出しの無いファイルは丸ごと 1 piece。
fn split_at_headings(text: &str) -> Vec<String> {
    let mut sections: Vec<String> = Vec::new();
    let mut current = String::new();
    for line in text.lines() {
        if line.starts_with('#') && !current.trim().is_empty() {
            sections.push(std::mem::take(&mut current));
        }
        current.push_str(line);
        current.push('\n');
    }
    if !current.trim().is_empty() {
        sections.push(current);
    }
    sections
}

/// 過大セクションをコードポイント境界で予算以下へ分割する。分割不能は `None`。
fn split_by_codepoints(section: &str, budget: usize) -> Option<Vec<String>> {
    let mut slices = Vec::new();
    let mut current = String::new();
    for c in section.chars() {
        if current.len() + c.len_utf8() > budget {
            if current.is_empty() {
                // 1 コードポイントが予算を超える — 分割不能 (防御的)。
                return None;
            }
            slices.push(std::mem::take(&mut current));
        }
        current.push(c);
    }
    if !current.is_empty() {
        slices.push(current);
    }
    Some(slices)
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

    #[test]
    fn sections_split_at_heading_boundaries_and_pack_to_the_target() {
        // 12KiB のセクション 3 つ → 20KiB ターゲットで 3 部 (12KiB×2 は 20KiB 超)。
        let big = "x".repeat(12 * 1024);
        let file = RuleContent::new(
            "org.md".to_string(),
            format!("# A\n{big}\n# B\n{big}\n# C\n{big}\n"),
        );
        let plan = SteeringPlan::pack(std::slice::from_ref(&file)).unwrap();
        assert_eq!(plan.part_count().as_u32(), 3);
    }

    #[test]
    fn an_oversize_section_splits_losslessly_at_codepoint_boundaries() {
        let huge = "あ".repeat(9 * 1024); // 27KiB (3 bytes × 9K) — 1 セクションでターゲット超
        let body = format!("# Huge\n{huge}\n");
        let file = RuleContent::new("org.md".to_string(), body.clone());
        let plan = SteeringPlan::pack(std::slice::from_ref(&file)).unwrap();
        assert!(plan.part_count().as_u32() >= 2);
        let rebuilt: String = plan
            .chunks()
            .iter()
            .flatten()
            .map(RuleContent::text)
            .collect();
        assert_eq!(rebuilt, body, "分割は無損失");
    }

    #[test]
    fn a_file_without_headings_is_a_single_piece() {
        let file = RuleContent::new(
            "org.md".to_string(),
            "plain text\nno headings\n".to_string(),
        );
        let plan = SteeringPlan::pack(std::slice::from_ref(&file)).unwrap();
        assert_eq!(plan.chunks().iter().flatten().count(), 1);
    }

    #[test]
    fn the_split_budget_counts_the_path_so_no_chunk_exceeds_the_target() {
        // 目標ちょうどのセクションでも、piece は text + path で数えられる — 分割予算が
        // path のぶんを差し引かないと 1 チャンクが目標を超える (回帰)。
        // 末尾改行つきの 20480 バイト 1 行 (見出し分割は行を改行正規化するので、無損失比較は
        // 改行で終わる素材で行う)。
        let body = format!("{}\n", "x".repeat(20 * 1024 - 1));
        let file = RuleContent::new("p.md".to_string(), body);
        let plan = SteeringPlan::pack(std::slice::from_ref(&file)).unwrap();
        for chunk in plan.chunks() {
            let total: usize = chunk
                .iter()
                .map(|piece| piece.text().len() + piece.path().len())
                .sum();
            assert!(total <= 20 * 1024, "チャンク {total} バイトが目標を超えた");
        }
        let rebuilt: String = plan
            .chunks()
            .iter()
            .flatten()
            .map(RuleContent::text)
            .collect();
        assert_eq!(rebuilt.len(), 20 * 1024, "分割は無損失");
    }

    #[test]
    fn packing_no_files_plans_nothing() {
        assert!(SteeringPlan::pack(&[]).unwrap().is_empty());
    }

    #[test]
    fn a_codepoint_wider_than_the_remaining_budget_is_refused_through_pack() {
        // 分割予算は `STEERING_TEXT_TARGET_BYTES - path.len()` なので、極端に長いパスは
        // 予算を数バイトまで削る。そこへマルチバイト 1 文字が来ると「1 コードポイントが
        // 予算を超える」が成立し、`split_by_codepoints` が分割不能を返す。
        //
        // **この枝は予算に path を含めた改訂で初めて到達可能になった** — それ以前の予算は
        // 常に 20KiB 固定で、1 文字 (最大 4 バイト) がそれを超えることは構成不能だった。
        // 防御枝を pack 経由で 1 度は踏んでおく。
        const TAIL: &str = "/deep.md";
        let path = format!(
            "{}{TAIL}",
            "d".repeat(STEERING_TEXT_TARGET_BYTES - 2 - TAIL.len())
        );
        assert_eq!(
            path.len(),
            STEERING_TEXT_TARGET_BYTES - 2,
            "残り予算を 2 バイトに削るパス長"
        );
        // 'あ' は 3 バイト — 残り 2 バイトには収まらない。
        let file = RuleContent::new(path.clone(), "あ".to_string());

        let error = SteeringPlan::pack(std::slice::from_ref(&file)).unwrap_err();
        assert_eq!(error.path(), path, "拒否はどのファイルかを運ぶ");
        assert_eq!(error.to_string(), format!("unsplittable section in {path}"));
    }

    #[test]
    fn the_unsplittable_error_names_its_file() {
        let error = UnsplittableSection {
            path: "org.md".to_string(),
        };
        assert_eq!(error.path(), "org.md");
        assert_eq!(error.to_string(), "unsplittable section in org.md");
        let boxed: Box<dyn std::error::Error> = Box::new(error);
        assert_eq!(boxed.to_string(), "unsplittable section in org.md");
    }
}
