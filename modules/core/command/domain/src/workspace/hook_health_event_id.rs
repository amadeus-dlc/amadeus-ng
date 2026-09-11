//! HookHealthの個々の観測イベントの識別子。
use super::HookHealthError;
use uuid::Uuid;
/// 集約IDとは別に、1件の保存事実を識別するUUIDv7。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HookHealthEventId(String);
impl HookHealthEventId {
    fn of_uuid(uuid: Uuid) -> Self {
        Self(uuid.as_hyphenated().to_string())
    }
    /// 正準UUIDv7を検査する。
    /// # Errors
    /// 小文字の正準UUIDv7でない場合。
    pub fn parse(raw: &str) -> Result<Self, HookHealthError> {
        let uuid = Uuid::try_parse(raw).map_err(|_| HookHealthError::InvalidEventIdentity)?;
        if uuid.get_version_num() != 7
            || uuid.get_variant() != uuid::Variant::RFC4122
            || uuid.as_hyphenated().to_string() != raw
        {
            return Err(HookHealthError::InvalidEventIdentity);
        }
        Ok(Self::of_uuid(uuid))
    }
    /// 集約のコマンドが新しい観測事実を採番する。
    #[must_use]
    pub fn generate() -> Self {
        Self::of_uuid(Uuid::now_v7())
    }
    /// 保存境界へ渡す正準表記。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for HookHealthEventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
