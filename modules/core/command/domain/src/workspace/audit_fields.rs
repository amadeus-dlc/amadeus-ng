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
    /// 右側を優先して結合する。同じキーは最初の位置を保つ。
    #[must_use]
    pub fn combine(&self, other: &Self) -> Self {
        other.fold_left(self.clone(), |out, key, value| {
            out.with(key.clone(), value.as_str())
        })
    }

    /// 他方と同じキーを除き、残るフィールドの順序を保つ。
    #[must_use]
    pub fn divide(&self, other: &Self) -> Self {
        self.filter(|key, _| !other.0.iter().any(|(candidate, _)| candidate == key))
    }

    /// 条件に一致するフィールドを挿入順で返す。
    #[must_use]
    pub fn filter(
        &self,
        mut predicate: impl FnMut(&AuditFieldKey, &AuditFieldValue) -> bool,
    ) -> Self {
        self.fold_left(Self::new(), |out, key, value| {
            if predicate(key, value) {
                out.with(key.clone(), value.as_str())
            } else {
                out
            }
        })
    }

    /// キーと値を変換する。予約キーの破棄・改行の無害化・同名置換はwithと同じ。
    #[must_use]
    pub fn map(
        &self,
        mut transform: impl FnMut(&AuditFieldKey, &AuditFieldValue) -> (AuditFieldKey, String),
    ) -> Self {
        self.fold_left(Self::new(), |out, key, value| {
            let (key, value) = transform(key, value);
            out.with(key, &value)
        })
    }

    /// 挿入順に左から畳み込む。空なら初期値を返す。
    pub fn fold_left<'a, A>(
        &'a self,
        initial: A,
        mut fold: impl FnMut(A, &'a AuditFieldKey, &'a AuditFieldValue) -> A,
    ) -> A {
        self.0
            .iter()
            .fold(initial, |acc, (key, value)| fold(acc, key, value))
    }

    /// 挿入順の添字で参照する。範囲外はNone。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<(&AuditFieldKey, &AuditFieldValue)> {
        self.0.get(index).map(|(key, value)| (key, value))
    }

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

impl core_infrastructure::collections::FirstClassCollection for AuditFields {
    type Item<'a> = (&'a AuditFieldKey, &'a AuditFieldValue);
    type Filtered = Self;
    fn len(&self) -> usize {
        Self::len(self)
    }
    fn at(&self, index: usize) -> Option<Self::Item<'_>> {
        Self::at(self, index)
    }
    fn fold_left<'a, A>(&'a self, initial: A, mut fold: impl FnMut(A, Self::Item<'a>) -> A) -> A {
        Self::fold_left(self, initial, |acc, key, value| fold(acc, (key, value)))
    }
    fn filter(&self, mut predicate: impl FnMut(Self::Item<'_>) -> bool) -> Self {
        Self::filter(self, |key, value| predicate((key, value)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(raw: &str) -> AuditFieldKey {
        AuditFieldKey::parse(raw).expect("テストのキーは文法内")
    }

    #[test]
    fn collection_operations_preserve_order_escape_and_reserved_keys() {
        let left = AuditFields::new()
            .with(key("Stage"), "one")
            .with(key("Details"), "two");
        let right = AuditFields::new()
            .with(key("Stage"), "new\nline")
            .with(key("Agent"), "three");
        let combined = left.combine(&right);
        assert_eq!(combined.at(0).unwrap().1.as_str(), "new\\nline");
        assert_eq!(combined.at(2).unwrap().0.as_str(), "Agent");
        assert_eq!(
            combined.divide(&right),
            left.filter(|key, _| key.as_str() == "Details")
        );
        assert_eq!(
            left.fold_left(String::new(), |acc, key, _| acc + key.as_str()),
            "StageDetails"
        );
        assert_eq!(
            left.map(|_, value| (key("Shared"), value.as_str().to_string()))
                .at(0)
                .unwrap()
                .1
                .as_str(),
            "two"
        );
        assert!(
            left.map(|_, value| (key("Timestamp"), value.as_str().to_string()))
                .is_empty()
        );
        assert!(left.at(usize::MAX).is_none());
        assert_eq!(left.at(0).unwrap().1.as_str(), "one");
        assert!(left.divide(&left).is_empty());
        assert_eq!(left.combine(&AuditFields::new()), left);
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
