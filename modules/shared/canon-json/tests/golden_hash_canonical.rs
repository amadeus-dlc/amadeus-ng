//! hash-canonical 受入表の全行比較 (FR7.3 の合格判定、BR2.3)。
//!
//! `tests/golden/upstream-3c3146cf/hash-canonical/cases.json` は upstream ピン `3c3146cf` の
//! `canonicalize` / `sha256` / `hashObject` を **実行して** 採った正解データである。
//! **1 行でも不一致なら FR7.3 は不合格**であり、直すのは実装であってゴールデンではない
//! (BR2.5 — ゴールデンの更新は upstream ピン更新の intent でのみ)。
//!
//! コーパスの読取には `serde_json` を直接使う。canon-json 自身の `parse` で読むと、
//! パーサの欠陥がオラクルを汚染して不一致を隠すため。

use canon_json::{
    JsonValue, Number, ObjectMembers, SerializationProfile, hash_canonical, hash_compact, parse,
    serialize,
};
use serde_json::Value;

const CORPUS_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../tests/golden/upstream-3c3146cf"
);

/// BR2.3 が要求する入力クラス。1 つでも欠けたら受入表として不完全。
const REQUIRED_CLASSES: &[&str] = &[
    "nesting",
    "integer-like-keys",
    "non-finite",
    "negative-zero",
    "exponent",
    "large-integers",
    "non-ascii",
    "escape",
    "empty",
    "struct-field-order",
    "float-integral",
];

fn corpus() -> Value {
    let path = format!("{CORPUS_DIR}/hash-canonical/cases.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("ゴールデンを読めない ({path}): {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("ゴールデンが JSON でない: {e}"))
}

fn provenance() -> Value {
    let path = format!("{CORPUS_DIR}/hash-canonical/provenance.json");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("来歴を読めない ({path}): {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("来歴が JSON でない: {e}"))
}

fn cases(corpus: &Value) -> &Vec<Value> {
    corpus["cases"]
        .as_array()
        .unwrap_or_else(|| panic!("cases は配列でなければならない"))
}

fn case_id(case: &Value) -> &str {
    case["id"]
        .as_str()
        .unwrap_or_else(|| panic!("id は文字列でなければならない"))
}

fn expected<'a>(case: &'a Value, field: &str) -> &'a str {
    case["expected"][field]
        .as_str()
        .unwrap_or_else(|| panic!("{}: expected.{field} が無い", case_id(case)))
}

/// ケースの入力を `JsonValue` として組み立てる。
///
/// JSON テキストで表せる入力は canon-json の `parse` を通す (受入は読取込みの経路で行う)。
/// NaN / ±Infinity のクラスだけは `construct` の宣言的な木から組み立てる。
fn input_value(case: &Value) -> JsonValue {
    if let Some(node) = case.get("construct") {
        return build(node, case_id(case));
    }
    let text = case["input"]
        .as_str()
        .unwrap_or_else(|| panic!("{}: input も construct も無い", case_id(case)));
    parse(text).unwrap_or_else(|e| panic!("{}: 入力を parse できない: {e}", case_id(case)))
}

