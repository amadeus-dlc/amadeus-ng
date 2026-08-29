//! メモリ上の JSON 値 (挿入順保持・不変) と型付き struct からの変換点。

use std::fmt;

use serde::Serialize;

/// メモリ上の JSON 値。オブジェクトのキー順を保持する (JS の挿入順に対応)。
///
/// `Eq` は導出しない — `Number::Float` が `f64` を持ち、`NaN != NaN` により全同値関係が
/// 成り立たないため (`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/domain-equality.md`
/// の「derive の構造的等価とドメイン同値が乖離する場合はドメイン側が勝つ」の帰結)。
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    /// JSON の `null`。
    Null,
    /// JSON の `true` / `false`。
    Bool(bool),
    /// JSON の数値。表現 (整数 / 浮動小数) を保持する。
    Number(Number),
    /// JSON の文字列 (整形式の UTF-8)。
    String(String),
    /// JSON の配列。順序は意味を持つ。
    Array(Vec<JsonValue>),
    /// JSON のオブジェクト。挿入順を保持し、キーは一意。
    Object(ObjectMembers),
}

/// JSON 数値の表現。非負は `PosInt` を優先し、負の整数は `NegInt`、小数・非有限は `Float`。
///
/// 表現の違いは同値関係に含まれる — `PosInt(1)` と `Float(1.0)` は等しくない。
/// 直列化結果は同じ `1` でも、往復 (parse → serialize) で表現が保たれることを保証したいため。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Number {
    /// 非負整数 (u64 の範囲)。
    PosInt(u64),
    /// 負整数 (i64 の範囲)。
    NegInt(i64),
    /// 浮動小数。非有限 (NaN / ±Infinity) も保持でき、直列化時に `null` へ落ちる (BR1.3)。
    Float(f64),
}

/// オブジェクトのメンバ列。挿入順を保持し、キーは一意。
///
/// 同名キーの再挿入は **値を置換し位置は最初の出現位置を維持する** (JS のオブジェクト
/// および `serde_json` の `preserve_order` と同じ意味論)。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ObjectMembers {
    entries: Vec<(String, JsonValue)>,
}

impl ObjectMembers {
    /// 空のメンバ列。
    #[must_use]
    pub const fn new() -> ObjectMembers {
        ObjectMembers {
            entries: Vec::new(),
        }
    }

    /// メンバを追加する。同名キーが既にあれば値を置換し、位置は維持して旧値を返す。
    pub fn insert(&mut self, key: impl Into<String>, value: JsonValue) -> Option<JsonValue> {
        let key = key.into();
        match self.entries.iter_mut().find(|(k, _)| *k == key) {
            Some(entry) => Some(std::mem::replace(&mut entry.1, value)),
            None => {
                self.entries.push((key, value));
                None
            }
        }
    }

    /// キーに対応する値。
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// メンバ数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// メンバが 1 つも無いか。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 挿入順のメンバ列。
    pub fn iter(&self) -> impl Iterator<Item = (&str, &JsonValue)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }
}

impl FromIterator<(String, JsonValue)> for ObjectMembers {
    fn from_iter<T: IntoIterator<Item = (String, JsonValue)>>(iter: T) -> ObjectMembers {
        let mut members = ObjectMembers::new();
        for (key, value) in iter {
            members.insert(key, value);
        }
        members
    }
}

/// 型付き値を `JsonValue` へ写す — 契約経路の唯一の変換点 (BR1.7)。
///
/// struct はフィールドの**宣言順**、動的マップは挿入順で `JsonValue::Object` になる
/// (`serde_json` の `preserve_order` が前提 — BR1.8)。非有限の `f64` は serde の既定に
/// 従って `null` になる (直列化時の BR1.3 と同じ結果)。
///
/// # Errors
///
/// serde の直列化が失敗したとき (文字列にできないキーを持つマップ、`Serialize` 実装が
/// エラーを返した場合など) に [`ToValueError`] を返す。
pub fn to_value<T: Serialize + ?Sized>(value: &T) -> Result<JsonValue, ToValueError> {
    // BR1.7 / ADR 0001 決定 5: ワークスペース唯一の `serde_json::to_value` 呼出点。
    // 契約経路の入口をここ 1 か所に閉じ、他クレートからの直接呼び出しは
    // `clippy.toml` の disallowed-methods が拒否する。
    #[allow(clippy::disallowed_methods)]
    let raw = serde_json::to_value(value)
        .map_err(|error| ToValueError::Serialization(error.to_string()))?;
    Ok(crate::canon_json::parse::from_serde(raw))
}

