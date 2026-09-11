//! セッション監査通知（呼出側が指定）の識別子。
use super::SessionAuditError;
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// 完成したセッション監査のドメイン型。
pub struct SessionAuditObservationId(String);
impl SessionAuditObservationId {
    fn of_uuid(v: Uuid) -> Self {
        Self(v.to_string())
    }
    #[must_use]
    /// 検査済みの値を返す。
    pub fn generate() -> Self {
        Self::of_uuid(Uuid::now_v7())
    }
    /// 検査済みの値を返す。
    /// # Errors
    /// 不変条件を満たさない場合。
    pub fn parse(raw: &str) -> Result<Self, SessionAuditError> {
        let u = Uuid::try_parse(raw).map_err(|_| SessionAuditError::InvalidEventIdentity)?;
        if u.get_version_num() == 7 && u.as_hyphenated().to_string() == raw {
            Ok(Self::of_uuid(u))
        } else {
            Err(SessionAuditError::InvalidEventIdentity)
        }
    }
    #[must_use]
    /// 検査済みの値を返す。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for SessionAuditObservationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
