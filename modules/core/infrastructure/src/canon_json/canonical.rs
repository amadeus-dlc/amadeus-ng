//! オブジェクトキーの並び順の決定 (BR1.1 / BR1.2)。writer の内部で使う非公開モジュール。
//!
//! 2 つの規則が重なる:
//!
//! 1. **integer-like キーの先頭寄せ (BR1.2)** — ECMAScript の所有プロパティ順序では、
//!    配列インデックスに見えるキー (0〜2^32-2 の正準十進表記) が挿入順に関係なく数値昇順で
//!    先頭に並ぶ。これは**全プロファイル**に効く。
//! 2. **再帰ソート (BR1.1)** — hash-canonical だけは残りのキーを UTF-16 コード単位順に整列する。
//!    upstream の `canonicalize` は `Object.keys().sort()` の後に `Object.fromEntries` を通すため、
//!    ソート後にもう一度 1. が適用される — つまり整列しても integer-like は先頭のままになる。

use std::cmp::Ordering;

use crate::canon_json::profile::KeyOrder;
use crate::canon_json::value::ObjectMembers;

/// 配列インデックスの上限 (ECMAScript: `ToUint32(P) != 2^32-1`)。
const MAX_ARRAY_INDEX: u64 = (u32::MAX as u64) - 1;

/// メンバを書き出す順序を、挿入順の添字列として返す。
pub(crate) fn member_order(members: &ObjectMembers, order: KeyOrder) -> Vec<usize> {
    let keys: Vec<&str> = members.iter().map(|(key, _)| key).collect();

    // 1. integer-like キーを数値昇順で先頭へ。
    let mut indexed: Vec<(usize, u64)> = keys
        .iter()
        .enumerate()
        .filter_map(|(i, key)| array_index(key).map(|n| (i, n)))
        .collect();
    indexed.sort_by_key(|(_, n)| *n);
    let mut order_indices: Vec<usize> = indexed.into_iter().map(|(i, _)| i).collect();

    // 2. 残りのキー。hash-canonical だけ UTF-16 コード単位順に整列し、他は挿入順のまま。
    let mut rest: Vec<usize> = keys
        .iter()
        .enumerate()
        .filter(|(_, key)| array_index(key).is_none())
        .map(|(i, _)| i)
        .collect();
    if order == KeyOrder::RecursiveSorted {
        // `rest` の要素は必ず `keys` の有効な添字 (enumerate 由来) — `_` 分岐には到達しない。
        rest.sort_by(|a, b| match (keys.get(*a), keys.get(*b)) {
            (Some(left), Some(right)) => utf16_cmp(left, right),
            _ => Ordering::Equal,
        });
    }
    order_indices.append(&mut rest);
    order_indices
}

/// キーが ECMAScript の配列インデックス (integer-like) なら、その数値。
///
/// 正準十進表記に限る — 先頭ゼロ (`01`)・符号 (`+1` / `-1`)・小数点 (`1.0`)・指数はいずれも
/// integer-like ではない。`0` だけは唯一の 1 文字ゼロとして許す。
fn array_index(key: &str) -> Option<u64> {
    if key == "0" {
        return Some(0);
    }
    if key.is_empty() || key.starts_with('0') || key.len() > 10 {
        return None;
    }
    if !key.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let value: u64 = key.parse().ok()?;
    if value <= MAX_ARRAY_INDEX {
        Some(value)
    } else {
        None
    }
}

/// UTF-16 コード単位順の比較 (JS の `Array#sort` 既定と同じ照合順序)。
///
/// Rust の `str` の `Ord` はコードポイント順 (= UTF-8 バイト順) であり、非 BMP 文字
/// (サロゲート `D800`〜`DFFF` で符号化される) と `U+E000`〜`U+FFFF` の比較で UTF-16 順と割れる。
fn utf16_cmp(left: &str, right: &str) -> Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

