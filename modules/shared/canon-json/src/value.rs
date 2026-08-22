//! メモリ上の JSON 値 (挿入順保持・不変) と型付き struct からの変換点。

use std::fmt;

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
