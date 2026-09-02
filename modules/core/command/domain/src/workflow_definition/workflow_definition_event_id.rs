//! `WorkflowDefinitionEventId` — ドメインイベント [`WorkflowDefinitionEvent`] 自身の識別子。
//!
//! [`WorkflowDefinitionEvent`]: super::workflow_definition_event::WorkflowDefinitionEvent

use std::fmt;

use uuid::Uuid;

use super::workflow_definition_event_id_error::WorkflowDefinitionEventIdError;

/// 定義のドメインイベント 1 件の識別子 (Always Valid — 不正値はこの型に存在しない)。
///
/// **ドメインイベントはエンティティの一種**なので自前の識別子を持つ (オーナー裁定
/// 2026-09-02、`coding-rules/domain-object-kinds.md`)。どの集約に起きた事実かは別の
/// フィールド `aggregate_id: WorkflowDefinitionId` が運ぶ — 集約の ID をイベントの id に
/// 流用しない。
///
/// 綴りは **UUIDv7 の正準表記**である。集約の識別子
/// [`WorkflowDefinitionId`](super::workflow_definition_id::WorkflowDefinitionId) は系譜名
/// (`harness.json` の `name`) であって UUID ではないので、両者は形からして別物である。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkflowDefinitionEventId(String);

impl WorkflowDefinitionEventId {
    /// 前後の空白を落としてから UUIDv7 の正準表記として検証する。
    ///
    /// # Errors
    ///
    /// UUIDv7 の正準表記でない綴りを拒否する。
    pub fn parse(s: &str) -> Result<WorkflowDefinitionEventId, WorkflowDefinitionEventIdError> {
        let trimmed = s.trim();
        let Ok(uuid) = Uuid::try_parse(trimmed) else {
            return Err(WorkflowDefinitionEventIdError::NotCanonicalUuidV7);
        };
        if uuid.get_version_num() != 7
            || uuid.get_variant() != uuid::Variant::RFC4122
            || uuid.as_hyphenated().to_string() != trimmed
        {
            return Err(WorkflowDefinitionEventIdError::NotCanonicalUuidV7);
        }
        Ok(WorkflowDefinitionEventId(trimmed.to_string()))
    }

    /// 新しい識別子を採番する (UUIDv7 — 時刻順に単調な乱数)。
    #[must_use]
    pub fn generate() -> WorkflowDefinitionEventId {
        // `Uuid::now_v7` は小文字の正準表記を生むので、この値は必ず `parse` を通る。
        // 採番をドメインに置くのは「イベント id は識別だけで、投影・ITF の答えに影響
        // しない」というオーナー裁定の例外である (`aggregate-commands.md` 2026-09-02)。
        WorkflowDefinitionEventId(Uuid::now_v7().as_hyphenated().to_string())
    }

    /// 生の識別子文字列 (trim 済み)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkflowDefinitionEventId {
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
            let id = WorkflowDefinitionEventId::parse(raw).unwrap();
            assert_eq!(id.as_str(), raw);
            assert_eq!(id.to_string(), raw);
        }
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_before_validation() {
        let id =
            WorkflowDefinitionEventId::parse("  0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000\n").unwrap();
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
                WorkflowDefinitionEventId::parse(raw),
                Err(WorkflowDefinitionEventIdError::NotCanonicalUuidV7),
                "accepted {raw:?}"
            );
        }
    }

    #[test]
    fn generate_mints_a_fresh_canonical_identifier_every_time() {
        let minted: Vec<WorkflowDefinitionEventId> = (0..16)
            .map(|_| WorkflowDefinitionEventId::generate())
            .collect();
        for id in &minted {
            // 採番した綴りは自分の parse を通る (Always Valid の閉性)。
            assert_eq!(
                WorkflowDefinitionEventId::parse(id.as_str()).as_ref(),
                Ok(id)
            );
        }
        let distinct: HashSet<&str> = minted
            .iter()
            .map(WorkflowDefinitionEventId::as_str)
            .collect();
        assert_eq!(distinct.len(), minted.len(), "採番が重複した");
    }

    #[test]
    fn the_id_works_as_a_map_and_set_key() {
        let a = WorkflowDefinitionEventId::parse(SAMPLE).unwrap();
        let b =
            WorkflowDefinitionEventId::parse("  0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000 ").unwrap();
        let mut hashed = HashSet::new();
        hashed.insert(a);
        assert!(hashed.contains(&b));
    }

    #[test]
    fn the_rejection_carries_material_not_wording() {
        assert_eq!(
            WorkflowDefinitionEventIdError::NotCanonicalUuidV7.to_string(),
            "not a canonical UUIDv7 (expected lowercase 8-4-4-4-12)"
        );
    }
}
