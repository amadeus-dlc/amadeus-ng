//! sha256 ダイジェスト本体 (BR1.6)。族は [`DigestFamily`](super::digest_family::DigestFamily)。

use sha2::{Digest as _, Sha256};

use crate::canon_json::profile::SerializationProfile;
use crate::canon_json::value::JsonValue;
use crate::canon_json::writer::serialize;

use super::digest_family::DigestFamily;

/// 小文字 16 進の桁。
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// sha256 ダイジェスト。族と 64 桁小文字 hex を持つ。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Digest {
    family: DigestFamily,
    hex: String,
}

impl Digest {
    /// 族と hex から組み立てる。ハッシュ計算を経ない構築を防ぐためクレート内に閉じる。
    pub(crate) const fn new(family: DigestFamily, hex: String) -> Digest {
        Digest { family, hex }
    }

    /// この値が属する族。
    #[must_use]
    pub const fn family(&self) -> DigestFamily {
        self.family
    }

    /// 64 桁の小文字 16 進 (接頭辞なし)。
    #[must_use]
    pub fn hex(&self) -> &str {
        &self.hex
    }

    /// 族ごとの表記 — 正準族は `sha256:` + hex、非正準族は hex そのもの。
    #[must_use]
    pub fn rendered(&self) -> String {
        match self.family {
            DigestFamily::CanonicalPrefixed => format!("sha256:{}", self.hex),
            DigestFamily::CompactRaw => self.hex.clone(),
        }
    }
}

/// 正準族のダイジェスト — hash-canonical 出力の UTF-8 バイト列を sha256 する (`hashObject` 互換)。
///
/// `rendered()` は `sha256:` 接頭辞付きになる。`contract_sha256`・approval fingerprint 用。
#[must_use]
pub fn hash_canonical(value: &JsonValue) -> Digest {
    let text = serialize(value, SerializationProfile::HashCanonical);
    Digest::new(DigestFamily::CanonicalPrefixed, sha256_hex(text.as_bytes()))
}

/// 非正準族のダイジェスト — contract-compact 出力の UTF-8 バイト列を sha256 する。
///
/// `rendered()` は接頭辞の無い生 hex になる。bundle hash・`directiveHash`・route hash 用。
#[must_use]
pub fn hash_compact(value: &JsonValue) -> Digest {
    let text = serialize(value, SerializationProfile::ContractCompact);
    Digest::new(DigestFamily::CompactRaw, sha256_hex(text.as_bytes()))
}

