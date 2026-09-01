//! メモリ上の JSON 値 (挿入順保持・不変) と型付き struct からの変換点。

use serde::Serialize;

use super::number::Number;
use super::object_members::ObjectMembers;
use super::to_value_error::ToValueError;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}

#[cfg(test)]
mod to_value_tests {
    // テストは固定長フィクスチャの添字参照を許容 (clippy.toml に相当設定が無いため file 単位で
    // allow)。panic! は想定外バリアントの即時失敗という検証用途で使っており、テスト失敗の
    // シグナルとして妥当なため同様に許容する。
    #![allow(clippy::indexing_slicing, clippy::panic)]

    use std::collections::BTreeMap;

    use serde::Serialize;

    use super::*;
    use crate::canon_json::profile::SerializationProfile;
    use crate::canon_json::writer::serialize;

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
