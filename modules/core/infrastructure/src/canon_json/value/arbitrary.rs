//! PBT 用の値生成器。3 系統を使い分ける (テスト専用)。
//!
//! - [`any_value`]      — 非有限数を含む。「非有限は `null` に落ちる」系の性質用。
//! - [`finite_value`]   — 非有限を含まない。テキスト水準の往復・冪等性用。
//! - [`stable_value`]   — 往復で**表現まで**保たれる部分集合。値水準の往復用。
//!
//! `stable_value` が狭いのは意図的である: `Float(1.0)` は `1` と書かれ、読み戻すと
//! `PosInt(1)` になる (表現が変わる)。integer-like キーも直列化時に先頭へ寄るため、
//! 値としての往復同一性は成り立たない。これらは表現が変わるだけで**テキストは安定**であり、
//! その安定性は `finite_value` を使うテキスト水準の性質が受け持つ。

use proptest::prelude::*;

use super::json_value::JsonValue;
use super::number::Number;
use super::object_members::ObjectMembers;

/// integer-like・非 ASCII・エスケープ対象を含む「意地の悪い」キー。
pub(crate) fn tricky_key() -> impl Strategy<Value = String> + Clone {
    prop_oneof![
        "[a-z][a-z0-9]{0,3}",
        Just(String::new()),
        Just("0".to_string()),
        Just("10".to_string()),
        Just("2".to_string()),
        Just("4294967295".to_string()),
        Just("あ".to_string()),
        Just("\u{1f600}".to_string()),
        Just("\u{fb00}".to_string()),
        Just("a\"b".to_string()),
        Just("a\nb".to_string()),
    ]
}

/// 往復で順序まで保たれるキー (integer-like を含まない)。
pub(crate) fn stable_key() -> impl Strategy<Value = String> + Clone {
    prop_oneof![
        "[a-z][a-z0-9]{0,3}",
        Just(String::new()),
        Just("あ".to_string()),
        Just("a\"b".to_string()),
    ]
}

fn any_number() -> impl Strategy<Value = Number> {
    prop_oneof![
        any::<u64>().prop_map(Number::PosInt),
        any::<i64>().prop_map(Number::NegInt),
        any::<f64>().prop_map(Number::Float),
        Just(Number::Float(f64::NAN)),
        Just(Number::Float(f64::INFINITY)),
        Just(Number::Float(f64::NEG_INFINITY)),
        Just(Number::Float(-0.0)),
    ]
}

fn finite_number() -> impl Strategy<Value = Number> {
    prop_oneof![
        any::<u64>().prop_map(Number::PosInt),
        any::<i64>().prop_map(Number::NegInt),
        any::<f64>()
            .prop_filter("有限のみ", |f| f.is_finite())
            .prop_map(Number::Float),
    ]
}

fn stable_number() -> impl Strategy<Value = Number> {
    const EXACT: u64 = 1 << 53;
    prop_oneof![
        (0u64..=EXACT).prop_map(Number::PosInt),
        #[allow(clippy::cast_possible_wrap)]
        (-(EXACT as i64)..0i64).prop_map(Number::NegInt),
        any::<f64>()
            .prop_filter(
                "有限かつ非整数 (整数値の f64 は整数として読み戻る)",
                |f| { f.is_finite() && f.fract() != 0.0 }
            )
            .prop_map(Number::Float),
    ]
}

fn tree(
    number: impl Strategy<Value = Number> + 'static,
    key: impl Strategy<Value = String> + Clone + 'static,
) -> impl Strategy<Value = JsonValue> {
    let leaf = prop_oneof![
        Just(JsonValue::Null),
        any::<bool>().prop_map(JsonValue::Bool),
        number.prop_map(JsonValue::Number),
        ".{0,6}".prop_map(JsonValue::String),
    ];
    leaf.prop_recursive(4, 40, 4, move |inner| {
        let key = key.clone();
        prop_oneof![
            proptest::collection::vec(inner.clone(), 0..4).prop_map(JsonValue::Array),
            proptest::collection::vec((key, inner), 0..4)
                .prop_map(|pairs| JsonValue::Object(pairs.into_iter().collect())),
        ]
    })
}

/// 非有限数を含みうる値。
pub(crate) fn any_value() -> impl Strategy<Value = JsonValue> {
    tree(any_number(), tricky_key())
}

/// 非有限数を含まない値。
pub(crate) fn finite_value() -> impl Strategy<Value = JsonValue> {
    tree(finite_number(), tricky_key())
}

/// 往復で表現まで保たれる値。
pub(crate) fn stable_value() -> impl Strategy<Value = JsonValue> {
    tree(stable_number(), stable_key())
}

/// キーが一意なメンバ列 (順序入れ替えの性質用)。
pub(crate) fn unique_pairs() -> impl Strategy<Value = Vec<(String, JsonValue)>> {
    proptest::collection::vec((tricky_key(), finite_value()), 0..6).prop_map(|pairs| {
        let mut seen = std::collections::BTreeSet::new();
        pairs
            .into_iter()
            .filter(|(key, _)| seen.insert(key.clone()))
            .collect()
    })
}

/// メンバ列から `JsonValue::Object` を組み立てる。
pub(crate) fn object_of(pairs: Vec<(String, JsonValue)>) -> JsonValue {
    let mut members = ObjectMembers::new();
    for (key, value) in pairs {
        members.insert(key, value);
    }
    JsonValue::Object(members)
}
