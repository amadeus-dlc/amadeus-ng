//! 保護された計画承認が使うセッション名と公開キー。
use super::PlanApprovalError;
/// 元のセッション名は保持し、公開キーが衝突した場合も名前を再照合する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanSession {
    raw: String,
    key: String,
}
impl PlanSession {
    /// セッションを保護された承認のキーへ写す。
    /// # Errors
    /// 空白のみ、または公開キーが空になる場合。
    pub fn new(raw: String) -> Result<Self, PlanApprovalError> {
        if core_infrastructure::ecmascript::trim(&raw).is_empty() {
            return Err(PlanApprovalError::new(
                "Plan Approval challenge requires a nonblank session",
            ));
        }
        let mut normalized = String::new();
        let mut invalid = false;
        for character in raw.chars() {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
                normalized.push(character);
                invalid = false;
            } else if !invalid {
                normalized.push('-');
                invalid = true;
            }
        }
        let key: String = normalized.trim_matches('-').chars().take(96).collect();
        if key.is_empty() {
            return Err(PlanApprovalError::new(
                "Plan Approval challenge requires a nonblank session",
            ));
        }
        Ok(Self { raw, key })
    }
    /// セッション名の原文。
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }
    /// 公開キー。同じキーでもrawが異なれば同じセッションとは扱わない。
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }
}
