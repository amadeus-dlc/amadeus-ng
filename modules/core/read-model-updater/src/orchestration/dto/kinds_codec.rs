//! `produces_kinds` の順序保存 JSON map codec (`#[serde(with = ...)]` 用の自由関数、**読む側**)。
//!
//! ドメインは `Vec<(String, Vec<String>)>` で**文書順を保持**する (upstream の emit 順は
//! 内容の一部 — b36)。JSON 上の形は書く側と同じオブジェクト (`{ "kind": [..] }`) であり、
//! 復号は JSON 文書の出現順で受ける (serde の streaming map は Map 型の内部順序に依らず
//! 呼出順を保つ)。
//!
//! 書く側 (command interface-adapter の `kinds_codec`) と**共有しない**同形の別モジュール
//! である (`coding-rules/cqrs-boundaries.md` — 側ごと専用化)。両者が同じバイトを読み書き
//! することは横断適合テストが固定する。

use std::fmt;

use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserializer, Serializer};

/// タプル列を JSON オブジェクトとして (並び順のまま) 直列化する。
///
/// # Errors
///
/// 下位のシリアライザが失敗した場合のみ。
pub(super) fn serialize<S>(
    kinds: &[(String, Vec<String>)],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = serializer.serialize_map(Some(kinds.len()))?;
    for (kind, artifacts) in kinds {
        map.serialize_entry(kind, artifacts)?;
    }
    map.end()
}

/// JSON オブジェクトを出現順のタプル列として復号する。
///
/// # Errors
///
/// JSON がオブジェクトでない・値が文字列配列でない場合。
pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Vec<(String, Vec<String>)>, D::Error>
where
    D: Deserializer<'de>,
{
    struct KindsVisitor;

    impl<'de> Visitor<'de> for KindsVisitor {
        type Value = Vec<(String, Vec<String>)>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a map of kind -> artifact list")
        }

        fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut out = Vec::with_capacity(access.size_hint().unwrap_or(0));
            while let Some(entry) = access.next_entry::<String, Vec<String>>()? {
                out.push(entry);
            }
            Ok(out)
        }
    }

    deserializer.deserialize_map(KindsVisitor)
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Holder {
        #[serde(with = "super")]
        kinds: Vec<(String, Vec<String>)>,
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
    )]
    #[test]
    fn the_document_order_survives_a_round_trip() {
        // `b` が `a` より先 — BTreeMap なら並び替わる順を、タプル列はそのまま保つ。
        let holder: Holder = serde_json::from_str(r#"{"kinds":{"b":["x"],"a":[]}}"#).unwrap();
        assert_eq!(
            holder.kinds,
            vec![
                ("b".to_string(), vec!["x".to_string()]),
                ("a".to_string(), Vec::new()),
            ]
        );
        assert_eq!(
            serde_json::to_string(&holder).unwrap(),
            r#"{"kinds":{"b":["x"],"a":[]}}"#
        );
    }

    #[test]
    fn a_non_map_is_rejected_with_the_expected_shape_in_the_message() {
        let error = serde_json::from_str::<Holder>(r#"{"kinds":[1]}"#).unwrap_err();
        assert!(
            error.to_string().contains("a map of kind -> artifact list"),
            "{error}"
        );
    }
}
