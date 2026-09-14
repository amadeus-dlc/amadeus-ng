//! `CodekbEventId` — ドメインイベント [`CodekbEvent`](super::CodekbEvent) 自身の識別子。

use std::fmt;

use uuid::Uuid;

use super::codekb_event_id_error::CodekbEventIdError;

/// codekb のドメインイベント 1 件の識別子 (Always Valid)。
///
/// **ドメインイベントはエンティティの一種**なので自前の識別子を持つ
/// (`coding-rules/domain-object-kinds.md`)。どの集約に起きた事実かは別のフィールド
/// `aggregate_id: CodekbRepoId` が運ぶ — 集約の ID をイベントの id に流用しない。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodekbEventId(String);

impl CodekbEventId {
    // 呼出元は UUIDv7 の検査済み入力、または採番結果に限る。
    fn of_uuid(uuid: Uuid) -> Self {
        Self(uuid.as_hyphenated().to_string())
    }

    /// 前後の空白を落としてから UUIDv7 の正準表記として検証する。
    ///
    /// # Errors
    ///
    /// UUIDv7 の正準表記でない綴りを拒否する。
    pub fn parse(s: &str) -> Result<CodekbEventId, CodekbEventIdError> {
        let trimmed = s.trim();
        let Ok(uuid) = Uuid::try_parse(trimmed) else {
            return Err(CodekbEventIdError::NotCanonicalUuidV7);
        };
        if uuid.get_version_num() != 7
            || uuid.get_variant() != uuid::Variant::RFC4122
            || uuid.as_hyphenated().to_string() != trimmed
        {
            return Err(CodekbEventIdError::NotCanonicalUuidV7);
        }
        Ok(Self::of_uuid(uuid))
    }

    /// 新しい識別子を採番する (UUIDv7)。
    ///
    /// 採番をドメインに置くのはオーナー裁定の例外である — イベント id は識別だけで、
    /// 投影や ITF の答えに影響しない (`coding-rules/aggregate-commands.md` 2026-09-02)。
    #[must_use]
    pub fn generate() -> CodekbEventId {
        Self::of_uuid(Uuid::now_v7())
    }

    /// 生の識別子文字列。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CodekbEventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
