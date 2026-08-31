//! `WorkflowDefinitionAggregateKeyDto` — 本家 `AggregateId` を満たす定義ストリームのストア鍵。

use core_command_domain::workflow_definition::WorkflowDefinitionId;
use event_store_adapter_rs::types::AggregateId;
use serde::{Deserialize, Serialize};
use std::fmt;

/// `WorkflowDefinition` 集約ストリームのストア鍵。
///
/// 実行・intent のストリームと**同じストアファイルに同居**する。前提は他の 2 つと同じで
/// 「集約識別子の**値**がストア全体で一意」であること — 定義 id はハーネス名 (`claude` /
/// `kiro` …) であり、UUID 空間とは決して衝突しない綴りなので満たされる。
/// 種別名は逐語で固定する
/// ([`IntentExecutionAggregateKeyDto`](super::IntentExecutionAggregateKeyDto) と同じ理由)。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowDefinitionAggregateKeyDto(String);

/// `WorkflowDefinition` 集約の種別名 (aid 列・パーティション鍵の材料)。
const DEFINITION_AGGREGATE_TYPE_NAME: &str = "WorkflowDefinition";

impl WorkflowDefinitionAggregateKeyDto {
    /// 定義の識別子をストア鍵へ写す。
    #[must_use]
    pub fn of(id: &WorkflowDefinitionId) -> WorkflowDefinitionAggregateKeyDto {
        WorkflowDefinitionAggregateKeyDto(id.as_str().to_string())
    }

    /// 鍵の生値 (ドメインへ戻すときの材料)。
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkflowDefinitionAggregateKeyDto {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AggregateId for WorkflowDefinitionAggregateKeyDto {
    fn type_name(&self) -> String {
        DEFINITION_AGGREGATE_TYPE_NAME.to_string()
    }

    fn value(&self) -> String {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::super::IntentExecutionAggregateKeyDto;
    use super::*;
    use core_command_domain::orchestration::IntentExecutionId;

    #[test]
    fn the_definition_key_reports_a_distinct_aggregate_type_name() {
        // 3 種の集約が同じストアファイルに同居する — 種別名が違えば pkey / skey は衝突しない。
        let definition = WorkflowDefinitionId::parse("claude").expect("定義 id");
        let key = WorkflowDefinitionAggregateKeyDto::of(&definition);
        assert_eq!(key.type_name(), "WorkflowDefinition");
        assert_eq!(key.value(), "claude");
        assert_eq!(key.raw(), "claude");
        assert_eq!(key.to_string(), "claude");

        let execution =
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("UUIDv7");
        assert_ne!(
            key.type_name(),
            IntentExecutionAggregateKeyDto::of(&execution).type_name()
        );
    }
}
