//! `SteeringTables` — 参照入力 (memory 層の規則ファイル) 由来の投影単位。
//!
//! # ジャーナル由来の面とは別の投影単位である
//!
//! [`ReadTables`] がジャーナルの全履歴から作られるのに対し、こちらは**人が編集するファイル**
//! から作られる。ジャーナルの走査位置とは無関係なので `as_of` を持たず、代わりに
//! [`MemoryRules::source_digest`] を保存済みの値と比べて、変わったときだけ差し替える
//! (取得ループの仕事 — 設計 §3)。書込も別トランザクションである。
//!
//! # 分割とパックはここに複製されている
//!
//! 見出し境界での分割・過大セクションのコードポイント分割・輸送目標へのパック (02 §10) は、
//! クエリ側の `SteeringPlan::pack` の**写し**である。RMU が計算結果まで作ってリードモデルへ
//! 置き、クエリ側は書かれた答えを読むだけになる (`coding-rules/cqrs-boundaries.md` 規則 6 の
//! 2026-09-02 追記)。クエリ側の複製は Bolt 3 で削除する。
//!
//! パックは CPU とメモリだけの計算であり、I/O を持たない — 読取は取得ループの
//! [`SteeringSource`] が行い、読み終えた [`MemoryRules`] を渡す。
//!
//! [`ReadTables`]: super::ReadTables
//! [`SteeringSource`]: crate::orchestration::SteeringSource

use core_command_domain::workflow_definition::PhaseId;

use super::memory_rules::MemoryRules;
use super::rule_content::RuleContent;
use super::steering_part_row::SteeringPartRow;
use super::steering_plan_row::SteeringPlanRow;
use super::unsplittable_section::UnsplittableSection;

/// チャンクのテキスト目標 (`STEERING_TEXT_TARGET_BYTES = 20 * 1024` — 02 §10)。
const STEERING_TEXT_TARGET_BYTES: usize = 20 * 1024;

/// 1 回のパックで作った steering 2 表の全行と、その素になった参照入力のダイジェスト。
///
/// フィールドは private。行の並びは決定的である — フェーズは番号順、部は 1 始まりの昇順で
/// あり、同じ参照入力からは同じ順序の行が出る。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteeringTables {
    plans: Vec<SteeringPlanRow>,
    parts: Vec<SteeringPartRow>,
    chunks: Vec<(PhaseId, Vec<Vec<RuleContent>>)>,
    source_digest: String,
}

impl SteeringTables {
    /// 読み終えた規則束を steering 2 表の行へパックする (**この型の唯一の構築経路**)。
    ///
    /// **5 フェーズすべてに計画の行が立つ** — 束が空でも行は在る。空計画は「規則を配らずに
    /// run-stage を出す」(bare run-stage) という答えであって、答えが無いことではない。
    /// initialization はフェーズ規則ファイルを持たないので base だけの束になる。
    ///
    /// # Errors
    ///
    /// 1 コードポイントが輸送目標を超えるセクションは分割不能 ([`UnsplittableSection`] —
    /// 防御的)。1 フェーズでも刻めなければ**行を 1 つも作らずに**止める (部分的な steering
    /// 面を書くと、読み手は「間違った束が在る」を見ることになる)。
    pub fn pack(rules: &MemoryRules) -> Result<SteeringTables, UnsplittableSection> {
        let source_digest = rules.source_digest();
        let mut plans = Vec::new();
        let mut parts = Vec::new();
        let mut chunks_by_phase = Vec::new();
        for phase in phases() {
            let chunks = pack_files(&rules.files_for(phase))?;
            plans.push(SteeringPlanRow::of(phase, &chunks));
            for (position, chunk) in chunks.iter().enumerate() {
                parts.push(SteeringPartRow::of(phase, position + 1, chunk));
            }
            chunks_by_phase.push((phase, chunks));
        }
        Ok(SteeringTables {
            plans,
            parts,
            chunks: chunks_by_phase,
            source_digest,
        })
    }

    /// `read_steering_plan` の行 (フェーズ番号順)。
    #[must_use]
    pub fn plans(&self) -> &[SteeringPlanRow] {
        &self.plans
    }