/// `to_value` が型付き値を `JsonValue` へ写せなかったときの理由。
///
/// 文言はアダプタ層 (message-catalog) が付ける — 本型は材料だけを保持する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToValueError {
    /// serde の直列化が失敗した (非文字列キーのマップ、`Serialize` 実装のエラー等)。
    Serialization(String),
}

impl ToValueError {
    /// 失敗の詳細 (serde が返した文言)。
    #[must_use]
    pub fn detail(&self) -> &str {
        match self {
            ToValueError::Serialization(detail) => detail,
        }
    }
}

impl fmt::Display for ToValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToValueError::Serialization(detail) => {
                write!(f, "型付き値を JsonValue へ変換できない: {detail}")
            }
        }
    }
}

/// `?` で他のエラー型へ持ち上げられるようにする。`source()` は返さない —
/// 原因は serde が返した文言として畳み込んであるため。
impl std::error::Error for ToValueError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &str) -> JsonValue {
        JsonValue::String(text.to_string())
    }

    #[test]
    fn object_members_preserves_insertion_order() {
        let mut members = ObjectMembers::new();
        members.insert("z", s("1"));
        members.insert("a", s("2"));
        members.insert("m", s("3"));

        let keys: Vec<&str> = members.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec!["z", "a", "m"]);
    }

    #[test]
    fn object_members_replaces_value_and_keeps_position() {
        let mut members = ObjectMembers::new();
        members.insert("a", JsonValue::Number(Number::PosInt(1)));
        members.insert("b", JsonValue::Number(Number::PosInt(2)));
        let previous = members.insert("a", JsonValue::Number(Number::PosInt(3)));

        assert_eq!(previous, Some(JsonValue::Number(Number::PosInt(1))));
        let keys: Vec<&str> = members.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec!["a", "b"], "位置は最初の出現位置を維持する");
        assert_eq!(
            members.get("a"),
            Some(&JsonValue::Number(Number::PosInt(3))),
            "値は後勝ち"
        );
    }

    #[test]
    fn object_members_get_returns_none_for_absent_key() {
        let mut members = ObjectMembers::new();
        members.insert("a", JsonValue::Null);

        assert_eq!(members.get("a"), Some(&JsonValue::Null));
        assert_eq!(members.get("missing"), None);
    }

    #[test]
    fn object_members_len_and_is_empty_track_unique_keys() {
        let mut members = ObjectMembers::new();
        assert_eq!(members.len(), 0);
        assert!(members.is_empty());

        members.insert("a", JsonValue::Null);
        members.insert("b", JsonValue::Null);
        members.insert("a", JsonValue::Bool(true));

        assert_eq!(members.len(), 2, "同名の再挿入は件数を増やさない");
        assert!(!members.is_empty());
    }

    #[test]
    fn object_members_accepts_empty_string_key() {
        let mut members = ObjectMembers::new();
        members.insert("", JsonValue::Number(Number::PosInt(2)));
        members.insert("z", JsonValue::Number(Number::PosInt(1)));

        let keys: Vec<&str> = members.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec!["", "z"]);
    }

    #[test]
    fn object_members_from_iter_applies_last_wins() {
        let members: ObjectMembers = vec![
            ("a".to_string(), JsonValue::Number(Number::PosInt(1))),
            ("b".to_string(), JsonValue::Number(Number::PosInt(2))),
            ("a".to_string(), JsonValue::Number(Number::PosInt(3))),
        ]
        .into_iter()
        .collect();

        let keys: Vec<&str> = members.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec!["a", "b"]);
        assert_eq!(
            members.get("a"),
            Some(&JsonValue::Number(Number::PosInt(3)))
        );
    }

    #[test]
    fn json_value_equality_distinguishes_number_representation() {
        assert_eq!(
            JsonValue::Number(Number::PosInt(1)),
            JsonValue::Number(Number::PosInt(1))
        );
        assert_ne!(
            JsonValue::Number(Number::PosInt(1)),
            JsonValue::Number(Number::Float(1.0)),
            "表現の違いは同値関係に含まれる"
        );
        assert_ne!(
            JsonValue::Number(Number::PosInt(0)),
            JsonValue::Number(Number::NegInt(0))
        );
    }

    #[test]
    fn json_value_equality_is_structural_for_containers() {
        let mut left = ObjectMembers::new();
        left.insert("a", JsonValue::Array(vec![JsonValue::Bool(true)]));
        let mut right = ObjectMembers::new();
        right.insert("a", JsonValue::Array(vec![JsonValue::Bool(true)]));

        assert_eq!(JsonValue::Object(left.clone()), JsonValue::Object(right));

        let mut reordered = ObjectMembers::new();
        reordered.insert("b", JsonValue::Null);
        reordered.insert("a", JsonValue::Array(vec![JsonValue::Bool(true)]));
        assert_ne!(JsonValue::Object(left), JsonValue::Object(reordered));
    }

    #[test]
    fn json_value_nan_is_not_equal_to_itself() {
        let nan = JsonValue::Number(Number::Float(f64::NAN));
        assert_ne!(nan, nan.clone(), "NaN の非反射性により Eq は導出できない");
    }

    #[test]
    fn to_value_error_exposes_detail_and_display() {
        let error = ToValueError::Serialization("key must be a string".to_string());

        assert_eq!(error.detail(), "key must be a string");
        assert_eq!(
            error.to_string(),
            "型付き値を JsonValue へ変換できない: key must be a string"
        );
    }
}

