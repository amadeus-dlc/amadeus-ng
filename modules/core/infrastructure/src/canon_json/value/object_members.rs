//! `JsonValue::Object` が運ぶメンバ列。

use super::json_value::JsonValue;

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

#[cfg(test)]
mod tests {
    use super::super::number::Number;
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
}
