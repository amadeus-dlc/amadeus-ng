//! `AuditFields` — 監査ブロックのフィールド群 (挿入順を保つ第一級コレクション)。

use super::audit_field_key::AuditFieldKey;
use super::audit_field_value::AuditFieldValue;

/// 描き手が自分で書く 2 つのキーのもう一方 (upstream `EMITTER_OWNED_FIELD_KEYS`)。`Event` と
/// 違い upstream の公開 `append` CLI は受理するので、拒否ではなく破棄する。
const TIMESTAMP_KEY: &str = "Timestamp";

/// 監査ブロックのフィールド群 — **挿入順を保つ**第一級コレクション。
///
/// 並びが観測面である (upstream は JS オブジェクトの列挙順 = 挿入順をそのまま書く) ため、
/// `BTreeMap` / `HashMap` では表現できない。同じキーを二度置くと、**位置は最初のまま値だけ**
/// 差し替わる — JS のプロパティ再代入と同じ意味論である。
///
/// `Timestamp` は受理して**黙って捨てる**。upstream は公開 `append` CLI でこのキーを受け取り、
/// 描画時に読み飛ばす。捨てる位置を描き手ではなくコレクションに置くことで、「第二の
/// `**Timestamp**:` 行は構成不能」が型の性質になる (描き手の規律に頼らない)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AuditFields(Vec<(AuditFieldKey, AuditFieldValue)>);

impl AuditFields {
    /// 空のフィールド群。
    #[must_use]
    pub const fn new() -> AuditFields {
        AuditFields(Vec::new())
    }

    /// フィールドを 1 つ加える (既存キーは位置を保って値だけ差し替え、`Timestamp` は破棄)。
    #[must_use]
    pub fn with(mut self, key: AuditFieldKey, value: &str) -> AuditFields {
        if key.as_str() == TIMESTAMP_KEY {
            return self;
        }
        let escaped = AuditFieldValue::of(value);
        if let Some(slot) = self.0.iter_mut().find(|(existing, _)| *existing == key) {
            slot.1 = escaped;
        } else {
            self.0.push((key, escaped));
        }
        self
    }

    /// 挿入順のフィールド列。
    pub fn iter(&self) -> impl Iterator<Item = (&AuditFieldKey, &AuditFieldValue)> {
        self.0.iter().map(|(key, value)| (key, value))
    }

    /// フィールドが 1 つも無いか。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// フィールドの個数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(raw: &str) -> AuditFieldKey {
        AuditFieldKey::parse(raw).expect("テストのキーは文法内")
    }

    #[test]
    fn the_fields_keep_the_order_they_were_inserted_in() {
        let fields = AuditFields::new()
            .with(key("Stage"), "practices-discovery")
            .with(key("Details"), "done")
            .with(key("Agent"), "aidlc-product-agent");
        assert_eq!(
            fields
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect::<Vec<_>>(),
            [
                ("Stage", "practices-discovery"),
                ("Details", "done"),
                ("Agent", "aidlc-product-agent"),
            ]
        );
        assert_eq!(fields.len(), 3);
        assert!(!fields.is_empty());
    }

    #[test]
    fn reinserting_a_key_replaces_the_value_in_place() {
        let fields = AuditFields::new()
            .with(key("Stage"), "first")
            .with(key("Details"), "d")
            .with(key("Stage"), "second");
        assert_eq!(
            fields
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect::<Vec<_>>(),
            [("Stage", "second"), ("Details", "d")],
            "位置は最初のまま、値だけ差し替わる"
        );
    }

    #[test]
    fn a_timestamp_field_is_accepted_and_discarded() {
        let fields = AuditFields::new()
            .with(key("Timestamp"), "1999-01-01T00:00:00Z")
            .with(key("Stage"), "s");
        assert_eq!(
            fields.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
            ["Stage"],
            "第二の Timestamp 行は構成不能"
        );
    }

    #[test]
    fn an_empty_field_set_is_empty() {
        assert!(AuditFields::new().is_empty());
        assert_eq!(AuditFields::new().len(), 0);
        assert_eq!(AuditFields::default(), AuditFields::new());
    }

    #[test]
    fn the_value_stored_is_the_escaped_one() {
        let fields = AuditFields::new().with(key("Feedback"), "line one\nline two");
        assert_eq!(
            fields.iter().map(|(_, v)| v.as_str()).collect::<Vec<_>>(),
            ["line one\\nline two"]
        );
    }

    #[test]
    fn the_key_and_the_value_render_themselves() {
        assert_eq!(key("Stage").to_string(), "Stage");
        assert_eq!(AuditFieldValue::of("v").to_string(), "v");
    }
}