/// PBT 用の値生成器。3 系統を使い分ける (テスト専用)。
///
/// - [`any_value`]      — 非有限数を含む。「非有限は `null` に落ちる」系の性質用。
/// - [`finite_value`]   — 非有限を含まない。テキスト水準の往復・冪等性用。
/// - [`stable_value`]   — 往復で**表現まで**保たれる部分集合。値水準の往復用。
///
/// `stable_value` が狭いのは意図的である: `Float(1.0)` は `1` と書かれ、読み戻すと
/// `PosInt(1)` になる (表現が変わる)。integer-like キーも直列化時に先頭へ寄るため、
/// 値としての往復同一性は成り立たない。これらは表現が変わるだけで**テキストは安定**であり、
/// その安定性は `finite_value` を使うテキスト水準の性質が受け持つ。
#[cfg(test)]
pub(crate) mod arbitrary {
    use proptest::prelude::*;

    use super::{JsonValue, Number, ObjectMembers};

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
}

#[cfg(test)]
mod to_value_tests {
    // テストは固定長フィクスチャの添字参照を許容 (clippy.toml に相当設定が無いため file 単位で
    // allow)。panic! は想定外バリアントの即時失敗という検証用途で使っており、テスト失敗の
    // シグナルとして妥当なため同様に許容する。
    #![allow(clippy::indexing_slicing, clippy::panic)]

    use std::collections::BTreeMap;

    use serde::Serialize;

    use crate::canon_json::profile::SerializationProfile;
    use crate::canon_json::writer::serialize;
    use super::*;

    #[derive(Serialize)]
    struct Directive {
        kind: String,
        stage: String,
        agent: Option<String>,
        consumes: Vec<String>,
        depth: u32,
    }

    #[derive(Serialize)]
    struct Nested {
        outer: String,
        inner: Inner,
    }

    #[derive(Serialize)]
    struct Inner {
        z: u8,
        a: u8,
    }