    /// `read_steering_part` の行 (フェーズ番号順・部の昇順)。
    #[must_use]
    pub fn parts(&self) -> &[SteeringPartRow] {
        &self.parts
    }

    /// 素になった参照入力のダイジェスト (全行に同じ値が書かれる)。
    #[must_use]
    pub fn source_digest(&self) -> &str {
        &self.source_digest
    }

    /// そのフェーズのパック済みチャンク列 (パックの性質を確かめるための素の形)。
    #[must_use]
    pub fn chunks_of(&self, phase: PhaseId) -> &[Vec<RuleContent>] {
        self.chunks
            .iter()
            .find(|(candidate, _)| *candidate == phase)
            .map_or(&[], |(_, chunks)| chunks.as_slice())
    }
}

/// フェーズの全列挙 (番号順)。
fn phases() -> impl Iterator<Item = PhaseId> {
    (0..=4_u32).filter_map(PhaseId::from_index)
}

/// 規則ファイル列を分割・パックしてチャンク列にする (クエリ側 `SteeringPlan::pack` の写し)。
///
/// 空のチャンクは部として意味を持たないので作らない。
fn pack_files(files: &[RuleContent]) -> Result<Vec<Vec<RuleContent>>, UnsplittableSection> {
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
            let slices = split_by_codepoints(&section, budget)
                .ok_or_else(|| UnsplittableSection::new(file.path().to_string()))?;
            for slice in slices {
                pieces.push(RuleContent::new(file.path().to_string(), slice));
            }
        }
    }
    let mut chunks: Vec<Vec<RuleContent>> = Vec::new();
    let mut current: Vec<RuleContent> = Vec::new();
    let mut current_bytes = 0_usize;
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
    Ok(chunks)
}

/// Markdown 見出し境界 (`#` 始まりの行) で分割する。見出しの無いファイルは丸ごと 1 piece。
///
/// 分割は**無損失**である — 全セクションを結合すると入力と 1 バイトも違わない
/// (`lines()` は終端改行の有無と CRLF を潰すので使わない)。本文はダイジェスト
/// (`bundle_digest`) の素材でもあるため、正規化はここに置かない。
fn split_at_headings(text: &str) -> Vec<String> {
    let mut sections: Vec<String> = Vec::new();
    let mut current = String::new();
    for line in text.split_inclusive('\n') {
        if line.starts_with('#') && !current.is_empty() {
            sections.push(std::mem::take(&mut current));
        }
        current.push_str(line);
    }
    if !current.is_empty() {
        sections.push(current);
    }
    sections
}

