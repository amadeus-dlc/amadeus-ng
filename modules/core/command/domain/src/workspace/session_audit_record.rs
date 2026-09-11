//! 実フックが観測した監査語彙と、必須項目を一体で保持する。
//!
//! セッション・委譲完了に加え、進行を変えないフック観測 (レビュー凍結の拒否) を同じ
//! 語彙で持つ — いずれも「フックが見た事実」であって workflow の遷移ではなく、投影も
//! 同じ監査ブロック描画を通る。語彙を増やすときは必須項目と許可項目をここで閉じる。
use super::{AuditFields, EventType, SessionAuditError};
#[derive(Debug, Clone, PartialEq, Eq)]
/// 種別と必須項目を検証済みの監査内容。
pub struct SessionAuditRecord {
    kind: EventType,
    fields: AuditFields,
}
impl SessionAuditRecord {
    /// 許された監査種別と項目だけから完全構築する。
    /// # Errors
    /// 別の監査語彙、必須項目欠落、未知項目がある場合。
    pub fn new(kind: EventType, fields: AuditFields) -> Result<Self, SessionAuditError> {
        let (required, allowed): (&[&str], &[&str]) = match kind {
            EventType::SessionStarted | EventType::SessionResumed => {
                (&["Source"], &["Source", "Session"])
            }
            EventType::SessionEnded => (&["Reason"], &["Reason"]),
            EventType::SessionCompacted => (
                &["Current Stage", "State Validity"],
                &["Current Stage", "State Validity"],
            ),
            EventType::SubagentCompleted => {
                (&["Agent Type"], &["Agent Type", "Agent ID", "Message"])
            }
            // レビュー凍結の拒否 — 工具・対象・ステージは拒否行の同定に要る。Unit は
            // per-unit ステージの書込みだけが名乗るので任意である
            // (upstream `aidlc-review-freeze.ts` の `...(verdict.unit ? { Unit } : {})`)。
            EventType::ReviewFreezeBlocked => (
                &["Tool", "Target", "Stage"],
                &["Tool", "Target", "Stage", "Unit"],
            ),
            // 読み取り範囲の拒否 — 差し向け記録が必ず Unit を名乗るので 4 項目とも必須で
            // ある (upstream `aidlc-reviewer-scope.ts` の `emitReviewerScopeBlocked` は
            // `Tool` / `Target` / `Stage` / `Unit` を常に渡す)。凍結側と違い Unit は任意に
            // しない — Unit の無い読み取り範囲は存在しない。
            EventType::ReviewerScopeBlocked => (
                &["Tool", "Target", "Stage", "Unit"],
                &["Tool", "Target", "Stage", "Unit"],
            ),
            _ => return Err(SessionAuditError::InvalidRecord),
        };
        if !required
            .iter()
            .all(|name| fields.iter().any(|(key, _)| key.as_str() == *name))
            || fields
                .iter()
                .any(|(key, _)| !allowed.contains(&key.as_str()))
        {
            return Err(SessionAuditError::InvalidRecord);
        }
        Ok(Self { kind, fields })
    }
    /// 保存・投影境界の種別。
    #[must_use]
    pub const fn kind(&self) -> EventType {
        self.kind
    }
    /// 保存・投影境界の順序付き項目。
    #[must_use]
    pub const fn fields(&self) -> &AuditFields {
        &self.fields
    }
}
