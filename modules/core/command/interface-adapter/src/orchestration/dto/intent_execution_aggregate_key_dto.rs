//! `IntentExecutionAggregateKeyDto` — 本家 `AggregateId` を満たす実行ストリームのストア鍵。
//!
//! `AggregateId` は「aid 列をどう組むか」というストアの語彙であり、ドメインの識別子型が
//! 直接実装するものではない (`coding-rules/domain-persistence-neutrality.md` /
//! `coding-rules/upstream-contracts.md` — 境界で変換する)。ドメインの
//! [`IntentExecutionId`] を包み、この層で trait を満たす。

use core_command_domain::orchestration::IntentExecutionId;
use event_store_adapter_rs::types::AggregateId;
use serde::{Deserialize, Serialize};
use std::fmt;

/// ストアが集約ストリームを引くときの鍵。
///
/// `type_name` は集約種別名で、本家がパーティション鍵の組み立てに使う。値が変わると既存の
/// 行を引けなくなるので逐語で固定する。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IntentExecutionAggregateKeyDto(String);

/// `IntentExecution` 集約の種別名 (aid 列・パーティション鍵の材料)。
const AGGREGATE_TYPE_NAME: &str = "IntentExecution";

impl IntentExecutionAggregateKeyDto {
    /// 実行の識別子をストア鍵へ写す。
    #[must_use]
    pub fn of(id: &IntentExecutionId) -> IntentExecutionAggregateKeyDto {
        IntentExecutionAggregateKeyDto(id.as_str().to_string())
    }

    /// 鍵の生値 (ドメインへ戻すときの材料)。
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IntentExecutionAggregateKeyDto {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AggregateId for IntentExecutionAggregateKeyDto {
    fn type_name(&self) -> String {
        AGGREGATE_TYPE_NAME.to_string()
    }

    fn value(&self) -> String {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id() -> IntentExecutionId {
        IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("UUIDv7")
    }

    #[test]
    fn the_key_reports_the_aggregate_type_name_and_the_raw_value() {
        let key = IntentExecutionAggregateKeyDto::of(&id());
        assert_eq!(key.type_name(), "IntentExecution");
        assert_eq!(key.value(), "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000");
        assert_eq!(key.raw(), "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000");
        assert_eq!(key.to_string(), "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000");
    }

    #[test]
    fn keys_built_from_the_same_id_compare_equal() {
        assert_eq!(
            IntentExecutionAggregateKeyDto::of(&id()),
            IntentExecutionAggregateKeyDto::of(&id())
        );
    }
}
