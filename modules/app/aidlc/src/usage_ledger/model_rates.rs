//! 世代ごとの単価表と、会話履歴の `message.model` から世代鍵への正規化。
//!
//! 本家 `tools/aidlc-usage.ts` の `DEFAULT_RATES` / `loadRates` / `normalizeModel` /
//! `computeCost`。単価は 100 万トークンあたりの USD で、既定表の上に配布ファイル
//! `tools/data/model-rates.json` と環境変数 `AIDLC_MODEL_RATES` のファイルを世代鍵ごとに
//! 重ねる。未知の世代はトークンだけ数えて費用を出さない（でっち上げの数字を書かない）。
use super::ordered_map::OrderedMap;
use core_infrastructure::canon_json::{JsonValue, Number, parse};
use core_infrastructure::ecmascript;
use harness_claude::TranscriptTokenCounts;
use std::path::Path;

/// 世代鍵ごとの単価表。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModelRates {
    rows: OrderedMap<PriceRow>,
}

/// 1 世代分の単価（USD / 1e6 トークン）。
#[derive(Debug, Clone, Copy, PartialEq)]
struct PriceRow {
    input: f64,
    output: f64,
    cache_write_5m: f64,
    cache_write_1h: f64,
    cache_read: f64,
}

/// 本家の公開価格（世代ごとに 1 行。族でまとめない）。
const DEFAULT_ROWS: [(&str, PriceRow); 8] = [
    ("opus-5", PriceRow::new(5.0, 25.0, 6.25, 10.0, 0.5)),
    ("opus-4-8", PriceRow::new(5.0, 25.0, 6.25, 10.0, 0.5)),
    ("opus-4-7", PriceRow::new(5.0, 25.0, 6.25, 10.0, 0.5)),
    ("opus-4-6", PriceRow::new(5.0, 25.0, 6.25, 10.0, 0.5)),
    ("sonnet-5", PriceRow::new(3.0, 15.0, 3.75, 6.0, 0.3)),
    ("sonnet-4-6", PriceRow::new(3.0, 15.0, 3.75, 6.0, 0.3)),
    ("haiku-4-5", PriceRow::new(1.0, 5.0, 1.25, 2.0, 0.1)),
    ("fable-5", PriceRow::new(10.0, 50.0, 12.5, 20.0, 1.0)),
];

/// 世代を持たない族名の別名。残りの文字列全体が一致したときだけ引く。
const BARE_ALIASES: [(&str, &str); 4] = [
    ("opus", "opus-4-8"),
    ("sonnet", "sonnet-4-6"),
    ("haiku", "haiku-4-5"),
    ("fable", "fable-5"),
];

impl PriceRow {
    const fn new(
        input: f64,
        output: f64,
        cache_write_5m: f64,
        cache_write_1h: f64,
        cache_read: f64,
    ) -> Self {
        Self {
            input,
            output,
            cache_write_5m,
            cache_write_1h,
            cache_read,
        }
    }

    /// JSON の 1 行。欠けた欄・数値でない欄・非有限があれば行ごと捨てる。
    fn of_json(value: &JsonValue) -> Option<Self> {
        let JsonValue::Object(members) = value else {
            return None;
        };
        let field = |name: &str| match members.get(name)? {
            JsonValue::Number(Number::PosInt(number)) => Some(*number as f64),
            JsonValue::Number(Number::NegInt(number)) => Some(*number as f64),
            JsonValue::Number(Number::Float(number)) if number.is_finite() => Some(*number),
            _ => None,
        };
        Some(Self::new(
            field("input")?,
            field("output")?,
            field("cacheWrite5m")?,
            field("cacheWrite1h")?,
            field("cacheRead")?,
        ))
    }

    /// 量に単価を掛ける（本家 `computeCost` と同じ順で足す）。
    fn charge(&self, counts: TranscriptTokenCounts) -> f64 {
        (counts.input() / 1e6) * self.input
            + (counts.output() / 1e6) * self.output
            + (counts.cache_create_5m() / 1e6) * self.cache_write_5m
            + (counts.cache_create_1h() / 1e6) * self.cache_write_1h
            + (counts.cache_read() / 1e6) * self.cache_read
    }
}

impl ModelRates {
    const fn new(rows: OrderedMap<PriceRow>) -> Self {
        Self { rows }
    }

    /// 既定表（本家の公開価格）。
    pub(crate) fn defaults() -> Self {
        let mut rows = OrderedMap::new();
        for (key, row) in DEFAULT_ROWS {
            rows.insert(key, row);
        }
        Self::new(rows)
    }

