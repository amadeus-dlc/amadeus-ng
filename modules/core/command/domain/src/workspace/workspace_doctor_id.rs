//! 診断対象 (space と record) に一意に対応する `WorkspaceDoctor` 集約の識別子。

use super::{HookHealthTarget, WorkspaceDoctorError};

/// 共有ストア内の他集約と混ざらない名前空間を持つ。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkspaceDoctorId(String);

impl WorkspaceDoctorId {
    const fn of_value(value: String) -> Self {
        Self(value)
    }

    /// 診断対象から安定した集約識別子を導く。
    #[must_use]
    pub fn for_target(target: &HookHealthTarget) -> Self {
        Self::of_value(format!(
            "workspace-doctor:{}",
            core_infrastructure::hash::sha256_hex(target.relative_directory().as_bytes())
        ))
    }

    /// 保存された識別子の正準形を確認する。
    ///
    /// # Errors
    ///
    /// 名前空間または 64 桁の小文字 16 進表記が不正な場合。
    pub fn parse(raw: &str) -> Result<Self, WorkspaceDoctorError> {
        let valid = raw.strip_prefix("workspace-doctor:").is_some_and(|digest| {
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        });
        if !valid {
            return Err(WorkspaceDoctorError::InvalidIdentity);
        }
        Ok(Self::of_value(raw.to_string()))
    }

    /// 保存・投影・Query の境界へ渡す識別子。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorkspaceDoctorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
