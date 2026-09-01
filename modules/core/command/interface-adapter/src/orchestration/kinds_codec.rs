//! `produces_kinds` の順序保存 JSON map codec (`#[serde(with = ...)]` 用の自由関数)。
//!
//! ドメインは `Vec<(String, Vec<String>)>` で**文書順を保持**する (upstream の emit 順は
//! 内容の一部 — b36)。JSON 上の形は従来どおりオブジェクト (`{ "kind": [..] }`) のまま、
//! 直列化はタプル列の並び順で map エントリを流し、復号は JSON 文書の出現順で受ける
//! (serde の streaming map は Map 型の内部順序に依らず呼出順を保つ)。

use std::fmt;

use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserializer, Serializer};

/// タプル列を JSON オブジェクトとして (並び順のまま) 直列化する。
///
/// # Errors
///
/// 下位のシリアライザが失敗した場合のみ。
pub(crate) fn serialize<S>(
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
pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Vec<(String, Vec<String>)>, D::Error>
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
