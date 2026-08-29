//! `IntentId` — 集約 `IntentExecution` の識別子 (`intents.json` の uuid・記録ディレクトリの id8。
//! entities.md IntentId)。

use std::fmt;

use uuid::Uuid;

/// `intents.json` の uuid にあたる集約識別子 (Always Valid — 不正値はこの型に存在しない)。
///
/// 形は **UUIDv7 の正準表記**に限る —
/// `^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$`
/// (小文字 36 字、version nibble は `7`、variant nibble は RFC の `10xx` = `8` / `9` / `a` / `b`)。
/// 大文字・短縮形・他 version・記録ディレクトリ名の kebab 表記は受理しない (BR4.1)。
/// 解析は uuid クレート、正準綴りの逐語検査 (正規化せず拒否) はこの `parse` が行う
/// (オーナー裁定 2026-08-30 — UUID 専用の自作モジュールを持たない)。
///
/// `Ord` は生文字列の辞書順。UUIDv7 の先頭 48 bit は Unix ミリ秒なので、この順序は
/// ミリ秒粒度の作成順になる (upstream 同等の性質。型としては形式だけを保証し、
/// 時刻の妥当性は検証しない — entities.md IntentId)。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IntentId(String);

/// `IntentId::parse` が拒否する形 (材料のみ — 利用者向け文言はアダプタ層)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentIdError {
    /// UUIDv7 の正準表記 (小文字 `8-4-4-4-12`・version `7`・RFC variant) でない。
    NotCanonicalUuidV7,
}

impl IntentId {
    /// 前後の空白を落としてから UUIDv7 の正準表記として検証する。
    ///
    /// `Uuid::try_parse` は寛容 (大文字・`{braced}`・URN・短縮形も受理) なので、再直列化した
    /// 正準表記と入力の逐語一致で「正規化せず拒否」(BR4.1) を実現する。
    ///
    /// # Errors
    ///
    /// UUIDv7 の正準表記でない綴りを拒否する。
    pub fn parse(s: &str) -> Result<IntentId, IntentIdError> {
        let trimmed = s.trim();
        let Ok(uuid) = Uuid::try_parse(trimmed) else {
            return Err(IntentIdError::NotCanonicalUuidV7);
        };
        if uuid.get_version_num() != 7
            || uuid.get_variant() != uuid::Variant::RFC4122
            || uuid.as_hyphenated().to_string() != trimmed
        {
            return Err(IntentIdError::NotCanonicalUuidV7);
        }
        Ok(IntentId(trimmed.to_string()))
    }

    /// 生の識別子文字列 (trim 済み)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IntentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for IntentIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntentIdError::NotCanonicalUuidV7 => {
                f.write_str("not a canonical UUIDv7 (expected lowercase 8-4-4-4-12)")
            }
        }
    }
}

