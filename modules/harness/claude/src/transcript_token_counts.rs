//! Claudeの`message.usage`が運ぶトークン量。
//!
//! 値は JS の `Number` と同じ f64 で保つ。会話履歴は外部が書く JSON であり、整数以外や
//! 非有限が来ても本家は 0 へ倒して数え続けるので、こちらも同じ土俵で扱う
//! （本家 `tools/aidlc-usage.ts:316-344` の `countsFromUsage`）。
use serde_json::Value;

/// 1 回の llm 呼出しで課金対象になる 5 つの量。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TranscriptTokenCounts {
    input: f64,
    output: f64,
    cache_create_5m: f64,
    cache_create_1h: f64,
    cache_read: f64,
}

impl TranscriptTokenCounts {
    /// 5 つの量から組む。
    #[must_use]
    pub const fn new(
        input: f64,
        output: f64,
        cache_create_5m: f64,
        cache_create_1h: f64,
        cache_read: f64,
    ) -> Self {
        Self {
            input,
            output,
            cache_create_5m,
            cache_create_1h,
            cache_read,
        }
    }

    /// `message.usage` オブジェクトから読む。
    ///
    /// 5m/1h の内訳は、その和が正のときだけ信頼する。Bedrock の converse API は内訳を
    /// 0 のまま置き、平坦な `cache_creation_input_tokens` に実数を入れるためである。
    #[must_use]
    pub fn of_usage(usage: &Value) -> Self {
        let creation = usage.get("cache_creation");
        let ephemeral_5m =
            number(creation.and_then(|value| value.get("ephemeral_5m_input_tokens")));
        let ephemeral_1h =
            number(creation.and_then(|value| value.get("ephemeral_1h_input_tokens")));
        let flat = number(usage.get("cache_creation_input_tokens"));
        let split = ephemeral_5m + ephemeral_1h;
        Self::new(
            number(usage.get("input_tokens")),
            number(usage.get("output_tokens")),
            if split > 0.0 { ephemeral_5m } else { flat },
            if split > 0.0 { ephemeral_1h } else { 0.0 },
            number(usage.get("cache_read_input_tokens")),
        )
    }

    /// 素の入力トークン。
    #[must_use]
    pub const fn input(&self) -> f64 {
        self.input
    }

    /// 生成トークン。
    #[must_use]
    pub const fn output(&self) -> f64 {
        self.output
    }

    /// 5 分 TTL のキャッシュ作成。
    #[must_use]
    pub const fn cache_create_5m(&self) -> f64 {
        self.cache_create_5m
    }

    /// 1 時間 TTL のキャッシュ作成。
    #[must_use]
    pub const fn cache_create_1h(&self) -> f64 {
        self.cache_create_1h
    }

    /// キャッシュ命中。
    #[must_use]
    pub const fn cache_read(&self) -> f64 {
        self.cache_read
    }

    /// 分割行の代表を選ぶための総量。
    #[must_use]
    pub fn magnitude(&self) -> f64 {
        self.input + self.output + self.cache_create_5m + self.cache_create_1h + self.cache_read
    }

    /// 2 つの量を足し合わせる。
    #[must_use]
    pub fn combine(&self, other: &Self) -> Self {
        Self::new(
            self.input + other.input,
            self.output + other.output,
            self.cache_create_5m + other.cache_create_5m,
            self.cache_create_1h + other.cache_create_1h,
            self.cache_read + other.cache_read,
        )
    }

    /// 1 つでも正の量があるか。
    #[must_use]
    pub fn has_any(&self) -> bool {
        self.input > 0.0
            || self.output > 0.0
            || self.cache_create_5m > 0.0
            || self.cache_create_1h > 0.0
            || self.cache_read > 0.0
    }
}

/// 有限の数値だけを採り、それ以外は 0 にする（本家 `num`）。
fn number(value: Option<&Value>) -> f64 {
    value
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(text: &str) -> Value {
        serde_json::from_str(text).expect("テストのJSON")
    }

    #[test]
    fn the_nested_split_wins_when_it_accounts_for_the_creation() {
        let counts = TranscriptTokenCounts::of_usage(&usage(
            r#"{"input_tokens":10,"output_tokens":2,"cache_read_input_tokens":3,
                "cache_creation_input_tokens":9,
                "cache_creation":{"ephemeral_5m_input_tokens":4,"ephemeral_1h_input_tokens":5}}"#,
        ));
        assert_eq!(counts.cache_create_5m(), 4.0);
        assert_eq!(counts.cache_create_1h(), 5.0);
        assert_eq!(counts.input(), 10.0);
        assert_eq!(counts.output(), 2.0);
        assert_eq!(counts.cache_read(), 3.0);
    }

    #[test]
    fn a_zeroed_split_falls_back_to_the_flat_total_as_a_five_minute_write() {
        let counts = TranscriptTokenCounts::of_usage(&usage(
            r#"{"cache_creation_input_tokens":9,
                "cache_creation":{"ephemeral_5m_input_tokens":0,"ephemeral_1h_input_tokens":0}}"#,
        ));
        assert_eq!(counts.cache_create_5m(), 9.0);
        assert_eq!(counts.cache_create_1h(), 0.0);
    }

    #[test]
    fn missing_and_non_finite_values_count_as_zero() {
        let counts = TranscriptTokenCounts::of_usage(&usage(r#"{"input_tokens":"12"}"#));
        assert_eq!(counts.magnitude(), 0.0);
        assert!(!counts.has_any());
    }

    #[test]
    fn combining_adds_every_bucket() {
        let left = TranscriptTokenCounts::new(1.0, 2.0, 3.0, 4.0, 5.0);
        let combined = left.combine(&left);
        assert_eq!(combined.magnitude(), 30.0);
        assert_eq!(combined.input(), 2.0);
    }
}
