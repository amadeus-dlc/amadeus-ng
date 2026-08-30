//! `AggregateKey` — 本家 `AggregateId` を満たすストア鍵のラッパ型。
//!
//! `AggregateId` は「aid 列をどう組むか」というストアの語彙であり、ドメインの識別子型が
//! 直接実装するものではない (`coding-rules/domain-persistence-neutrality.md` /
//! `coding-rules/upstream-contracts.md` — 境界で変換する)。ドメインの
//! [`IntentExecutionId`] を包み、この層で trait を満たす。

use core_command_domain::orchestration::{IntentExecutionId, IntentId};
use event_store_adapter_rs::types::AggregateId;
use serde::{Deserialize, Serialize};
use std::fmt;

/// ストアが集約ストリームを引くときの鍵。
///
/// `type_name` は集約種別名で、本家がパーティション鍵の組み立てに使う。値が変わると既存の
/// 行を引けなくなるので逐語で固定する。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AggregateKey(String);

/// `IntentExecution` 集約の種別名 (aid 列・パーティション鍵の材料)。
const AGGREGATE_TYPE_NAME: &str = "IntentExecution";

impl AggregateKey {
    /// 実行の識別子をストア鍵へ写す。
    #[must_use]
    pub fn of(id: &IntentExecutionId) -> AggregateKey {
        AggregateKey(id.as_str().to_string())
    }

    /// 鍵の生値 (ドメインへ戻すときの材料)。
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AggregateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AggregateId for AggregateKey {
    fn type_name(&self) -> String {
        AGGREGATE_TYPE_NAME.to_string()
    }

    fn value(&self) -> String {
        self.0.clone()
    }
}

/// `Intent` 集約ストリームのストア鍵。
///
/// 実行のストリームと**同じストアファイルに同居**する (issue #50 の設計裁定)。本家の
/// pkey / skey は `type_name` を材料に組まれて種別ごとに分かれるが、journal の
/// `(aid, seq_nr)` UNIQUE 索引だけは **`type_name` を含まない生値**で張られている (実測) —
/// したがって同居の前提は「集約識別子の**値**がストア全体で一意」であることで、識別子が
/// UUID である限り満たされる。種別名は逐語で固定する ([`AggregateKey`] と同じ理由)。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IntentAggregateKey(String);

/// `Intent` 集約の種別名 (aid 列・パーティション鍵の材料)。
const INTENT_AGGREGATE_TYPE_NAME: &str = "Intent";

impl IntentAggregateKey {
    /// intent の識別子をストア鍵へ写す。
    #[must_use]
    pub fn of(id: &IntentId) -> IntentAggregateKey {
        IntentAggregateKey(id.as_str().to_string())
    }

    /// 鍵の生値 (ドメインへ戻すときの材料)。
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IntentAggregateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AggregateId for IntentAggregateKey {
    fn type_name(&self) -> String {
        INTENT_AGGREGATE_TYPE_NAME.to_string()
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
        let key = AggregateKey::of(&id());
        assert_eq!(key.type_name(), "IntentExecution");
        assert_eq!(key.value(), "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000");
        assert_eq!(key.raw(), "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000");
        assert_eq!(key.to_string(), "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000");
    }

    #[test]
    fn keys_built_from_the_same_id_compare_equal() {
        assert_eq!(AggregateKey::of(&id()), AggregateKey::of(&id()));
    }

    #[test]
    fn the_intent_key_reports_a_distinct_aggregate_type_name() {
        // 同じストアファイルに同居しても、種別名が違えば pkey / skey は衝突しない
        // (issue #50)。
        let intent = IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("UUIDv7");
        let key = IntentAggregateKey::of(&intent);
        assert_eq!(key.type_name(), "Intent");
        assert_eq!(key.value(), "01a02785-1bd8-76eb-aeea-5aa303ebd5b6");
        assert_eq!(key.raw(), "01a02785-1bd8-76eb-aeea-5aa303ebd5b6");
        assert_eq!(key.to_string(), "01a02785-1bd8-76eb-aeea-5aa303ebd5b6");
        assert_ne!(key.type_name(), AggregateKey::of(&id()).type_name());
    }
}
