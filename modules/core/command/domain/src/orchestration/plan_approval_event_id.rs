//! 呼出側が発行し、共有承認イベントの保存と結果取得を対応させる識別子。
use super::plan_approval_error::PlanApprovalError;
use std::fmt;
use uuid::Uuid;

/// 共有承認イベント要求の識別子。イベントIDと実行IDは別の型で区別する。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlanApprovalEventId(String);
impl PlanApprovalEventId {
    fn new(uuid: Uuid) -> Self {
        Self(uuid.as_hyphenated().to_string())
    }
    /// UUIDv7の正準表記を受け取る。
    /// # Errors
    /// 小文字の正準UUIDv7でない場合。
    pub fn parse(raw: &str) -> Result<Self, PlanApprovalError> {
        let raw = raw.trim();
        let uuid = Uuid::try_parse(raw).map_err(|_| {
            PlanApprovalError::new("not a canonical UUIDv7 (expected lowercase 8-4-4-4-12)")
        })?;
        if uuid.get_version_num() != 7
            || uuid.get_variant() != uuid::Variant::RFC4122
            || uuid.as_hyphenated().to_string() != raw
        {
            return Err(PlanApprovalError::new(
                "not a canonical UUIDv7 (expected lowercase 8-4-4-4-12)",
            ));
        }
        Ok(Self::new(uuid))
    }
    /// 呼出側が新しい共有承認イベントを識別するために採番する。
    #[must_use]
    pub fn generate() -> Self {
        Self::new(Uuid::now_v7())
    }
    /// 保存・照会の境界へ正準表記を渡す。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for PlanApprovalEventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
