//! 挿入順を保つ文字列鍵の対応表。
//!
//! 台帳の JSON はキー順まで含めて観測契約なので、整列する `BTreeMap` は使えない。
//! 共有部品の [`core_infrastructure::canon_json::ObjectMembers`] と同じ意味論
//! （同名キーの再挿入は値を置換し、位置は最初の出現位置を保つ）を、値の型を選べる形で持つ。

/// 挿入順のメンバ列。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OrderedMap<T> {
    entries: Vec<(String, T)>,
}

impl<T> Default for OrderedMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> OrderedMap<T> {
    /// 空のメンバ列。
    pub(crate) const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// キーに対応する値。
    pub(crate) fn get(&self, key: &str) -> Option<&T> {
        self.entries
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    /// メンバを追加する。同名キーがあれば値を置換し、位置は保つ。
    pub(crate) fn insert(&mut self, key: &str, value: T) {
        match self.entries.iter_mut().find(|(name, _)| name == key) {
            Some(entry) => entry.1 = value,
            None => self.entries.push((key.to_string(), value)),
        }
    }

    /// 挿入順に左から畳み込む。
    pub(crate) fn fold_left<'a, A>(
        &'a self,
        initial: A,
        mut fold: impl FnMut(A, &'a str, &'a T) -> A,
    ) -> A {
        self.entries
            .iter()
            .fold(initial, |accumulator, (key, value)| {
                fold(accumulator, key, value)
            })
    }

    /// 挿入順のキー列。
    pub(crate) fn keys(&self) -> Vec<&str> {
        self.entries.iter().map(|(key, _)| key.as_str()).collect()
    }
}

impl<T: Default> OrderedMap<T> {
    /// キーの値を更新する。まだ無ければ既定値から作ってから更新する。
    pub(crate) fn update(&mut self, key: &str, change: impl FnOnce(&mut T)) {
        match self.entries.iter_mut().find(|(name, _)| name == key) {
            Some(entry) => change(&mut entry.1),
            None => {
                let mut value = T::default();
                change(&mut value);
                self.entries.push((key.to_string(), value));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reinserted_key_keeps_its_first_position() {
        let mut map = OrderedMap::new();
        map.insert("z", 1);
        map.insert("a", 2);
        map.insert("z", 3);
        assert_eq!(map.keys(), vec!["z", "a"]);
        assert_eq!(map.get("z"), Some(&3));
    }

    #[test]
    fn updating_an_absent_key_starts_from_the_default() {
        let mut map: OrderedMap<u32> = OrderedMap::new();
        map.update("k", |value| *value += 5);
        map.update("k", |value| *value += 2);
        assert_eq!(map.get("k"), Some(&7));
        assert_eq!(map.keys(), vec!["k"]);
    }

    #[test]
    fn an_empty_map_folds_to_its_initial_value() {
        let map: OrderedMap<u32> = OrderedMap::new();
        assert!(map.keys().is_empty());
        assert_eq!(map.fold_left(0, |sum, _, value| sum + value), 0);
        assert_eq!(map.get("missing"), None);
    }

    #[test]
    fn folding_visits_members_in_insertion_order() {
        let mut map = OrderedMap::new();
        map.insert("b", 1);
        map.insert("a", 2);
        assert_eq!(
            map.fold_left(String::new(), |mut text, key, value| {
                text.push_str(&format!("{key}{value}"));
                text
            }),
            "b1a2"
        );
    }
}
