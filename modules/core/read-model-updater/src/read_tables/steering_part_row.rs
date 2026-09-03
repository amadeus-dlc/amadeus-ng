//! `SteeringPartRow` — `read_steering_part` の 1 行 (配信計画の 1 部の中身)。

use core_command_domain::workflow_definition::PhaseId;

use super::json_column;
use super::row_id;
use super::rule_content::RuleContent;

/// `read_steering_part` の 1 行。主キーは 1 列 `id` (自然キー (`phase`, `part_index`) から
/// 導いた代理キー)。`steering_plan_id` は `read_steering_plan.id` を指す FK である。
///
/// `part_index` は **1 始まり**である (upstream の部番号と同じ数え方 — 「1 / 3 部」)。
/// `rules_content` は `[{path, text}]` の 1 行 JSON で、`load-steering` が届ける中身
/// そのものである。
///
/// `SteeringPlanRow` と同じく `as_of` 列を持たない (参照入力由来)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteeringPartRow {
    id: String,
    steering_plan_id: String,
    phase: String,
    part_index: usize,
    rules_content: String,
}

impl SteeringPartRow {
    /// 1 部のチャンクを 1 行へ写す (**この型の唯一の構築経路**)。
    ///
    /// `part_index` は呼び手が 1 始まりで採番する (チャンク列の位置 + 1)。
    #[must_use]
    pub(crate) fn of(phase: PhaseId, part_index: usize, chunk: &[RuleContent]) -> SteeringPartRow {
        SteeringPartRow {
            id: row_id::steering_part(phase.as_str(), part_index),
            steering_plan_id: row_id::steering_plan(phase.as_str()),
            phase: phase.as_str().to_string(),
            part_index,
            rules_content: json_column::rule_contents(chunk),
        }
    }

    /// 主キー — 自然キー (`phase`, `part_index`) から導いた代理キー。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// `read_steering_plan.id` を指す FK (同じフェーズの配信計画)。
    #[must_use]
    pub fn steering_plan_id(&self) -> &str {
        &self.steering_plan_id
    }

    /// フェーズの綴り (`PhaseId::as_str`)。
    #[must_use]
    pub fn phase(&self) -> &str {
        &self.phase
    }

    /// 部の番号 (1 始まり)。
    #[must_use]
    pub const fn part_index(&self) -> usize {
        self.part_index
    }

    /// この部が届ける `[{path, text}]` の 1 行 JSON 配列。
    #[must_use]
    pub fn rules_content(&self) -> &str {
        &self.rules_content
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_part_carries_its_pieces_as_a_path_and_text_array() {
        let chunk = [RuleContent::new(
            "org.md".to_string(),
            "# Org\n".to_string(),
        )];
        let row = SteeringPartRow::of(PhaseId::Ideation, 1, &chunk);
        assert_eq!(row.phase(), "ideation");
        assert_eq!(row.part_index(), 1);
        assert_eq!(
            row.rules_content(),
            r##"[{"path":"org.md","text":"# Org\n"}]"##
        );
    }

    #[test]
    fn an_empty_chunk_is_an_empty_array_not_null() {
        assert_eq!(
            SteeringPartRow::of(PhaseId::Operation, 3, &[]).rules_content(),
            "[]"
        );
    }
}
