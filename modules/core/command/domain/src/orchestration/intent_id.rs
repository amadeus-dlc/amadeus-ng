//! `IntentId` — 集約 `Intent` の識別子 (`intents.json` の uuid・記録ディレクトリの id8。
//! entities.md IntentId)。

use std::fmt;

use event_store_adapter_rs::types::AggregateId;
use serde::{Deserialize, Serialize};

/// 正準形の文字数 (`8-4-4-4-12` + ハイフン 4)。
const CANONICAL_LEN: usize = 36;
/// `-` が来る 0 始まり位置。
const HYPHEN_POSITIONS: [usize; 4] = [8, 13, 18, 23];
/// version nibble の 0 始まり位置 (16 進 13 桁目)。
const VERSION_POSITION: usize = 14;
/// variant nibble の 0 始まり位置 (16 進 17 桁目)。
const VARIANT_POSITION: usize = 19;
/// UUIDv7 の version nibble。
const VERSION_NIBBLE: char = '7';
/// 本家 `AggregateId::type_name` が返す集約種別名 (この識別子が指す集約ルートの型名)。
const AGGREGATE_TYPE_NAME: &str = "Intent";

/// `intents.json` の uuid にあたる集約識別子 (Always Valid — 不正値はこの型に存在しない)。
///
/// 形は **UUIDv7 の正準表記**に限る —
/// `^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$`
/// (小文字 36 字、version nibble は `7`、variant nibble は RFC の `10xx` = `8` / `9` / `a` / `b`)。
/// 大文字・短縮形・他 version・記録ディレクトリ名の kebab 表記は受理しない (BR4.1)。
///
/// `Ord` は生文字列の辞書順。UUIDv7 の先頭 48 bit は Unix ミリ秒なので、この順序は
/// ミリ秒粒度の作成順になる (upstream 同等の性質。型としては形式だけを保証し、
/// 時刻の妥当性は検証しない — entities.md IntentId)。
///
/// serde は表現の写しである。`Serialize` は newtype として生文字列へ落ち、`Deserialize` は
/// [`IntentId::parse`] と同じ検査を通す (`try_from`) — 復号が Always Valid を破る抜け道に
/// ならないようにするためである。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct IntentId(String);

/// `IntentId::parse` が拒否する形 (材料のみ — 利用者向け文言はアダプタ層)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentIdError {
    /// 前後の空白を除くと空になる。
    Empty,
    /// 正準形の 36 字でない。
    Length {
        /// 実際の文字数 (前後の空白を除いたもの)。
        actual: usize,
    },
    /// ハイフン位置か 16 進小文字の並びが正準形に合わない。位置は 0 始まりの文字位置。
    Format {
        /// 最初に形式へ合わなかった文字の 0 始まり位置。
        position: usize,
    },
    /// version nibble が `7` でない (UUIDv7 以外)。
    Version {
        /// 実際に置かれていた nibble。
        found: char,
    },
    /// variant nibble が RFC の `10xx` (`8` / `9` / `a` / `b`) でない。
    Variant {
        /// 実際に置かれていた nibble。
        found: char,
    },
}

/// 16 進の小文字桁 (`[0-9a-f]`)。大文字は受理しない。
const fn is_lower_hex(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, 'a'..='f')
}

/// RFC の variant nibble (`10xx`)。
const fn is_variant_nibble(c: char) -> bool {
    matches!(c, '8' | '9' | 'a' | 'b')
}

impl IntentId {
    /// 前後の空白を落としてから UUIDv7 の正準表記として検証する。
    ///
    /// # Errors
    ///
    /// 空・36 字でない長さ・ハイフン位置や 16 進小文字の並びの違反・version nibble が `7`
    /// 以外・variant nibble が `8` / `9` / `a` / `b` 以外を、それぞれ拒否する。
    pub fn parse(s: &str) -> Result<IntentId, IntentIdError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(IntentIdError::Empty);
        }
        let actual = trimmed.chars().count();
        if actual != CANONICAL_LEN {
            return Err(IntentIdError::Length { actual });
        }
        for (position, c) in trimmed.chars().enumerate() {
            if HYPHEN_POSITIONS.contains(&position) {
                if c != '-' {
                    return Err(IntentIdError::Format { position });
                }
                continue;
            }
            if !is_lower_hex(c) {
                return Err(IntentIdError::Format { position });
            }
            if position == VERSION_POSITION && c != VERSION_NIBBLE {
                return Err(IntentIdError::Version { found: c });
            }
            if position == VARIANT_POSITION && !is_variant_nibble(c) {
                return Err(IntentIdError::Variant { found: c });
            }
        }
        Ok(IntentId(trimmed.to_string()))
    }

    /// 生の識別子文字列 (trim 済み)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for IntentId {
    type Error = IntentIdError;

    fn try_from(value: String) -> Result<IntentId, IntentIdError> {
        IntentId::parse(&value)
    }
}

/// 本家 event-store-adapter-rs の集約識別子契約 (ADR-010 Conformist — 契約は 1 文字も変えない)。
///
/// `value()` は我々の [tell-dont-ask] が禁じる綴りだが、**外部 trait の実装は Published
/// Language への準拠**であり、名前の所有者は本家である。したがって
/// [ubiquitous-language] §例外の作法に従い、ここに理由を書いたうえでそのまま実装する。
///
/// [tell-dont-ask]: https://github.com/amadeus-dlc/amadeus-ng/blob/main/aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/tell-dont-ask.md
/// [ubiquitous-language]: https://github.com/amadeus-dlc/amadeus-ng/blob/main/aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/ubiquitous-language.md
impl AggregateId for IntentId {
    fn type_name(&self) -> String {
        AGGREGATE_TYPE_NAME.to_string()
    }

