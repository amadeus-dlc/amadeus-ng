//! steering の分割配信 — piece 化 (見出し境界) と 20KiB ターゲットへのパック (02 §10)。
//!
//! 純ロジック。ルールを Markdown 見出し境界で分割し、過大セクションはコードポイント境界で
//! 分割し、piece を `STEERING_TEXT_TARGET_BYTES` までパックする。分割不能は blocking。

use core_command_domain::orchestration::RuleContent;

use super::rule_bundle_source::RuleFile;

/// チャンクのテキスト目標 (`STEERING_TEXT_TARGET_BYTES = 20 * 1024`)。
pub(crate) const STEERING_TEXT_TARGET_BYTES: usize = 20 * 1024;

/// 分割の失敗 (材料のみ — 逐語文言は wording)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SteeringPlanError {
    /// セクションを輸送上限未満へ分割できない。
    UnsplittableSection,
}

/// 配信計画 — piece をパックしたチャンク列。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SteeringPlan {
    chunks: Vec<Vec<RuleContent>>,
}

impl SteeringPlan {
    /// ルール束から配信計画を組む。束が空なら空計画 (bare run-stage)。
    pub(crate) fn from_bundle(files: &[RuleFile]) -> Result<SteeringPlan, SteeringPlanError> {
        let mut pieces = Vec::new();
        for file in files {
            for section in split_at_headings(file.text()) {
                if section.len() <= STEERING_TEXT_TARGET_BYTES {
                    pieces.push(RuleContent::new(file.path().to_string(), section));
                    continue;
                }
                for slice in split_by_codepoints(&section)? {
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
        Ok(SteeringPlan { chunks })
    }

    /// チャンク列。
    pub(crate) fn chunks(&self) -> &[Vec<RuleContent>] {
        &self.chunks
    }

    /// パート総数。
    pub(crate) fn parts(&self) -> u32 {
        u32::try_from(self.chunks.len()).unwrap_or(u32::MAX)
    }

    /// 束が空か (bare run-stage でよい)。
    pub(crate) const fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// 束ダイジェストの素材 — path と text を読み順で連結した決定論的文字列。
    pub(crate) fn digest_material(&self) -> String {
        let mut material = String::new();
        for chunk in &self.chunks {
            for piece in chunk {
                material.push_str(piece.path());
                material.push('\n');
                material.push_str(piece.text());
                material.push('\n');
            }
        }
        material
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

/// 過大セクションをコードポイント境界でターゲット以下へ分割する。
fn split_by_codepoints(section: &str) -> Result<Vec<String>, SteeringPlanError> {
    let mut slices = Vec::new();
    let mut current = String::new();
    for c in section.chars() {
        if current.len() + c.len_utf8() > STEERING_TEXT_TARGET_BYTES {
            if current.is_empty() {
                // 1 コードポイントが上限を超える — 分割不能 (防御的)。
                return Err(SteeringPlanError::UnsplittableSection);
            }
            slices.push(std::mem::take(&mut current));
        }
        current.push(c);
    }
    if !current.is_empty() {
        slices.push(current);
    }
    Ok(slices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_bundle_plans_nothing() {
        let plan = SteeringPlan::from_bundle(&[]).unwrap();
        assert!(plan.is_empty());
        assert_eq!(plan.parts(), 0);
    }

    #[test]
    fn sections_split_at_heading_boundaries_and_keep_their_path() {
        let files = vec![RuleFile::new(
            "memory/org.md".to_string(),
            "# Org\nbody a\n## Way of Working\nbody b\n".to_string(),
        )];
        let plan = SteeringPlan::from_bundle(&files).unwrap();
        let pieces: Vec<_> = plan.chunks().iter().flatten().collect();
        assert_eq!(pieces.len(), 2);
        let first = pieces.first().unwrap();
        let second = pieces.get(1).unwrap();
        assert_eq!(first.path(), "memory/org.md");
        assert!(first.text().starts_with("# Org"));
        assert!(second.text().starts_with("## Way of Working"));
    }

    #[test]
    fn pieces_pack_up_to_the_target_bytes() {
        // 12KiB のセクション 3 つ → 20KiB ターゲットで 1+1+1 ではなく 1 チャンク 1 つ + 2 つ目。
        let big = "x".repeat(12 * 1024);
        let files = vec![RuleFile::new(
            "memory/team.md".to_string(),
            format!("# A\n{big}\n# B\n{big}\n# C\n{big}\n"),
        )];
        let plan = SteeringPlan::from_bundle(&files).unwrap();
        assert_eq!(
            plan.parts(),
            3,
            "12KiB×2 は 20KiB を超えるので 1 piece 1 チャンク"
        );
    }

    #[test]
    fn an_oversize_section_splits_at_codepoint_boundaries() {
        let huge = "あ".repeat(9 * 1024); // 27KiB (3 bytes × 9K) — 1 セクションでターゲット超
        let files = vec![RuleFile::new(
            "memory/project.md".to_string(),
            format!("# Huge\n{huge}\n"),
        )];
        let plan = SteeringPlan::from_bundle(&files).unwrap();
        assert!(plan.parts() >= 2);
        let rebuilt: String = plan
            .chunks()
            .iter()
            .flatten()
            .map(RuleContent::text)
            .collect();
        assert_eq!(rebuilt, format!("# Huge\n{huge}\n"), "分割は無損失");
    }

    #[test]
    fn the_digest_material_is_deterministic_and_order_preserving() {
        let files = vec![
            RuleFile::new("a.md".to_string(), "# A\n1\n".to_string()),
            RuleFile::new("b.md".to_string(), "# B\n2\n".to_string()),
        ];
        let one = SteeringPlan::from_bundle(&files).unwrap().digest_material();
        let two = SteeringPlan::from_bundle(&files).unwrap().digest_material();
        assert_eq!(one, two);
        assert!(one.find("a.md").unwrap() < one.find("b.md").unwrap());
    }
}