fn build(node: &Value, id: &str) -> JsonValue {
    let tag = node["t"]
        .as_str()
        .unwrap_or_else(|| panic!("{id}: construct.t が無い"));
    match tag {
        "null" => JsonValue::Null,
        "bool" => JsonValue::Bool(node["v"].as_bool().unwrap_or_else(|| panic!("{id}: bool"))),
        "u64" => JsonValue::Number(Number::PosInt(
            node["v"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| panic!("{id}: u64")),
        )),
        "f64" => JsonValue::Number(Number::Float(
            match node["v"].as_str().unwrap_or_else(|| panic!("{id}: f64")) {
                "nan" => f64::NAN,
                "inf" => f64::INFINITY,
                "-inf" => f64::NEG_INFINITY,
                other => other
                    .parse()
                    .unwrap_or_else(|e| panic!("{id}: f64 {other}: {e}")),
            },
        )),
        "str" => JsonValue::String(
            node["v"]
                .as_str()
                .unwrap_or_else(|| panic!("{id}: str"))
                .to_string(),
        ),
        "arr" => JsonValue::Array(
            node["v"]
                .as_array()
                .unwrap_or_else(|| panic!("{id}: arr"))
                .iter()
                .map(|child| build(child, id))
                .collect(),
        ),
        "obj" => {
            let mut members = ObjectMembers::new();
            for pair in node["v"].as_array().unwrap_or_else(|| panic!("{id}: obj")) {
                let key = pair[0].as_str().unwrap_or_else(|| panic!("{id}: obj key"));
                members.insert(key, build(&pair[1], id));
            }
            JsonValue::Object(members)
        }
        other => panic!("{id}: 未知の construct タグ {other}"),
    }
}

/// 全行を突き合わせ、不一致を 1 件も出さないことを要求する。
fn assert_every_row<F>(field: &str, actual_of: F)
where
    F: Fn(&JsonValue) -> String,
{
    let corpus = corpus();
    let rows = cases(&corpus);
    assert!(!rows.is_empty(), "受入表が空");

    let mut failures: Vec<String> = Vec::new();
    for case in rows {
        let value = input_value(case);
        let actual = actual_of(&value);
        let want = expected(case, field);
        if actual != want {
            failures.push(format!(
                "  [{}]\n    expected: {want:?}\n    actual  : {actual:?}",
                case_id(case)
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{field}: {} / {} 行が不一致\n{}",
        failures.len(),
        rows.len(),
        failures.join("\n")
    );
}

#[test]
fn hash_canonical_output_matches_every_row() {
    assert_every_row("canonical_output", |value| {
        serialize(value, SerializationProfile::HashCanonical)
    });
}

#[test]
fn canonical_digest_matches_every_row() {
    assert_every_row("canonical_digest", |value| hash_canonical(value).rendered());
}

#[test]
fn contract_compact_output_matches_every_row() {
    assert_every_row("compact_output", |value| {
        serialize(value, SerializationProfile::ContractCompact)
    });
}

#[test]
fn compact_digest_matches_every_row() {
    assert_every_row("compact_digest_hex", |value| hash_compact(value).rendered());
}

#[test]
fn contract_pretty_output_matches_every_row() {
    assert_every_row("pretty_output", |value| {
        serialize(value, SerializationProfile::ContractPretty)
    });
}

#[test]
fn every_required_input_class_is_covered() {
    let corpus = corpus();
    let present: Vec<&str> = cases(&corpus)
        .iter()
        .filter_map(|case| case["class"].as_str())
        .collect();

    for class in REQUIRED_CLASSES {
        assert!(
            present.contains(class),
            "BR2.3 の入力クラス {class} が受入表に無い"
        );
    }
}

#[test]
fn the_corpus_carries_its_provenance() {
    let provenance = provenance();

    assert_eq!(
        provenance["upstream_commit"].as_str(),
        Some("3c3146cfd7cef33020d48e8d48d4e80d0f8c2820")
    );
    for field in ["source_url", "captured_at", "command", "bun_version"] {
        assert!(
            provenance[field].as_str().is_some_and(|v| !v.is_empty()),
            "BR2.1: provenance.{field} が無い"
        );
    }
    assert!(
        provenance["snippet"]["sha256"]
            .as_str()
            .is_some_and(|v| v.len() == 64),
        "BR2.1: 抽出スニペットの sha256 が無い"
    );

    let corpus = corpus();
    assert_eq!(
        provenance["case_count"].as_u64(),
        Some(cases(&corpus).len() as u64),
        "来歴のケース数と受入表の行数が食い違う"
    );
    for case in cases(&corpus) {
        assert!(
            case["provenance"]["commit"].as_str().is_some(),
            "{}: ケース単位の provenance が無い",
            case_id(case)
        );
    }
}
