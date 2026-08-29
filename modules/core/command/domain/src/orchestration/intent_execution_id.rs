//! `IntentExecutionId` — 集約 `IntentExecution` の識別子。
//!
//! 1 つの intent から実行は何回でも起きる (1 intent : n 実行 — オーナー裁定 2026-08-29) ので、
//! 実行は intent の識別子を自然キーとして借りられない。実行自身の同一性を担うのがこの型で
//! あり、本家 `AggregateId` を実装するのもこちらである。形は `IntentId` と同じ UUIDv7 正準
//! 表記だが、**型が違えば取り違えはコンパイルで落ちる** (Entity + Id 法則)。

use std::fmt;

use super::uuid_v7::{CANONICAL_LEN, MalformedUuidV7, VERSION_NIBBLE, parse_canonical};

/// 1 回の実行の識別子 (Always Valid — 不正値はこの型に存在しない)。
///
/// 形は **UUIDv7 の正準表記**に限る —
/// `^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$`
/// (小文字 36 字、version nibble は `7`、variant nibble は RFC の `10xx` = `8` / `9` / `a` / `b`)。
/// 検査の正本は [`super::uuid_v7`] で `IntentId` と共有する (BR4.1)。
///
/// `Ord` は生文字列の辞書順。UUIDv7 の先頭 48 bit は Unix ミリ秒なので、この順序は
/// ミリ秒粒度の作成順になる。型としては形式だけを保証し、時刻の妥当性は検証しない。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IntentExecutionId(String);

/// `IntentExecutionId::parse` が拒否する形 (材料のみ — 利用者向け文言はアダプタ層)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentExecutionIdError {
    /// 前後の空白を除くと空になる。
    Empty,
    /// 正準形の 36 字でない。
    Length {
        /// 実際の文字数 (前後の空白を除いたもの)。
        actual: usize,
    },
    /// uuid として解析できない、または解析できても正準綴り (小文字 `8-4-4-4-12`) でない
    /// (大文字・短縮形・`{braced}` など)。
    NotCanonical,
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

impl From<MalformedUuidV7> for IntentExecutionIdError {
    fn from(reason: MalformedUuidV7) -> IntentExecutionIdError {
        match reason {
            MalformedUuidV7::Empty => IntentExecutionIdError::Empty,
            MalformedUuidV7::Length { actual } => IntentExecutionIdError::Length { actual },
            MalformedUuidV7::NotCanonical => IntentExecutionIdError::NotCanonical,
            MalformedUuidV7::Version { found } => IntentExecutionIdError::Version { found },
            MalformedUuidV7::Variant { found } => IntentExecutionIdError::Variant { found },
        }
    }
}

impl IntentExecutionId {
    /// 前後の空白を落としてから UUIDv7 の正準表記として検証する。
    ///
    /// # Errors
    ///
    /// 空・36 字でない長さ・ハイフン位置や 16 進小文字の並びの違反・version nibble が `7`
    /// 以外・variant nibble が `8` / `9` / `a` / `b` 以外を、それぞれ拒否する。
    pub fn parse(s: &str) -> Result<IntentExecutionId, IntentExecutionIdError> {
        parse_canonical(s)
            .map(IntentExecutionId)
            .map_err(IntentExecutionIdError::from)
    }

    /// 生の識別子文字列 (trim 済み)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IntentExecutionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for IntentExecutionIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntentExecutionIdError::Empty => f.write_str("empty"),
            IntentExecutionIdError::Length { actual } => {
                write!(f, "length {actual} (expected {CANONICAL_LEN})")
            }
            IntentExecutionIdError::NotCanonical => {
                f.write_str("not canonical (expected lowercase 8-4-4-4-12)")
            }
            IntentExecutionIdError::Version { found } => {
                write!(f, "version nibble '{found}' (expected '{VERSION_NIBBLE}')")
            }
            IntentExecutionIdError::Variant { found } => {
                write!(
                    f,
                    "variant nibble '{found}' (expected one of '8' '9' 'a' 'b')"
                )
            }
        }
    }
}

