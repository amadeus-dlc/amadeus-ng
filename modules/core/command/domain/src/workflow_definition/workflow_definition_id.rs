//! `WorkflowDefinitionId` — 集約 `WorkflowDefinition` の識別子 (ADR-008)。

use std::fmt;

use super::workflow_definition_id_error::WorkflowDefinitionIdError;

/// このハーネスにインストールされたワークフロー定義の**系譜 ID** (Always Valid — 不正値は
/// この型に存在しない)。
///
/// 内容 (ピン更新・プラグイン選択・再コンパイル) が変わっても不変であり、内容の版は
/// `DefinitionRevision` が別に持つ。供給元は Repository 実装で、値は `harness.json` の
/// `name` (出荷ハーネスでは `claude`)。
///
/// `Ord` は生文字列の辞書順。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkflowDefinitionId(String);

impl WorkflowDefinitionId {
    /// 前後の空白を落としてから検証する。
    ///
    /// # Errors
    ///
    /// 空 (空白のみを含む) と、制御文字を含む値を拒否する。
    pub fn parse(s: &str) -> Result<WorkflowDefinitionId, WorkflowDefinitionIdError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(WorkflowDefinitionIdError::Empty);
        }
        if let Some(c) = trimmed.chars().find(|c| c.is_control()) {
            return Err(WorkflowDefinitionIdError::ControlCharacter(c));
        }
        Ok(WorkflowDefinitionId(trimmed.to_string()))
    }

    /// 生の id 文字列 (trim 済み)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for WorkflowDefinitionId {
    type Error = WorkflowDefinitionIdError;

    fn try_from(value: String) -> Result<WorkflowDefinitionId, WorkflowDefinitionIdError> {
        WorkflowDefinitionId::parse(&value)
    }
}

impl fmt::Display for WorkflowDefinitionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashSet};

    #[test]
    fn parse_round_trips_the_shipped_harness_name() {
        for raw in ["claude", "kiro", "codex", "opencode", "copilot"] {
            let id = WorkflowDefinitionId::parse(raw).unwrap();
            assert_eq!(id.as_str(), raw);
            assert_eq!(id.to_string(), raw);
        }
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_before_validation() {
        let id = WorkflowDefinitionId::parse("  claude\n").unwrap();
        assert_eq!(id.as_str(), "claude");
        // trim の結果が同じなら同値。
        assert_eq!(id, WorkflowDefinitionId::parse("claude").unwrap());
    }

    #[test]
    fn an_empty_or_blank_name_cannot_be_constructed() {
        assert_eq!(
            WorkflowDefinitionId::parse(""),
            Err(WorkflowDefinitionIdError::Empty)
        );
        assert_eq!(
            WorkflowDefinitionId::parse("   \t\n "),
            Err(WorkflowDefinitionIdError::Empty)
        );
    }

    #[test]
    fn an_interior_control_character_is_rejected() {
        // id は状態ファイルと監査行に 1 行として載るため、内部の制御文字は表現できない。
        assert_eq!(
            WorkflowDefinitionId::parse("cla\nude"),
            Err(WorkflowDefinitionIdError::ControlCharacter('\n'))
        );
        assert_eq!(
            WorkflowDefinitionId::parse("cla\u{7}ude"),
            Err(WorkflowDefinitionIdError::ControlCharacter('\u{7}'))
        );
    }

    #[test]
    fn ordering_is_the_lexicographic_order_of_the_raw_string() {
        let mut sorted: Vec<WorkflowDefinitionId> = ["kiro", "claude", "opencode"]
            .iter()
            .map(|s| WorkflowDefinitionId::parse(s).unwrap())
            .collect();
        sorted.sort();
        let names: Vec<&str> = sorted.iter().map(WorkflowDefinitionId::as_str).collect();
        assert_eq!(names, ["claude", "kiro", "opencode"]);
    }

    #[test]
    fn the_id_works_as_a_map_and_set_key() {
        let a = WorkflowDefinitionId::parse("claude").unwrap();
        let b = WorkflowDefinitionId::parse(" claude ").unwrap();
        let mut hashed = HashSet::new();
        hashed.insert(a.clone());
        assert!(
            hashed.contains(&b),
            "Hash は Eq と整合していなければならない"
        );
        let ordered: BTreeSet<WorkflowDefinitionId> = [a, b].into_iter().collect();
        assert_eq!(ordered.len(), 1);
    }

    #[test]
    fn the_rejection_carries_material_not_wording() {
        assert_eq!(WorkflowDefinitionIdError::Empty.to_string(), "empty");
        assert_eq!(
            WorkflowDefinitionIdError::ControlCharacter('\n').to_string(),
            "control character U+000A"
        );
    }
}
