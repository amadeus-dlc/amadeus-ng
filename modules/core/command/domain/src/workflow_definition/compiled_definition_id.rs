//! `CompiledDefinitionId` — 集約 [`CompiledDefinition`] の識別子。
//!
//! [`CompiledDefinition`]: super::CompiledDefinition

use std::fmt;

use super::compiled_definition_id_error::CompiledDefinitionIdError;
use super::workflow_definition_id::WorkflowDefinitionId;

/// コンパイル済み定義 (配布束) の識別子 (Always Valid — 不正値はこの型に存在しない)。
///
/// 値の供給元は Repository 実装で、`harness.json` の `name` (出荷ハーネスでは `claude`)。
/// [`WorkflowDefinitionId`](super::WorkflowDefinitionId) と**同じ系譜を同じ文法で**指すが、
/// 別集約の識別子なので型は別である — 集約は自前の識別子を持ち、Repository は自集約の
/// ID で引く (一般原則。系譜の突合せは合成ルートが同じ `harness.json` から両 ID を鋳造する
/// ことで成立する)。
///
/// `Ord` は生文字列の辞書順。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CompiledDefinitionId(String);

impl CompiledDefinitionId {
    /// 前後の空白を落としてから検証する。
    ///
    /// # Errors
    ///
    /// 空 (空白のみを含む) と、制御文字を含む値を拒否する。
    pub fn parse(s: &str) -> Result<CompiledDefinitionId, CompiledDefinitionIdError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(CompiledDefinitionIdError::Empty);
        }
        if let Some(c) = trimmed.chars().find(|c| c.is_control()) {
            return Err(CompiledDefinitionIdError::ControlCharacter(c));
        }
        Ok(CompiledDefinitionId(trimmed.to_string()))
    }

    /// 生の id 文字列 (trim 済み)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl PartialEq<WorkflowDefinitionId> for CompiledDefinitionId {
    /// 系譜の同一性 — 配布束の識別子とジャーナルの定義の系譜 ID は、同じ `harness.json` の
    /// `name` を同じ文法で指す。等しい名前 = 同じ系譜 (`coding-rules/domain-equality.md`:
    /// ドメインの同値関係は `PartialEq` で表す)。受け手の集約 (`WorkflowDefinition`) は
    /// これで取り違えをガードする。
    fn eq(&self, other: &WorkflowDefinitionId) -> bool {
        self.0 == other.as_str()
    }
}

impl PartialEq<CompiledDefinitionId> for WorkflowDefinitionId {
    /// [`PartialEq<WorkflowDefinitionId> for CompiledDefinitionId`] の対称形。
    fn eq(&self, other: &CompiledDefinitionId) -> bool {
        other == self
    }
}

impl TryFrom<String> for CompiledDefinitionId {
    type Error = CompiledDefinitionIdError;

    fn try_from(value: String) -> Result<CompiledDefinitionId, CompiledDefinitionIdError> {
        CompiledDefinitionId::parse(&value)
    }
}

impl fmt::Display for CompiledDefinitionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_round_trips_the_shipped_harness_name() {
        let id = CompiledDefinitionId::parse("claude").unwrap();
        assert_eq!(id.as_str(), "claude");
        assert_eq!(id.to_string(), "claude");
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_before_validation() {
        assert_eq!(
            CompiledDefinitionId::parse("  claude\n").unwrap(),
            CompiledDefinitionId::parse("claude").unwrap()
        );
    }

    #[test]
    fn an_empty_or_blank_name_cannot_be_constructed() {
        assert_eq!(
            CompiledDefinitionId::parse("   \t\n "),
            Err(CompiledDefinitionIdError::Empty)
        );
    }

    #[test]
    fn an_interior_control_character_is_rejected() {
        assert_eq!(
            CompiledDefinitionId::parse("cla\nude"),
            Err(CompiledDefinitionIdError::ControlCharacter('\n'))
        );
    }

    #[test]
    fn try_from_string_is_the_same_gate_as_parse() {
        assert_eq!(
            CompiledDefinitionId::try_from("  claude ".to_string()),
            CompiledDefinitionId::parse("claude")
        );
        assert_eq!(
            CompiledDefinitionId::try_from(String::new()),
            Err(CompiledDefinitionIdError::Empty)
        );
    }

    #[test]
    fn the_same_name_is_the_same_lineage_across_the_two_id_types() {
        let bundle = CompiledDefinitionId::parse("claude").unwrap();
        assert!(bundle == WorkflowDefinitionId::parse("claude").unwrap());
        assert!(WorkflowDefinitionId::parse("claude").unwrap() == bundle);
        assert!(bundle != WorkflowDefinitionId::parse("kiro").unwrap());
    }

    #[test]
    fn the_rejection_carries_material_not_wording() {
        assert_eq!(CompiledDefinitionIdError::Empty.to_string(), "empty");
        assert_eq!(
            CompiledDefinitionIdError::ControlCharacter('\n').to_string(),
            "control character U+000A"
        );
    }
}
