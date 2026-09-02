//! `IntentExecutionEventId` — ドメインイベント [`IntentExecutionEvent`] 自身の識別子。
//!
//! [`IntentExecutionEvent`]: super::intent_execution_event::IntentExecutionEvent

use std::fmt;

use uuid::Uuid;

use super::intent_execution_event_id_error::IntentExecutionEventIdError;

/// 実行のドメインイベント 1 件の識別子 (Always Valid — 不正値はこの型に存在しない)。
///
/// **ドメインイベントはエンティティの一種**なので自前の識別子を持つ (オーナー裁定
/// 2026-09-02、`coding-rules/domain-object-kinds.md`)。どの集約に起きた事実かは別の
/// フィールド `aggregate_id: IntentExecutionId` が運ぶ — 集約の ID をイベントの id に
/// 流用しない。
///
/// 形は [`IntentExecutionId`](super::intent_execution_id::IntentExecutionId) と同じ
/// **UUIDv7 の正準表記**だが、**型が違えば取り違えはコンパイルで落ちる** (Entity + Id 法則)。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IntentExecutionEventId(String);

impl IntentExecutionEventId {
    /// 前後の空白を落としてから UUIDv7 の正準表記として検証する。
    ///
    /// # Errors
    ///
    /// UUIDv7 の正準表記でない綴りを拒否する。
    pub fn parse(s: &str) -> Result<IntentExecutionEventId, IntentExecutionEventIdError> {
        let trimmed = s.trim();
        let Ok(uuid) = Uuid::try_parse(trimmed) else {
            return Err(IntentExecutionEventIdError::NotCanonicalUuidV7);
        };
        if uuid.get_version_num() != 7
            || uuid.get_variant() != uuid::Variant::RFC4122
            || uuid.as_hyphenated().to_string() != trimmed
        {
            return Err(IntentExecutionEventIdError::NotCanonicalUuidV7);
        }
        Ok(IntentExecutionEventId(trimmed.to_string()))
    }

    /// 新しい識別子を採番する (UUIDv7 — 時刻順に単調な乱数)。
    #[must_use]
    pub fn generate() -> IntentExecutionEventId {
        // `Uuid::now_v7` は小文字の正準表記を生むので、この値は必ず `parse` を通る。
        // 採番をドメインに置くのは「イベント id は識別だけで、投影・ITF の答えに影響
        // しない」というオーナー裁定の例外である (`aggregate-commands.md` 2026-09-02)。
        IntentExecutionEventId(Uuid::now_v7().as_hyphenated().to_string())
    }

    /// 生の識別子文字列 (trim 済み)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IntentExecutionEventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const SAMPLE: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";

    #[test]
    fn parse_accepts_a_canonical_lowercase_uuidv7() {
        for raw in [
            SAMPLE,
            "018f3b2c-4d5e-7f60-8abc-def012345678",
            "01a02785-1bd8-76eb-aeea-5aa303ebd5b6",
            "0190aaaa-bbbb-7ccc-bddd-eeeeffff0000",
        ] {
            let id = IntentExecutionEventId::parse(raw).unwrap();
            assert_eq!(id.as_str(), raw);
            assert_eq!(id.to_string(), raw);
        }
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_before_validation() {
        let id = IntentExecutionEventId::parse("  0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000\n").unwrap();
        assert_eq!(id.as_str(), SAMPLE);
    }

    #[test]
    fn a_spelling_outside_the_canonical_form_is_rejected() {
        for raw in [
            "",
            "  \t\n",
            // 35 字 / 37 字。
            "0190aaaa-bbbb-7ccc-9ddd-eeeeffff000",
            "0190aaaa-bbbb-7ccc-9ddd-eeeeffff00000",
            // 大文字。
            "0190AAAA-bbbb-7ccc-9ddd-eeeeffff0000",
            // ハイフン位置。
            "0190aaaab-bbb-7ccc-9ddd-eeeeffff0000",
            // 非 16 進。
            "0190aaaa-bbbb-7ccc-9ddd-eeeegfff0000",
            // version nibble が 7 でない。
            "0190aaaa-bbbb-4ccc-9ddd-eeeeffff0000",
            // variant nibble が RFC ではない。
            "0190aaaa-bbbb-7ccc-cddd-eeeeffff0000",
        ] {
            assert_eq!(
                IntentExecutionEventId::parse(raw),
                Err(IntentExecutionEventIdError::NotCanonicalUuidV7),
                "accepted {raw:?}"
            );
        }
    }

    #[test]
    fn generate_mints_a_fresh_canonical_identifier_every_time() {
        let minted: Vec<IntentExecutionEventId> = (0..16)
            .map(|_| IntentExecutionEventId::generate())
            .collect();
        for id in &minted {
            // 採番した綴りは自分の parse を通る (Always Valid の閉性)。
            assert_eq!(IntentExecutionEventId::parse(id.as_str()).as_ref(), Ok(id));
        }
        let distinct: HashSet<&str> = minted.iter().map(IntentExecutionEventId::as_str).collect();
        assert_eq!(distinct.len(), minted.len(), "採番が重複した");
    }

    #[test]
    fn the_id_works_as_a_map_and_set_key() {
        let a = IntentExecutionEventId::parse(SAMPLE).unwrap();
        let b = IntentExecutionEventId::parse("  0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000 ").unwrap();
        let mut hashed = HashSet::new();
        hashed.insert(a);
        assert!(hashed.contains(&b));
    }

    #[test]
    fn the_rejection_carries_material_not_wording() {
        assert_eq!(
            IntentExecutionEventIdError::NotCanonicalUuidV7.to_string(),
            "not a canonical UUIDv7 (expected lowercase 8-4-4-4-12)"
        );
    }
}
