//! 観測領域とフックの組に一意に対応するHookHealth集約の識別子。
use super::{HookHealthError, HookHealthTarget, HookName};
/// 共有ストア内の他集約と混ざらない名前空間を持つ。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HookHealthId(String);
impl HookHealthId {
    const fn of_value(value: String) -> Self {
        Self(value)
    }
    /// 観測領域とフックの組から、安定した集約識別子を導く。
    #[must_use]
    pub fn for_hook(target: &HookHealthTarget, hook: &HookName) -> Self {
        let material = format!("{}\0{}", target.relative_directory(), hook.as_str());
        Self::of_value(format!(
            "hook-health:{}",
            core_infrastructure::hash::sha256_hex(material.as_bytes())
        ))
    }
    /// 保存された識別子の正準形を確認する。
    /// # Errors
    /// 名前空間または64桁の小文字16進表記が不正な場合。
    pub fn parse(raw: &str) -> Result<Self, HookHealthError> {
        let valid = raw.strip_prefix("hook-health:").is_some_and(|digest| {
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        });
        if !valid {
            return Err(HookHealthError::InvalidIdentity);
        }
        Ok(Self::of_value(raw.to_string()))
    }
    /// 保存・Queryの境界へ渡す識別子。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for HookHealthId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