    /// 既定表に配布ファイルと `AIDLC_MODEL_RATES` を重ねる。読めないファイルは何も足さない。
    pub(crate) fn load(shipped: &Path) -> Self {
        let mut rates = Self::defaults();
        rates.overlay(shipped);
        if let Some(path) = std::env::var_os("AIDLC_MODEL_RATES").filter(|path| !path.is_empty()) {
            rates.overlay(Path::new(&path));
        }
        rates
    }

    /// 単価ファイルの `rates` を世代鍵ごとに重ねる。壊れた行はその行だけ捨てる。
    fn overlay(&mut self, path: &Path) {
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let Ok(JsonValue::Object(members)) = parse(&String::from_utf8_lossy(&bytes)) else {
            return;
        };
        let Some(JsonValue::Object(rates)) = members.get("rates") else {
            return;
        };
        for (key, row) in rates.iter() {
            if let Some(row) = PriceRow::of_json(row) {
                self.rows.insert(key, row);
            }
        }
    }

    /// `message.model` を世代鍵へ正規化する。未知の形は `None`。
    ///
    /// 本家 `normalizeModel`: 別名 → `converse/` の除去 → `<region>.anthropic.` の除去 →
    /// `claude-` の要求 → 長い鍵から順に語境界で照合、の順である。
    pub(crate) fn normalize(&self, model: &str) -> Option<String> {
        let lowered = ecmascript::trim(model).to_lowercase();
        if let Some((_, alias)) = BARE_ALIASES.iter().find(|(bare, _)| *bare == lowered) {
            return Some((*alias).to_string());
        }
        let (had_converse, unprefixed) = match lowered.strip_prefix("converse/") {
            Some(rest) => (true, rest),
            None => (false, lowered.as_str()),
        };
        let bare = match strip_provider(unprefixed) {
            Some(rest) => rest,
            None if had_converse => return None,
            None => unprefixed,
        };
        let generation = bare.strip_prefix("claude-")?;
        let mut keys = self.rows.keys();
        keys.sort_by_key(|key| std::cmp::Reverse(key.len()));
        keys.into_iter()
            .find(|key| {
                generation == *key
                    || generation.starts_with(&format!("{key}-"))
                    || generation.starts_with(&format!("{key}["))
            })
            .map(str::to_string)
    }

    /// `byModel` の鍵。正規化できなければ逐語のモデル名。
    pub(crate) fn bucket(&self, model: &str) -> String {
        self.normalize(model).unwrap_or_else(|| model.to_string())
    }

    /// 量に単価を掛ける。未知の世代は `None`。
    pub(crate) fn price(&self, counts: TranscriptTokenCounts, model: &str) -> Option<f64> {
        let key = self.normalize(model)?;
        self.rows.get(&key).map(|row| row.charge(counts))
    }
}