impl std::error::Error for IntentIdError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashSet};

    /// `intents.json` の実データ (11 号 §2.2 / entities.md IntentId)。
    const SAMPLE: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

    #[test]
    fn parse_accepts_a_lowercase_uuidv7() {
        for raw in [
            SAMPLE,
            // variant nibble は 8 / 9 / a / b のいずれでもよい (10xx)。
            "018f3b2c-4d5e-7f60-8abc-def012345678",
            "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000",
            "0190aaaa-bbbb-7ccc-bddd-eeeeffff0000",
        ] {
            let id = IntentId::parse(raw).unwrap();
            assert_eq!(id.as_str(), raw);
            assert_eq!(id.to_string(), raw);
        }
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_before_validation() {
        let id = IntentId::parse("  01a02785-1bd8-76eb-aeea-5aa303ebd5b6\n").unwrap();
        assert_eq!(id.as_str(), SAMPLE);
        assert_eq!(id, IntentId::parse(SAMPLE).unwrap());
    }

    #[test]
    fn an_empty_or_blank_value_cannot_be_constructed() {
        assert_eq!(IntentId::parse(""), Err(IntentIdError::NotCanonicalUuidV7));
        assert_eq!(
            IntentId::parse("  \t\n"),
            Err(IntentIdError::NotCanonicalUuidV7)
        );
    }

    #[test]
    fn a_value_that_is_not_thirty_six_characters_is_rejected() {
        assert_eq!(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b"),
            Err(IntentIdError::NotCanonicalUuidV7)
        );
        assert_eq!(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6f"),
            Err(IntentIdError::NotCanonicalUuidV7)
        );
    }

    #[test]
    fn the_kebab_record_directory_name_is_no_longer_accepted() {
        // BR4.1: 旧形式 (記録ディレクトリ名) の受理は廃止した。長さで落ちる。
        assert_eq!(
            IntentId::parse("260822-stage1-selfhost"),
            Err(IntentIdError::NotCanonicalUuidV7)
        );
        assert_eq!(
            IntentId::parse("u2"),
            Err(IntentIdError::NotCanonicalUuidV7)
        );
    }

    #[test]
    fn uppercase_hex_is_rejected() {
        assert_eq!(
            IntentId::parse("01A02785-1bd8-76eb-aeea-5aa303ebd5b6"),
            Err(IntentIdError::NotCanonicalUuidV7)
        );
        assert_eq!(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303EBD5b6"),
            Err(IntentIdError::NotCanonicalUuidV7)
        );
    }

    #[test]
    fn hyphens_must_sit_at_the_canonical_positions() {
        // 8 文字目 (0 始まり位置 8) に `-` が無い。
        assert_eq!(
            IntentId::parse("01a027851-bd8-76eb-aeea-5aa303ebd5b6"),
            Err(IntentIdError::NotCanonicalUuidV7)
        );
        // 16 進が来るべき位置に `-` がある。
        assert_eq!(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303-bd5b6"),
            Err(IntentIdError::NotCanonicalUuidV7)
        );
    }

    #[test]
    fn non_hex_characters_are_rejected() {
        assert_eq!(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303gbd5b6"),
            Err(IntentIdError::NotCanonicalUuidV7)
        );
    }

    #[test]
    fn the_version_nibble_must_be_seven() {
        // UUIDv4 (13 番目の 16 進桁が 4)。
        assert_eq!(
            IntentId::parse("01a02785-1bd8-46eb-aeea-5aa303ebd5b6"),
            Err(IntentIdError::NotCanonicalUuidV7)
        );
        assert_eq!(
            IntentId::parse("01a02785-1bd8-16eb-aeea-5aa303ebd5b6"),
            Err(IntentIdError::NotCanonicalUuidV7)
        );
    }

    #[test]
    fn the_variant_nibble_must_encode_the_rfc_variant() {
        // 17 番目の 16 進桁は 8 / 9 / a / b (2 進 10xx) のみ。
        assert_eq!(
            IntentId::parse("01a02785-1bd8-76eb-ceea-5aa303ebd5b6"),
            Err(IntentIdError::NotCanonicalUuidV7)
        );
        assert_eq!(
            IntentId::parse("01a02785-1bd8-76eb-7eea-5aa303ebd5b6"),
            Err(IntentIdError::NotCanonicalUuidV7)
        );
    }

    #[test]
    fn ordering_is_the_lexicographic_order_of_the_raw_string() {
        // UUIDv7 の先頭 48 bit は Unix ミリ秒なので、文字列順が作成順になる。
        let mut sorted: Vec<IntentId> = [
            "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000",
            "018f3b2c-4d5e-7f60-8abc-def012345678",
            "01a02785-1bd8-76eb-aeea-5aa303ebd5b6",
        ]
        .iter()
        .map(|s| IntentId::parse(s).unwrap())
        .collect();
        sorted.sort();
        let raw: Vec<&str> = sorted.iter().map(IntentId::as_str).collect();
        assert_eq!(
            raw,
            [
                "018f3b2c-4d5e-7f60-8abc-def012345678",
                "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000",
                "01a02785-1bd8-76eb-aeea-5aa303ebd5b6",
            ]
        );
    }

    #[test]
    fn the_id_works_as_a_map_and_set_key() {
        let a = IntentId::parse(SAMPLE).unwrap();
        let b = IntentId::parse("  01a02785-1bd8-76eb-aeea-5aa303ebd5b6 ").unwrap();
        let mut hashed = HashSet::new();
        hashed.insert(a.clone());
        assert!(hashed.contains(&b));
        let ordered: BTreeSet<IntentId> = [a, b].into_iter().collect();
        assert_eq!(ordered.len(), 1);
    }

    #[test]
    fn the_rejection_carries_material_not_wording() {
        assert_eq!(
            IntentIdError::NotCanonicalUuidV7.to_string(),
            "not a canonical UUIDv7 (expected lowercase 8-4-4-4-12)"
        );
    }
}