    fn value(&self) -> String {
        self.0.clone()
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
            IntentIdError::Empty => f.write_str("empty"),
            IntentIdError::Length { actual } => {
                write!(f, "length {actual} (expected {CANONICAL_LEN})")
            }
            IntentIdError::Format { position } => {
                write!(f, "invalid character at position {position}")
            }
            IntentIdError::Version { found } => {
                write!(f, "version nibble '{found}' (expected '{VERSION_NIBBLE}')")
            }
            IntentIdError::Variant { found } => {
                write!(
                    f,
                    "variant nibble '{found}' (expected one of '8' '9' 'a' 'b')"
                )
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
        assert_eq!(IntentId::parse(""), Err(IntentIdError::Empty));
        assert_eq!(IntentId::parse("  \t\n"), Err(IntentIdError::Empty));
    }

    #[test]
    fn a_value_that_is_not_thirty_six_characters_is_rejected() {
        assert_eq!(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b"),
            Err(IntentIdError::Length { actual: 35 })
        );
        assert_eq!(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6f"),
            Err(IntentIdError::Length { actual: 37 })
        );
    }

    #[test]
    fn the_kebab_record_directory_name_is_no_longer_accepted() {
        // BR4.1: 旧形式 (記録ディレクトリ名) の受理は廃止した。長さで落ちる。
        assert_eq!(
            IntentId::parse("260822-stage1-selfhost"),
            Err(IntentIdError::Length { actual: 22 })
        );
        assert_eq!(
            IntentId::parse("u2"),
            Err(IntentIdError::Length { actual: 2 })
        );
    }

    #[test]
    fn uppercase_hex_is_rejected() {
        assert_eq!(
            IntentId::parse("01A02785-1bd8-76eb-aeea-5aa303ebd5b6"),
            Err(IntentIdError::Format { position: 2 })
        );
        assert_eq!(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303EBD5b6"),
            Err(IntentIdError::Format { position: 30 })
        );
    }

    #[test]
    fn hyphens_must_sit_at_the_canonical_positions() {
        // 8 文字目 (0 始まり位置 8) に `-` が無い。
        assert_eq!(
            IntentId::parse("01a027851-bd8-76eb-aeea-5aa303ebd5b6"),
            Err(IntentIdError::Format { position: 8 })
        );
        // 16 進が来るべき位置に `-` がある。
        assert_eq!(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303-bd5b6"),
            Err(IntentIdError::Format { position: 30 })
        );
    }

    #[test]
    fn non_hex_characters_are_rejected() {
        assert_eq!(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303gbd5b6"),
            Err(IntentIdError::Format { position: 30 })
        );
    }

    #[test]
    fn the_version_nibble_must_be_seven() {
        // UUIDv4 (13 番目の 16 進桁が 4)。
        assert_eq!(
            IntentId::parse("01a02785-1bd8-46eb-aeea-5aa303ebd5b6"),
            Err(IntentIdError::Version { found: '4' })
        );
        assert_eq!(
            IntentId::parse("01a02785-1bd8-16eb-aeea-5aa303ebd5b6"),
            Err(IntentIdError::Version { found: '1' })
        );
    }

    #[test]
    fn the_variant_nibble_must_encode_the_rfc_variant() {
        // 17 番目の 16 進桁は 8 / 9 / a / b (2 進 10xx) のみ。
        assert_eq!(
            IntentId::parse("01a02785-1bd8-76eb-ceea-5aa303ebd5b6"),
            Err(IntentIdError::Variant { found: 'c' })
        );
        assert_eq!(
            IntentId::parse("01a02785-1bd8-76eb-7eea-5aa303ebd5b6"),
            Err(IntentIdError::Variant { found: '7' })
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
    fn the_aggregate_id_contract_reports_the_type_name_and_the_raw_value() {
        let id = IntentId::parse(SAMPLE).unwrap();
        assert_eq!(id.type_name(), "Intent");
        assert_eq!(id.value(), SAMPLE);
    }

    #[test]
    fn the_identifier_round_trips_through_serde_and_an_invalid_form_is_refused() {
        let id = IntentId::parse(SAMPLE).unwrap();
        // 本家 trait の serde 境界の往復確認であり、契約 JSON (BR1.7) の直列化経路では
        // ないため、canon-json を経ない素の serde_json を使う。
        #[allow(
            clippy::disallowed_methods,
            reason = "契約 JSON ではなく serde 境界そのものの往復確認 (BR1.7 の射程外)"
        )]
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, format!("\"{SAMPLE}\""));
        assert_eq!(serde_json::from_str::<IntentId>(&json).unwrap(), id);
        // Always Valid — 復号は `parse` と同じ検査を通る (不正値はこの型に存在しない)。
        assert!(serde_json::from_str::<IntentId>("\"not-a-uuid\"").is_err());
    }

    #[test]
    fn the_rejection_carries_material_not_wording() {
        assert_eq!(IntentIdError::Empty.to_string(), "empty");
        assert_eq!(
            IntentIdError::Length { actual: 35 }.to_string(),
            "length 35 (expected 36)"
        );
        assert_eq!(
            IntentIdError::Format { position: 8 }.to_string(),
            "invalid character at position 8"
        );
        assert_eq!(
            IntentIdError::Version { found: '4' }.to_string(),
            "version nibble '4' (expected '7')"
        );
        assert_eq!(
            IntentIdError::Variant { found: 'c' }.to_string(),
            "variant nibble 'c' (expected one of '8' '9' 'a' 'b')"
        );
    }
}