    fn keys(value: &JsonValue) -> Vec<String> {
        match value {
            JsonValue::Object(members) => members.iter().map(|(k, _)| k.to_string()).collect(),
            other => panic!("object を期待したが {other:?}"),
        }
    }

    #[test]
    fn struct_fields_keep_their_declaration_order() {
        let directive = Directive {
            kind: "run-stage".to_string(),
            stage: "domain-design".to_string(),
            agent: Some("aidlc-architect-agent".to_string()),
            consumes: vec!["a".to_string()],
            depth: 3,
        };

        let value = to_value(&directive).unwrap();

        assert_eq!(
            keys(&value),
            vec!["kind", "stage", "agent", "consumes", "depth"],
            "アルファベット順ではなく宣言順"
        );
    }

    #[test]
    fn nested_structs_keep_their_own_declaration_order() {
        let value = to_value(&Nested {
            outer: "o".to_string(),
            inner: Inner { z: 1, a: 2 },
        })
        .unwrap();

        assert_eq!(keys(&value), vec!["outer", "inner"]);
        let JsonValue::Object(members) = &value else {
            panic!("object を期待した");
        };
        let inner = members.get("inner").unwrap();
        assert_eq!(keys(inner), vec!["z", "a"]);
    }

    #[test]
    fn none_is_serialized_as_null_by_default() {
        let value = to_value(&Directive {
            kind: "done".to_string(),
            stage: String::new(),
            agent: None,
            consumes: Vec::new(),
            depth: 0,
        })
        .unwrap();

        let JsonValue::Object(members) = &value else {
            panic!("object を期待した");
        };
        assert_eq!(
            members.get("agent"),
            Some(&JsonValue::Null),
            "serde の既定では None のフィールドも null として現れる"
        );
        assert_eq!(members.get("consumes"), Some(&JsonValue::Array(Vec::new())));
    }

    #[test]
    fn numbers_map_onto_the_representation_preserving_variants() {
        let value = to_value(&(1u64, -1i64, 1.5f64, 1.0f64)).unwrap();
        let JsonValue::Array(items) = value else {
            panic!("array を期待した");
        };

        assert_eq!(items[0], JsonValue::Number(Number::PosInt(1)));
        assert_eq!(items[1], JsonValue::Number(Number::NegInt(-1)));
        assert_eq!(items[2], JsonValue::Number(Number::Float(1.5)));
        assert_eq!(items[3], JsonValue::Number(Number::Float(1.0)));
    }

    #[test]
    fn dynamic_maps_keep_their_insertion_order() {
        let mut map = serde_json::Map::new();
        map.insert("z".to_string(), serde_json::Value::from(1));
        map.insert("a".to_string(), serde_json::Value::from(2));

        let value = to_value(&map).unwrap();

        assert_eq!(keys(&value), vec!["z", "a"]);
    }

    #[test]
    fn maps_with_non_string_keys_are_rejected() {
        let mut map: BTreeMap<(u8, u8), u8> = BTreeMap::new();
        map.insert((1, 2), 3);

        let error = to_value(&map).unwrap_err();

        assert!(!error.detail().is_empty());
        assert!(
            error
                .to_string()
                .starts_with("型付き値を JsonValue へ変換できない"),
            "実際: {error}"
        );
    }

    #[test]
    fn to_value_feeds_the_serializer_end_to_end() {
        let directive = Directive {
            kind: "run-stage".to_string(),
            stage: "s".to_string(),
            agent: None,
            consumes: vec!["a".to_string(), "b".to_string()],
            depth: 2,
        };

        let value = to_value(&directive).unwrap();

        assert_eq!(
            serialize(&value, SerializationProfile::ContractCompact),
            r#"{"kind":"run-stage","stage":"s","agent":null,"consumes":["a","b"],"depth":2}"#
        );
        assert_eq!(
            serialize(&value, SerializationProfile::HashCanonical),
            r#"{"agent":null,"consumes":["a","b"],"depth":2,"kind":"run-stage","stage":"s"}"#
        );
    }
}
