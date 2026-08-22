//! sha256 ダイジェストの 2 族 (BR1.6)。

use sha2::{Digest as _, Sha256};

use crate::profile::SerializationProfile;
use crate::value::JsonValue;
use crate::writer::serialize;

/// 小文字 16 進の桁。
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// ダイジェストの族。どのプロファイルのバイト列から計算し、どう表記するかを型で固定する。
///
/// 族を型で持つのは取り違えを防ぐため — 正準族と非正準族は同じ 64 桁 hex でも
/// 入力バイト列が違うので、混ぜると静かに不一致になる。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DigestFamily {
    /// `hashObject` 互換。hash-canonical 出力の sha256 を `sha256:` 接頭辞付きで表記する。
    /// `contract_sha256`・approval fingerprint がこの族。
    CanonicalPrefixed,
    /// contract-compact 出力の sha256 を生 hex で表記する。
    /// bundle hash・`directiveHash`・route hash・ルール配送の冪等 digest がこの族。
    CompactRaw,
}

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
        hex.push(HEX_DIGITS[usize::from(byte >> 4)] as char);
        hex.push(HEX_DIGITS[usize::from(byte & 0x0f)] as char);
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{Number, ObjectMembers};

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