#[cfg(test)]
mod tests {
    // テストは固定長フィクスチャの添字参照を許容 (clippy.toml に相当設定が無いため file 単位で allow)。
    #![allow(clippy::indexing_slicing)]

    use super::*;
    use crate::canon_json::value::JsonValue;

    fn members_of(keys: &[&str]) -> ObjectMembers {
        let mut members = ObjectMembers::new();
        for key in keys {
            members.insert(*key, JsonValue::Null);
        }
        members
    }

    fn ordered_keys(keys: &[&str], order: KeyOrder) -> Vec<String> {
        let members = members_of(keys);
        let entries: Vec<(&str, &JsonValue)> = members.iter().collect();
        member_order(&members, order)
            .into_iter()
            .map(|i| entries[i].0.to_string())
            .collect()
    }

    #[test]
    fn integer_like_keys_come_first_in_numeric_order() {
        assert_eq!(
            ordered_keys(&["b", "10", "2", "a", "0"], KeyOrder::DeclaredOrInsertion),
            vec!["0", "2", "10", "b", "a"]
        );
    }

    #[test]
    fn non_integer_keys_keep_insertion_order_for_contract_profiles() {
        assert_eq!(
            ordered_keys(&["z", "a", "m"], KeyOrder::DeclaredOrInsertion),
            vec!["z", "a", "m"]
        );
    }

    #[test]
    fn hash_canonical_sorts_non_integer_keys_by_utf16() {
        assert_eq!(
            ordered_keys(&["b", "10", "2", "a", "0"], KeyOrder::RecursiveSorted),
            vec!["0", "2", "10", "a", "b"],
            "integer-like は先頭のまま、残りだけが整列する"
        );
        assert_eq!(
            ordered_keys(&["b", "あ", "A", "a"], KeyOrder::RecursiveSorted),
            vec!["A", "a", "b", "あ"]
        );
        assert_eq!(
            ordered_keys(&["z", ""], KeyOrder::RecursiveSorted),
            vec!["", "z"],
            "空文字列キーは整列で最小"
        );
    }

    #[test]
    fn utf16_order_differs_from_codepoint_order_for_non_bmp() {
        assert_eq!(
            ordered_keys(&["\u{fb00}", "\u{1f600}"], KeyOrder::RecursiveSorted),
            vec!["\u{1f600}", "\u{fb00}"],
            "サロゲート D83D は U+FB00 より小さい (コードポイント順なら逆)"
        );
        assert_eq!(utf16_cmp("\u{1f600}", "\u{fb00}"), Ordering::Less);
        assert_eq!(
            "\u{1f600}".cmp("\u{fb00}"),
            Ordering::Greater,
            "Rust 既定のコードポイント順とは逆になることの確認"
        );
    }

    #[test]
    fn array_index_boundary_is_two_pow_32_minus_two() {
        assert_eq!(array_index("4294967294"), Some(4_294_967_294));
        assert_eq!(
            array_index("4294967295"),
            None,
            "2^32-1 は配列インデックスではない"
        );
        assert_eq!(array_index("0"), Some(0));
        assert_eq!(array_index("9"), Some(9));
    }

    #[test]
    fn non_canonical_decimal_keys_are_not_integer_like() {
        for key in ["01", "+1", "-1", "1.0", "", " 1", "1 ", "1e2", "０"] {
            assert_eq!(array_index(key), None, "{key:?} は integer-like ではない");
        }
    }

    #[test]
    fn non_canonical_decimal_keys_sort_after_the_single_integer_like_key() {
        assert_eq!(
            ordered_keys(&["01", "1", "+1", "1.0", "-1"], KeyOrder::RecursiveSorted),
            vec!["1", "+1", "-1", "01", "1.0"]
        );
        assert_eq!(
            ordered_keys(
                &["01", "1", "+1", "1.0", "-1"],
                KeyOrder::DeclaredOrInsertion
            ),
            vec!["1", "01", "+1", "1.0", "-1"]
        );
    }
}