impl std::error::Error for IntentExecutionIdError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::IntentId;
    use std::collections::{BTreeSet, HashSet};

    /// 実行識別子の標本 (形は `IntentId` と同じ UUIDv7 正準表記)。
    const SAMPLE: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";

    #[test]
    fn parse_accepts_a_lowercase_uuidv7() {
        for raw in [
            SAMPLE,
            // variant nibble は 8 / 9 / a / b のいずれでもよい (10xx)。
            "018f3b2c-4d5e-7f60-8abc-def012345678",
            "01a02785-1bd8-76eb-aeea-5aa303ebd5b6",
            "0190aaaa-bbbb-7ccc-bddd-eeeeffff0000",
        ] {
            let id = IntentExecutionId::parse(raw).unwrap();
            assert_eq!(id.as_str(), raw);
            assert_eq!(id.to_string(), raw);
        }
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_before_validation() {
        let id = IntentExecutionId::parse("  0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000\n").unwrap();
        assert_eq!(id.as_str(), SAMPLE);
        assert_eq!(id, IntentExecutionId::parse(SAMPLE).unwrap());
    }

    #[test]
    fn an_empty_or_blank_value_cannot_be_constructed() {
        assert_eq!(
            IntentExecutionId::parse(""),
            Err(IntentExecutionIdError::Empty)
        );
        assert_eq!(
            IntentExecutionId::parse("  \t\n"),
            Err(IntentExecutionIdError::Empty)
        );
    }

    #[test]
    fn a_value_that_is_not_thirty_six_characters_is_rejected() {
        assert_eq!(
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff000"),
            Err(IntentExecutionIdError::Length { actual: 35 })
        );
        assert_eq!(
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff00000"),
            Err(IntentExecutionIdError::Length { actual: 37 })
        );
    }

    #[test]
    fn uppercase_hex_is_rejected() {
        assert_eq!(
            IntentExecutionId::parse("0190AAAA-bbbb-7ccc-9ddd-eeeeffff0000"),
            Err(IntentExecutionIdError::NotCanonical)
        );
    }

    #[test]
    fn hyphens_must_sit_at_the_canonical_positions() {
        // 0 始まり位置 8 に `-` が無い。
        assert_eq!(
            IntentExecutionId::parse("0190aaaab-bbb-7ccc-9ddd-eeeeffff0000"),
            Err(IntentExecutionIdError::NotCanonical)
        );
        // 16 進が来るべき位置に `-` がある。
        assert_eq!(
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeff-f0000"),
            Err(IntentExecutionIdError::NotCanonical)
        );
    }

    #[test]
    fn non_hex_characters_are_rejected() {
        assert_eq!(
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeegfff0000"),
            Err(IntentExecutionIdError::NotCanonical)
        );
    }

    #[test]
    fn the_version_nibble_must_be_seven() {
        assert_eq!(
            IntentExecutionId::parse("0190aaaa-bbbb-4ccc-9ddd-eeeeffff0000"),
            Err(IntentExecutionIdError::Version { found: '4' })
        );
    }

    #[test]
    fn the_variant_nibble_must_encode_the_rfc_variant() {
        assert_eq!(
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-cddd-eeeeffff0000"),
            Err(IntentExecutionIdError::Variant { found: 'c' })
        );
    }

    #[test]
    fn ordering_is_the_lexicographic_order_of_the_raw_string() {
        let mut sorted: Vec<IntentExecutionId> = [
            "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000",
            "018f3b2c-4d5e-7f60-8abc-def012345678",
            "01a02785-1bd8-76eb-aeea-5aa303ebd5b6",
        ]
        .iter()
        .map(|s| IntentExecutionId::parse(s).unwrap())
        .collect();
        sorted.sort();
        let raw: Vec<&str> = sorted.iter().map(IntentExecutionId::as_str).collect();
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
        let a = IntentExecutionId::parse(SAMPLE).unwrap();
        let b = IntentExecutionId::parse("  0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000 ").unwrap();
        let mut hashed = HashSet::new();
        hashed.insert(a.clone());
        assert!(hashed.contains(&b));
        let ordered: BTreeSet<IntentExecutionId> = [a, b].into_iter().collect();
        assert_eq!(ordered.len(), 1);
    }

    #[test]
    fn the_rejection_carries_material_not_wording() {
        assert_eq!(IntentExecutionIdError::Empty.to_string(), "empty");
        assert_eq!(
            IntentExecutionIdError::Length { actual: 35 }.to_string(),
            "length 35 (expected 36)"
        );
        assert_eq!(
            IntentExecutionIdError::NotCanonical.to_string(),
            "not canonical (expected lowercase 8-4-4-4-12)"
        );
        assert_eq!(
            IntentExecutionIdError::Version { found: '4' }.to_string(),
            "version nibble '4' (expected '7')"
        );
        assert_eq!(
            IntentExecutionIdError::Variant { found: 'c' }.to_string(),
            "variant nibble 'c' (expected one of '8' '9' 'a' 'b')"
        );
    }

    #[test]
    fn an_execution_id_is_a_different_type_from_the_intent_id_even_with_the_same_text() {
        // 1 intent : n 実行になったので、実行の同一性は intent の同一性と別物である。
        // 同じ綴りでも型が違えば取り違えはコンパイルで落ちる (Entity + Id 法則)。
        let execution = IntentExecutionId::parse(SAMPLE).unwrap();
        let intent = IntentId::parse(SAMPLE).unwrap();
        assert_eq!(execution.as_str(), intent.as_str());
    }
}