/// 過大セクションをコードポイント境界で予算以下へ分割する。分割不能は `None`。
fn split_by_codepoints(section: &str, budget: usize) -> Option<Vec<String>> {
    let mut slices = Vec::new();
    let mut current = String::new();
    for character in section.chars() {
        if current.len() + character.len_utf8() > budget {
            if current.is_empty() {
                // 1 コードポイントが予算を超える — 分割不能 (防御的)。
                return None;
            }
            slices.push(std::mem::take(&mut current));
        }
        current.push(character);
    }
    if !current.is_empty() {
        slices.push(current);
    }
    Some(slices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read_tables::{MemoryRules, RuleContent};
    use core_command_domain::workflow_definition::PhaseId;

    fn content(path: &str, text: &str) -> RuleContent {
        RuleContent::new(path.to_string(), text.to_string())
    }

    fn rules(base: &[(&str, &str)], phases: &[(PhaseId, (&str, &str))]) -> MemoryRules {
        MemoryRules::new(
            base.iter().map(|(p, t)| content(p, t)).collect(),
            phases
                .iter()
                .map(|(phase, (p, t))| (*phase, content(p, t)))
                .collect(),
        )
    }

    /// 束の中身を部の順に平坦化して集めた本文 (無損失性の検収に使う)。
    fn rebuilt(tables: &SteeringTables, phase: PhaseId) -> String {
        tables
            .parts()
            .iter()
            .filter(|row| row.phase() == phase.as_str())
            .map(|row| row.rules_content().to_string())
            .collect::<Vec<_>>()
            .join("")
    }

    #[test]
    fn every_phase_gets_a_plan_row_and_initialization_carries_the_base_only() {
        let tables = SteeringTables::pack(&rules(
            &[("memory/org.md", "# Org\n")],
            &[
                (PhaseId::Inception, ("memory/phases/inception.md", "# I\n")),
                (
                    PhaseId::Construction,
                    ("memory/phases/construction.md", "# C\n"),
                ),
            ],
        ))
        .expect("小さな束は分割できる");

        let phases: Vec<&str> = tables.plans().iter().map(SteeringPlanRow::phase).collect();
        assert_eq!(
            phases,
            [
                "initialization",
                "ideation",
                "inception",
                "construction",
                "operation"
            ],
            "束は phase の関数 — 5 フェーズすべてに行が立つ"
        );

        let initialization = tables
            .plans()
            .iter()
            .find(|row| row.phase() == "initialization")
            .expect("initialization の行");
        assert_eq!(
            initialization.delivered_paths(),
            r#"["memory/org.md"]"#,
            "initialization はフェーズ規則を持たないので base だけを配る"
        );

        let inception = tables
            .plans()
            .iter()
            .find(|row| row.phase() == "inception")
            .expect("inception の行");
        assert_eq!(
            inception.delivered_paths(),
            r#"["memory/org.md","memory/phases/inception.md"]"#,
            "フェーズ規則は base の後 (strict-additive)"
        );
    }

    #[test]
    fn a_phase_rule_of_another_phase_is_not_packed() {
        let tables = SteeringTables::pack(&rules(
            &[("memory/org.md", "# Org\n")],
            &[(
                PhaseId::Construction,
                ("memory/phases/construction.md", "# C\n"),
            )],
        ))
        .expect("パックできる");
        assert_eq!(
            rebuilt(&tables, PhaseId::Inception),
            r##"[{"path":"memory/org.md","text":"# Org\n"}]"##,
            "inception の束に construction の規則は載らない"
        );
    }

    #[test]
    fn an_empty_memory_layer_plans_nothing_but_still_rows_every_phase() {
        let tables = SteeringTables::pack(&MemoryRules::default()).expect("空も計画できる");
        assert_eq!(tables.plans().len(), 5);
        assert!(
            tables.parts().is_empty(),
            "部は 1 つも無い (bare run-stage)"
        );
        for row in tables.plans() {
            assert_eq!(row.part_count(), 0);
            assert_eq!(row.delivered_paths(), "[]");
        }
    }

    #[test]
    fn sections_split_at_heading_boundaries_and_pack_to_the_target() {
        // 12KiB のセクション 3 つ → 20KiB ターゲットで 3 部 (12KiB×2 は 20KiB 超)。
        let big = "x".repeat(12 * 1024);
        let body = format!("# A\n{big}\n# B\n{big}\n# C\n{big}\n");
        let tables = SteeringTables::pack(&rules(&[("org.md", &body)], &[])).expect("パックできる");
        let row = tables
            .plans()
            .iter()
            .find(|row| row.phase() == "ideation")
            .expect("ideation の行");
        assert_eq!(row.part_count(), 3);
    }

    #[test]
    fn an_oversize_section_splits_losslessly_at_codepoint_boundaries() {
        let huge = "あ".repeat(9 * 1024); // 27KiB — 1 セクションでターゲット超
        let body = format!("# Huge\n{huge}\n");
        let tables = SteeringTables::pack(&rules(&[("org.md", &body)], &[])).expect("パックできる");
        let row = tables
            .plans()
            .iter()
            .find(|row| row.phase() == "ideation")
            .expect("ideation の行");
        assert!(row.part_count() >= 2);
        assert!(
            rebuilt(&tables, PhaseId::Ideation).contains("あ"),
            "刻んだ本文が行に載る"
        );
    }

    #[test]
    fn the_split_budget_counts_the_path_so_no_chunk_exceeds_the_target() {
        // 目標ちょうどのセクションでも piece は text + path で数えられる — 予算が path の
        // ぶんを差し引かないと 1 チャンクが目標を超える (クエリ側 PR #67 の回帰)。
        let body = format!("{}\n", "x".repeat(20 * 1024 - 1));
        let tables = SteeringTables::pack(&rules(&[("p.md", &body)], &[])).expect("パックできる");
        for chunk in tables.chunks_of(PhaseId::Ideation) {
            let total: usize = chunk
                .iter()
                .map(|piece| piece.text().len() + piece.path().len())
                .sum();
            assert!(total <= 20 * 1024, "チャンク {total} バイトが目標を超えた");
        }
    }

    #[test]
    fn the_split_preserves_the_body_byte_for_byte() {
        for body in ["a", "a\r\nb", " \t", "# H\r\npara\r\n", "pre\n# H\nbody"] {
            let tables =
                SteeringTables::pack(&rules(&[("org.md", body)], &[])).expect("パックできる");
            let text: String = tables
                .chunks_of(PhaseId::Ideation)
                .iter()
                .flatten()
                .map(RuleContent::text)
                .collect();
            assert_eq!(text, body, "分割は無損失: {body:?}");
        }
    }

    #[test]
    fn a_codepoint_wider_than_the_remaining_budget_is_refused() {
        // 分割予算は 20KiB - path.len() なので、極端に長いパスは予算を数バイトまで削る。
        const TAIL: &str = "/deep.md";
        let path = format!("{}{TAIL}", "d".repeat(20 * 1024 - 2 - TAIL.len()));
        let error = SteeringTables::pack(&rules(&[(&path, "あ")], &[])).expect_err("分割不能");
        assert_eq!(error.path(), path, "拒否はどのファイルかを運ぶ");
        assert_eq!(error.to_string(), format!("unsplittable section in {path}"));
    }

    #[test]
    fn the_bundle_digest_reads_the_chunk_nesting_not_a_flat_list() {
        // 内容が同じでも分割が違えば別の束である — 平坦化すると continue の照合が
        // 分割の変化を見逃し、部の欠落・重複配信を許す。
        let joined = SteeringTables::pack(&rules(&[("a.md", "# X\n"), ("b.md", "# Y\n")], &[]))
            .expect("パックできる");
        // 20KiB 目標いっぱいの詰め物で 2 部に割る。
        let filler = "z".repeat(20 * 1024);
        let split = SteeringTables::pack(&rules(
            &[("a.md", &format!("# X\n{filler}")), ("b.md", "# Y\n")],
            &[],
        ))
        .expect("パックできる");
        let digest = |tables: &SteeringTables| {
            tables
                .plans()
                .iter()
                .find(|row| row.phase() == "ideation")
                .expect("ideation の行")
                .bundle_digest()
                .to_string()
        };
        assert_ne!(digest(&joined), digest(&split));
        assert_eq!(digest(&joined), digest(&joined), "決定的である");
    }

    #[test]
    fn the_part_index_starts_at_one() {
        let big = "x".repeat(12 * 1024);
        let body = format!("# A\n{big}\n# B\n{big}\n");
        let tables = SteeringTables::pack(&rules(&[("org.md", &body)], &[])).expect("パックできる");
        let indexes: Vec<usize> = tables
            .parts()
            .iter()
            .filter(|row| row.phase() == "ideation")
            .map(SteeringPartRow::part_index)
            .collect();
        assert_eq!(indexes, [1, 2]);
    }

    #[test]
    fn the_delivered_paths_ledger_deduplicates_in_reading_order() {
        // 同じファイルが複数の piece に割れても台帳は 1 度だけ載せる。
        let big = "x".repeat(12 * 1024);
        let body = format!("# A\n{big}\n# B\n{big}\n");
        let tables = SteeringTables::pack(&rules(&[("org.md", &body), ("team.md", "# T\n")], &[]))
            .expect("パックできる");
        let row = tables
            .plans()
            .iter()
            .find(|row| row.phase() == "ideation")
            .expect("ideation の行");
        assert_eq!(row.delivered_paths(), r#"["org.md","team.md"]"#);
    }
}