/// バイト列の sha256 を 64 桁の小文字 16 進で返す。
fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let mut hex = String::with_capacity(64);
    for byte in hasher.finalize() {
        // ニブル (0..16) の変換なので添字は必ず範囲内 — `unwrap_or` の既定分岐には到達しない。
        hex.push(
            HEX_DIGITS
                .get(usize::from(byte >> 4))
                .copied()
                .unwrap_or(b'0') as char,
        );
        hex.push(
            HEX_DIGITS
                .get(usize::from(byte & 0x0f))
                .copied()
                .unwrap_or(b'0') as char,
        );
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canon_json::value::{Number, ObjectMembers};

    const HEX: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    #[test]
    fn canonical_family_renders_with_the_sha256_prefix() {
        let digest = Digest::new(DigestFamily::CanonicalPrefixed, HEX.to_string());

        assert_eq!(digest.rendered(), format!("sha256:{HEX}"));
    }

    #[test]
    fn compact_family_renders_the_bare_hex() {
        let digest = Digest::new(DigestFamily::CompactRaw, HEX.to_string());

        assert_eq!(digest.rendered(), HEX);
    }

    #[test]
    fn hex_never_carries_the_prefix() {
        for family in [DigestFamily::CanonicalPrefixed, DigestFamily::CompactRaw] {
            let digest = Digest::new(family, HEX.to_string());
            assert_eq!(digest.hex(), HEX);
            assert!(!digest.hex().starts_with("sha256:"));
        }
    }

    #[test]
    fn family_is_reported_back() {
        assert_eq!(
            Digest::new(DigestFamily::CanonicalPrefixed, HEX.to_string()).family(),
            DigestFamily::CanonicalPrefixed
        );
        assert_eq!(
            Digest::new(DigestFamily::CompactRaw, HEX.to_string()).family(),
            DigestFamily::CompactRaw
        );
    }

    #[test]
    fn equality_includes_the_family() {
        let canonical = Digest::new(DigestFamily::CanonicalPrefixed, HEX.to_string());
        let compact = Digest::new(DigestFamily::CompactRaw, HEX.to_string());

        assert_ne!(canonical, compact, "同じ hex でも族が違えば別の値");
        assert_eq!(
            canonical,
            Digest::new(DigestFamily::CanonicalPrefixed, HEX.to_string())
        );
    }

    #[test]
    fn sha256_hex_matches_the_known_empty_string_vector() {
        assert_eq!(sha256_hex(b""), HEX);
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hash_canonical_hashes_the_sorted_serialization() {
        let mut members = ObjectMembers::new();
        members.insert("z", JsonValue::Number(Number::PosInt(1)));
        members.insert("a", JsonValue::Number(Number::PosInt(2)));
        let value = JsonValue::Object(members);

        let digest = hash_canonical(&value);

        assert_eq!(digest.family(), DigestFamily::CanonicalPrefixed);
        assert_eq!(digest.hex(), sha256_hex(br#"{"a":2,"z":1}"#));
        assert!(digest.rendered().starts_with("sha256:"));
    }

    #[test]
    fn hash_compact_hashes_the_insertion_ordered_serialization() {
        let mut members = ObjectMembers::new();
        members.insert("z", JsonValue::Number(Number::PosInt(1)));
        members.insert("a", JsonValue::Number(Number::PosInt(2)));
        let value = JsonValue::Object(members);

        let digest = hash_compact(&value);

        assert_eq!(digest.family(), DigestFamily::CompactRaw);
        assert_eq!(digest.hex(), sha256_hex(br#"{"z":1,"a":2}"#));
        assert_eq!(digest.rendered(), digest.hex());
    }

    #[test]
    fn the_two_families_differ_when_the_key_order_differs() {
        let mut members = ObjectMembers::new();
        members.insert("z", JsonValue::Number(Number::PosInt(1)));
        members.insert("a", JsonValue::Number(Number::PosInt(2)));
        let value = JsonValue::Object(members);

        assert_ne!(
            hash_canonical(&value).hex(),
            hash_compact(&value).hex(),
            "整列の有無で入力バイト列が変わる"
        );
    }

    #[test]
    fn hashing_is_deterministic_and_idempotent() {
        let value = JsonValue::Array(vec![
            JsonValue::Number(Number::Float(1.0)),
            JsonValue::String("あ".to_string()),
        ]);

        assert_eq!(hash_canonical(&value), hash_canonical(&value));
        assert_eq!(hash_compact(&value), hash_compact(&value));
    }

    #[test]
    fn rendered_length_differs_by_family() {
        let canonical = Digest::new(DigestFamily::CanonicalPrefixed, HEX.to_string());
        let compact = Digest::new(DigestFamily::CompactRaw, HEX.to_string());

        assert_eq!(compact.rendered().len(), 64);
        assert_eq!(canonical.rendered().len(), 64 + "sha256:".len());
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::*;
    use crate::canon_json::parse::parse;
    use crate::canon_json::value::arbitrary;

    proptest! {
        /// ダイジェストは常に 64 桁の小文字 16 進 (族に関わらず)。
        #[test]
        fn hex_is_always_sixty_four_lowercase_hex_digits(value in arbitrary::any_value()) {
            for digest in [hash_canonical(&value), hash_compact(&value)] {
                prop_assert_eq!(digest.hex().len(), 64);
                prop_assert!(
                    digest.hex().bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                );
            }
        }

        /// 冪等性 — 一度正準化した値を読み直しても正準族のダイジェストは変わらない。
        #[test]
        fn hash_canonical_survives_a_canonical_round_trip(value in arbitrary::finite_value()) {
            let text = serialize(&value, SerializationProfile::HashCanonical);
            let parsed = parse(&text).map_err(|e| TestCaseError::fail(e.to_string()))?;

            prop_assert_eq!(hash_canonical(&parsed), hash_canonical(&value));
        }

        /// ダイジェストは対応するプロファイルの出力バイト列から決まる。
        #[test]
        fn digests_are_the_sha256_of_the_profile_output(value in arbitrary::any_value()) {
            let canonical = serialize(&value, SerializationProfile::HashCanonical);
            let compact = serialize(&value, SerializationProfile::ContractCompact);

            let canonical_digest = hash_canonical(&value);
            let compact_digest = hash_compact(&value);

            prop_assert_eq!(canonical_digest.hex(), sha256_hex(canonical.as_bytes()));
            prop_assert_eq!(compact_digest.hex(), sha256_hex(compact.as_bytes()));
        }
    }
}
