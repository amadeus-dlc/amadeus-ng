//! 成果物監査集約の識別子。
use super::{HookHealthError, HookHealthTarget};
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// 完成した成果物監査のドメイン型。
pub struct ArtifactAuditId(String);
impl ArtifactAuditId {
    const fn of_value(v: String) -> Self {
        Self(v)
    }
    #[must_use]
    /// 検査済みの値を返す。
    pub fn for_target(target: &HookHealthTarget) -> Self {
        Self::of_value(format!(
            "artifact-audit:{}",
            core_infrastructure::hash::sha256_hex(target.relative_directory().as_bytes())
        ))
    }
    /// 検査済みの値を返す。
    /// # Errors
    /// 不変条件を満たさない場合。
    pub fn parse(raw: &str) -> Result<Self, HookHealthError> {
        let valid = raw.strip_prefix("artifact-audit:").is_some_and(|v| {
            v.len() == 64
                && v.bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        });
        if valid {
            Ok(Self::of_value(raw.into()))
        } else {
            Err(HookHealthError::InvalidIdentity)
        }
    }
    #[must_use]
    /// 検査済みの値を返す。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for ArtifactAuditId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
