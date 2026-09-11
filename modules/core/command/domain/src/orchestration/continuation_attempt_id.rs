//! 停止要求を、保存後のQuery結果へ対応付ける相関識別子。
use super::ContinuationError;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// 停止要求を、保存後のQuery結果へ対応付ける相関識別子。
pub struct ContinuationAttemptId(String);
impl ContinuationAttemptId {
    fn new(uuid: uuid::Uuid) -> Self {
        Self(uuid.hyphenated().to_string())
    }
    /// 正準UUIDv7を検査する。
    /// # Errors
    /// 小文字の正準UUIDv7でない場合。
    pub fn parse(value: &str) -> Result<Self, ContinuationError> {
        let uuid = uuid::Uuid::try_parse(value).map_err(|_| ContinuationError::InvalidIdentity)?;
        if uuid.get_version_num() != 7
            || uuid.get_variant() != uuid::Variant::RFC4122
            || uuid.hyphenated().to_string() != value
        {
            return Err(ContinuationError::InvalidIdentity);
        }
        Ok(Self::new(uuid))
    }
    /// 新しい識別子を発行する。
    #[must_use]
    pub fn generate() -> Self {
        Self::new(uuid::Uuid::now_v7())
    }
    /// 保存・Query境界へ渡す正準形。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for ContinuationAttemptId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