/// 先頭の `<region>.anthropic.` または `anthropic.` を外す（本家の
/// `^(?:[a-z0-9-]+\.)?anthropic\.`）。region は `[a-z0-9-]+` に限る。
fn strip_provider(model: &str) -> Option<&str> {
    if let Some(rest) = model.strip_prefix("anthropic.") {
        return Some(rest);
    }
    let (region, rest) = model.split_once('.')?;
    let region_shape = !region.is_empty()
        && region
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if !region_shape {
        return None;
    }
    rest.strip_prefix("anthropic.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_wire_forms_normalize_to_their_generation() {
        let rates = ModelRates::defaults();
        for (wire, generation) in [
            ("claude-opus-4-8", "opus-4-8"),
            ("converse/us.anthropic.claude-opus-4-8", "opus-4-8"),
            ("global.anthropic.claude-opus-4-8[1m]", "opus-4-8"),
            ("claude-haiku-4-5-20251001", "haiku-4-5"),
            (
                "converse/au.anthropic.claude-haiku-4-5-20251001-v1:0",
                "haiku-4-5",
            ),
            (" Claude-Sonnet-5 ", "sonnet-5"),
            ("opus", "opus-4-8"),
        ] {
            assert_eq!(rates.normalize(wire).as_deref(), Some(generation), "{wire}");
        }
    }

    #[test]
    fn unknown_shapes_stay_unpriced() {
        let rates = ModelRates::defaults();
        for wire in [
            "<synthetic>",
            "",
            "claude-opus-9-9",
            "converse/opus-4-8",
            "converse/claude-opus-4-8",
            "opus-6",
            "gpt-5",
        ] {
            assert_eq!(rates.normalize(wire), None, "{wire}");
            assert_eq!(
                rates.price(TranscriptTokenCounts::new(1.0, 1.0, 0.0, 0.0, 0.0), wire),
                None,
                "{wire}"
            );
        }
    }

    #[test]
    fn pricing_follows_the_upstream_rate_math() {
        let rates = ModelRates::defaults();
        assert_eq!(
            rates.price(
                TranscriptTokenCounts::new(100.0, 20.0, 0.0, 0.0, 0.0),
                "claude-opus-4-8"
            ),
            Some(0.001)
        );
        assert_eq!(
            rates.price(
                TranscriptTokenCounts::new(1000.0, 300.0, 2000.0, 0.0, 5000.0),
                "claude-sonnet-5"
            ),
            Some(0.0165)
        );
    }

    #[test]
    fn the_bucket_key_falls_back_to_the_raw_model() {
        let rates = ModelRates::defaults();
        assert_eq!(
            rates.bucket("converse/us.anthropic.claude-opus-4-8"),
            "opus-4-8"
        );
        assert_eq!(rates.bucket("claude-opus-9-9"), "claude-opus-9-9");
        assert_eq!(rates.bucket("<synthetic>"), "<synthetic>");
        assert_eq!(rates.bucket(""), "");
    }

    #[test]
    fn an_override_file_layers_on_top_of_the_defaults() {
        let directory = tempfile::tempdir().expect("一時ディレクトリ");
        let path = directory.path().join("model-rates.json");
        std::fs::write(
            &path,
            r#"{"rates":{"opus-4-8":{"input":1,"output":1,"cacheWrite5m":1,"cacheWrite1h":1,"cacheRead":1},
                        "opus-6":{"input":2,"output":2,"cacheWrite5m":2,"cacheWrite1h":2,"cacheRead":2},
                        "broken":{"input":"x","output":1,"cacheWrite5m":1,"cacheWrite1h":1,"cacheRead":1}}}"#,
        )
        .expect("テストのファイル");
        let rates = ModelRates::load(&path);
        let counts = TranscriptTokenCounts::new(1_000_000.0, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(rates.price(counts, "claude-opus-4-8"), Some(1.0));
        assert_eq!(
            rates.normalize("claude-opus-6-20270101").as_deref(),
            Some("opus-6")
        );
        assert_eq!(rates.normalize("claude-broken"), None);
        assert_eq!(rates.price(counts, "claude-sonnet-5"), Some(3.0));
        let absent = ModelRates::load(&directory.path().join("missing.json"));
        assert_eq!(absent, ModelRates::defaults());
    }

    /// 壊れた単価ファイルは何も重ねない — JSON でない・object でない・`rates` が object でない
    /// のいずれでも既定表のままである。行の側は負数・小数を受け、object でない行は捨てる。
    #[test]
    fn malformed_override_files_add_nothing_and_odd_rows_are_dropped() {
        let directory = tempfile::tempdir().expect("一時ディレクトリ");
        let path = directory.path().join("model-rates.json");
        for body in ["not json", "[1,2,3]", r#"{"rates":[]}"#, r#"{"other":{}}"#] {
            std::fs::write(&path, body).expect("テストのファイル");
            assert_eq!(ModelRates::load(&path), ModelRates::defaults(), "{body}");
        }
        std::fs::write(
            &path,
            r#"{"rates":{"opus-4-8":{"input":-1,"output":0.5,"cacheWrite5m":0,"cacheWrite1h":0,"cacheRead":0},
                        "sonnet-5":[1,2,3]}}"#,
        )
        .expect("テストのファイル");
        let rates = ModelRates::load(&path);
        let counts = TranscriptTokenCounts::new(1_000_000.0, 1_000_000.0, 0.0, 0.0, 0.0);
        assert_eq!(rates.price(counts, "claude-opus-4-8"), Some(-0.5));
        assert_eq!(
            rates.price(counts, "claude-sonnet-5"),
            ModelRates::defaults().price(counts, "claude-sonnet-5"),
            "配列の行は捨てる"
        );
    }

    /// provider 接頭辞は `anthropic.` 単独か `<region>.anthropic.` の形だけ外す。
    #[test]
    fn provider_prefixes_are_stripped_only_in_the_two_upstream_shapes() {
        assert_eq!(
            strip_provider("anthropic.claude-opus-4-8"),
            Some("claude-opus-4-8")
        );
        assert_eq!(
            strip_provider("us-east-1.anthropic.claude-opus-4-8"),
            Some("claude-opus-4-8")
        );
        assert_eq!(
            strip_provider("US.anthropic.claude-opus-4-8"),
            None,
            "region は小文字"
        );
        assert_eq!(strip_provider("claude-opus-4-8"), None, "点が無い");
        assert_eq!(strip_provider("us.other.claude"), None);
    }
}
