//! `SerializationProfile` — 直列化プロファイル 3 値の閉集合 (ADR 0001 決定 2)。

use super::indent::Indent;
use super::key_order::KeyOrder;

/// 直列化プロファイル。用途ごとに体裁とキー順が決まる 3 値の閉集合。
///
/// 追加はプロファイル仕様 (ADR 0001) の改訂を伴う。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SerializationProfile {
    /// ディスク成果物と Markdown に埋め込む契約 JSON。2 スペース + 末尾改行。
    ContractPretty,
    /// stdout の 1 行 JSON と非正準ハッシュ族の入力形。空白なし。
    ContractCompact,
    /// `hashObject` 互換のハッシュ入力形。空白なし + 全オブジェクトキーの再帰ソート。
    HashCanonical,
}

impl SerializationProfile {
    /// 3 値の全列挙 (プロファイル横断のテストと網羅検査のため)。
    pub const ALL: &'static [SerializationProfile] = &[
        SerializationProfile::ContractPretty,
        SerializationProfile::ContractCompact,
        SerializationProfile::HashCanonical,
    ];

    /// インデントの単位。
    #[must_use]
    pub const fn indent(self) -> Indent {
        match self {
            SerializationProfile::ContractPretty => Indent::TwoSpaces,
            SerializationProfile::ContractCompact | SerializationProfile::HashCanonical => {
                Indent::None
            }
        }
    }

    /// ファイル末尾に改行を 1 つ付けるか。
    #[must_use]
    pub const fn trailing_newline(self) -> bool {
        matches!(self, SerializationProfile::ContractPretty)
    }

    /// オブジェクトキーの並べ方。
    #[must_use]
    pub const fn key_order(self) -> KeyOrder {
        match self {
            SerializationProfile::ContractPretty | SerializationProfile::ContractCompact => {
                KeyOrder::DeclaredOrInsertion
            }
            SerializationProfile::HashCanonical => KeyOrder::RecursiveSorted,
        }
    }

    /// このプロファイルが担う用途 (診断・ドキュメント用)。
    #[must_use]
    pub const fn purpose(self) -> &'static str {
        match self {
            SerializationProfile::ContractPretty => "ディスク成果物と Markdown に埋め込む契約 JSON",
            SerializationProfile::ContractCompact => {
                "stdout の 1 行 JSON と非正準ハッシュ族の入力形"
            }
            SerializationProfile::HashCanonical => "hashObject 互換のハッシュ入力形",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_pretty_uses_two_spaces_and_trailing_newline() {
        let profile = SerializationProfile::ContractPretty;

        assert_eq!(profile.indent(), Indent::TwoSpaces);
        assert!(profile.trailing_newline());
        assert_eq!(profile.key_order(), KeyOrder::DeclaredOrInsertion);
    }

    #[test]
    fn contract_compact_has_no_whitespace_and_no_trailing_newline() {
        let profile = SerializationProfile::ContractCompact;

        assert_eq!(profile.indent(), Indent::None);
        assert!(!profile.trailing_newline());
        assert_eq!(profile.key_order(), KeyOrder::DeclaredOrInsertion);
    }

    #[test]
    fn hash_canonical_is_the_only_recursively_sorted_profile() {
        assert_eq!(
            SerializationProfile::HashCanonical.key_order(),
            KeyOrder::RecursiveSorted
        );

        let sorted: Vec<SerializationProfile> = SerializationProfile::ALL
            .iter()
            .copied()
            .filter(|p| p.key_order() == KeyOrder::RecursiveSorted)
            .collect();
        assert_eq!(sorted, vec![SerializationProfile::HashCanonical]);
    }

    #[test]
    fn hash_canonical_shares_the_compact_layout() {
        let profile = SerializationProfile::HashCanonical;

        assert_eq!(profile.indent(), Indent::None);
        assert!(!profile.trailing_newline());
    }

    #[test]
    fn every_profile_declares_a_non_empty_purpose() {
        for profile in SerializationProfile::ALL {
            assert!(!profile.purpose().is_empty(), "{profile:?} の purpose が空");
        }
    }

    #[test]
    fn all_enumerates_the_closed_set_exactly_once() {
        assert_eq!(SerializationProfile::ALL.len(), 3);
        let mut seen = SerializationProfile::ALL.to_vec();
        seen.dedup();
        assert_eq!(seen.len(), 3, "重複のない閉集合");
    }
}
