//! 1 つの集計単位のトークン量と費用。
use core_infrastructure::canon_json::{JsonValue, Number, ObjectMembers};
use harness_claude::TranscriptTokenCounts;

/// トークン量と、値付けできた分の USD。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Totals {
    tokens: TranscriptTokenCounts,
    usd: f64,
}

impl Totals {
    /// 量と費用から組む。
    pub(crate) const fn new(tokens: TranscriptTokenCounts, usd: f64) -> Self {
        Self { tokens, usd }
    }

    /// 1 行分を足し込む。値付けできない行は量だけを足す。
    pub(crate) fn add(&mut self, tokens: TranscriptTokenCounts, usd: Option<f64>) {
        self.tokens = self.tokens.combine(&tokens);
        if let Some(usd) = usd.filter(|usd| usd.is_finite()) {
            self.usd += usd;
        }
    }

    /// JSON の `{tokens, usd}`。
    pub(crate) fn to_json(self) -> JsonValue {
        let mut tokens = ObjectMembers::new();
        tokens.insert("input", count(self.tokens.input()));
        tokens.insert("output", count(self.tokens.output()));
        tokens.insert("cacheCreate5m", count(self.tokens.cache_create_5m()));
        tokens.insert("cacheCreate1h", count(self.tokens.cache_create_1h()));
        tokens.insert("cacheRead", count(self.tokens.cache_read()));
        let mut fields = ObjectMembers::new();
        fields.insert("tokens", JsonValue::Object(tokens));
        fields.insert("usd", count(self.usd));
        JsonValue::Object(fields)
    }

    /// JSON から読む。欠けた値は 0 とする。
    pub(crate) fn of_json(value: Option<&JsonValue>) -> Self {
        let tokens = value.and_then(|value| member(value, "tokens"));
        Self::new(
            TranscriptTokenCounts::new(
                number(tokens, "input"),
                number(tokens, "output"),
                number(tokens, "cacheCreate5m"),
                number(tokens, "cacheCreate1h"),
                number(tokens, "cacheRead"),
            ),
            value.map_or(0.0, |value| number(Some(value), "usd")),
        )
    }
}

/// JS の `Number` と同じ表記で書く（整数値は小数点を持たない）。
const fn count(value: f64) -> JsonValue {
    JsonValue::Number(Number::Float(value))
}

fn member<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    match value {
        JsonValue::Object(members) => members.get(key),
        _ => None,
    }
}

fn number(value: Option<&JsonValue>, key: &str) -> f64 {
    match value.and_then(|value| member(value, key)) {
        Some(JsonValue::Number(Number::Float(number))) if number.is_finite() => *number,
        Some(JsonValue::Number(Number::PosInt(number))) => *number as f64,
        Some(JsonValue::Number(Number::NegInt(number))) => *number as f64,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_infrastructure::canon_json::{SerializationProfile, parse, serialize};

    #[test]
    fn an_empty_total_writes_the_upstream_zero_shape() {
        assert_eq!(
            serialize(
                &Totals::default().to_json(),
                SerializationProfile::ContractCompact
            ),
            r#"{"tokens":{"input":0,"output":0,"cacheCreate5m":0,"cacheCreate1h":0,"cacheRead":0},"usd":0}"#
        );
    }

    #[test]
    fn adding_a_priceless_row_keeps_the_tokens_and_leaves_the_cost() {
        let mut totals = Totals::default();
        totals.add(TranscriptTokenCounts::new(7.0, 3.0, 0.0, 0.0, 0.0), None);
        assert_eq!(
            totals,
            Totals::new(TranscriptTokenCounts::new(7.0, 3.0, 0.0, 0.0, 0.0), 0.0)
        );
    }

    #[test]
    fn a_non_finite_cost_is_ignored() {
        let mut totals = Totals::default();
        totals.add(TranscriptTokenCounts::default(), Some(f64::NAN));
        totals.add(TranscriptTokenCounts::default(), Some(0.5));
        assert_eq!(totals, Totals::new(TranscriptTokenCounts::default(), 0.5));
    }

    #[test]
    fn reading_back_a_written_total_round_trips() {
        let mut totals = Totals::default();
        totals.add(
            TranscriptTokenCounts::new(1.0, 2.0, 3.0, 4.0, 5.0),
            Some(0.25),
        );
        let text = serialize(&totals.to_json(), SerializationProfile::ContractCompact);
        let parsed = parse(&text).expect("自分で書いたJSON");
        assert_eq!(Totals::of_json(Some(&parsed)), totals);
    }

    #[test]
    fn a_missing_or_malformed_shape_reads_as_zero() {
        assert_eq!(Totals::of_json(None), Totals::default());
        let parsed = parse(r#"{"tokens":"nope","usd":"nope"}"#).expect("テストのJSON");
        assert_eq!(Totals::of_json(Some(&parsed)), Totals::default());
    }
}
