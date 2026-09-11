//! コード生成計画の段階全体または個別作業単位の指定。
use super::PlanApprovalError;
use core_infrastructure::ecmascript::trim;
/// 正規化済みの承認対象。作業単位は安全な1パス成分に限る。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanTarget {
    unit: Option<String>,
}
impl PlanTarget {
    const fn of_unit(unit: Option<String>) -> Self {
        Self { unit }
    }

    /// 作業単位を分けない段階全体。
    #[must_use]
    pub const fn stage_level() -> Self {
        Self::of_unit(None)
    }
    /// 個別作業単位を指定する。
    /// # Errors
    /// 空、64文字超、または安全なASCIIパス成分でない場合。
    pub fn for_unit(value: &str) -> Result<Self, PlanApprovalError> {
        let value = trim(value);
        if value.is_empty() {
            return Err(PlanApprovalError::new("Unit name is empty"));
        }
        let length = value.encode_utf16().count();
        if length > 64 {
            let prefix =
                String::from_utf16_lossy(&value.encode_utf16().take(32).collect::<Vec<_>>());
            return Err(PlanApprovalError::new(format!(
                "Unit name \"{prefix}...\" is {length} chars; max is 64"
            )));
        }
        if !value.starts_with(|character: char| character.is_ascii_alphanumeric())
            || !value.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
            })
        {
            return Err(PlanApprovalError::new(format!(
                "Invalid Unit name \"{value}\" - must match /^[A-Za-z0-9][A-Za-z0-9._-]*$/ (ASCII letter/digit, then ASCII letters/digits/dot/underscore/hyphen)"
            )));
        }
        Ok(Self::of_unit(Some(value.to_string())))
    }
    /// 作業単位。段階全体ならNone。
    #[must_use]
    pub fn unit(&self) -> Option<&str> {
        self.unit.as_deref()
    }
    /// 公開の承認対象識別子。
    #[must_use]
    pub fn id(&self) -> String {
        self.unit().map_or_else(
            || "stage:code-generation".to_string(),
            |unit| format!("unit:{unit}"),
        )
    }
}
