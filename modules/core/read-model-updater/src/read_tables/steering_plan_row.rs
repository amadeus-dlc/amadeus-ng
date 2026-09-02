//! `SteeringPlanRow` — `read_steering_plan` の 1 行 (フェーズ 1 つぶんの配信計画)。

use core_command_domain::workflow_definition::PhaseId;

use super::digest;
use super::json_column;
use super::rule_content::RuleContent;

/// `read_steering_plan` の 1 行。主キーは `phase`。
///
/// **束は phase の関数である** — ステージの `rules_in_context` は束の選択に使わない
/// (設計 §0 の調査事実)。したがって行はフェーズごとに 1 本で、実行にも scope にも依らない。
///
/// `as_of` 列を持たないのは、この面が**参照入力由来**でジャーナルの走査位置と無関係だから
/// である。いつ時点かを名乗るのは `source_digest` の役目であり、それが変わらない限り行は
/// 書き替わらない。`source_digest` はスナップショット全体の性質なので行型には持たせず、
/// 書込 (`sql`) が [`SteeringTables::source_digest`] を全行へ書く (`as_of` と同じ流儀)。
///
/// [`SteeringTables::source_digest`]: super::SteeringTables::source_digest
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteeringPlanRow {
    phase: String,
    bundle_digest: String,
    part_count: usize,
    delivered_paths: String,
}

impl SteeringPlanRow {
    /// フェーズ 1 つぶんのパック済みチャンク列を 1 行へ写す (**この型の唯一の構築経路**)。
    #[must_use]
    pub(crate) fn of(phase: PhaseId, chunks: &[Vec<RuleContent>]) -> SteeringPlanRow {
        SteeringPlanRow {
            phase: phase.as_str().to_string(),
            bundle_digest: digest::bundle(chunks),
            part_count: chunks.len(),
            delivered_paths: json_column::strings(&delivered_paths(chunks)),
        }
    }

    /// フェーズの綴り (`PhaseId::as_str`)。
    #[must_use]
    pub fn phase(&self) -> &str {
        &self.phase
    }

    /// ルール束のダイジェスト (チャンクの入れ子配列 — 分割境界を含む)。
    #[must_use]
    pub fn bundle_digest(&self) -> &str {
        &self.bundle_digest
    }

    /// パート総数 (0 = 空計画 = bare run-stage)。
    #[must_use]
    pub const fn part_count(&self) -> usize {
        self.part_count
    }

    /// 配信済みルールのパス台帳の 1 行 JSON 配列 (読み順・重複除去)。
    #[must_use]
    pub fn delivered_paths(&self) -> &str {
        &self.delivered_paths
    }
}

/// パス台帳 — 読み順のまま、同じパスは 1 度だけ。
///
/// 1 ファイルが複数の piece へ割れても台帳の 1 行である。並べ替えも整列もしない
/// (台帳は配信の順序そのものである)。
fn delivered_paths(chunks: &[Vec<RuleContent>]) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    for piece in chunks.iter().flatten() {
        if !paths.iter().any(|path| path == piece.path()) {
            paths.push(piece.path().to_string());
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content(path: &str, text: &str) -> RuleContent {
        RuleContent::new(path.to_string(), text.to_string())
    }

    #[test]
    fn an_empty_plan_rows_zero_parts_and_an_empty_ledger() {
        let row = SteeringPlanRow::of(PhaseId::Initialization, &[]);
        assert_eq!(row.phase(), "initialization");
        assert_eq!(row.part_count(), 0);
        assert_eq!(row.delivered_paths(), "[]");
        assert_eq!(row.bundle_digest().len(), 64);
    }

    #[test]
    fn the_ledger_deduplicates_in_reading_order_across_chunk_boundaries() {
        let chunks = vec![
            vec![content("a.md", "1"), content("a.md", "2")],
            vec![content("b.md", "3"), content("a.md", "4")],
        ];
        let row = SteeringPlanRow::of(PhaseId::Inception, &chunks);
        assert_eq!(
            row.delivered_paths(),
            r#"["a.md","b.md"]"#,
            "並べ替えず、同じパスは 1 度だけ"
        );
        assert_eq!(row.part_count(), 2);
    }
}
