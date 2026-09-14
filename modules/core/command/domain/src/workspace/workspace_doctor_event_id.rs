//! `WorkspaceDoctor` の個々の診断イベントの識別子。

use super::WorkspaceDoctorError;
use uuid::Uuid;

/// 集約 ID とは別に、1 件の保存事実を識別する UUIDv7。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkspaceDoctorEventId(String);

impl WorkspaceDoctorEventId {
    fn of_uuid(uuid: Uuid) -> Self {
        Self(uuid.as_hyphenated().to_string())
    }

    /// 正準 UUIDv7 を検査する。
    ///
    /// # Errors
    ///
    /// 小文字の正準 UUIDv7 でない場合。
    pub fn parse(raw: &str) -> Result<Self, WorkspaceDoctorError> {
        let uuid = Uuid::try_parse(raw).map_err(|_| WorkspaceDoctorError::InvalidEventIdentity)?;
        if uuid.get_version_num() != 7
            || uuid.get_variant() != uuid::Variant::RFC4122
            || uuid.as_hyphenated().to_string() != raw
        {
            return Err(WorkspaceDoctorError::InvalidEventIdentity);
        }
        Ok(Self::of_uuid(uuid))
    }

    /// 集約のコマンドが新しい診断事実を採番する。
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

impl std::fmt::Display for WorkspaceDoctorEventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
