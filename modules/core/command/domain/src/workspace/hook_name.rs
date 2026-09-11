//! フックのファイル名成分に使える識別子。
use super::HookHealthError;
/// 小文字ASCIIで始まるkebab-caseのフック名。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HookName(String);
impl HookName {
    /// フック名を検査する。区切りや空白を読み替えない。
    /// # Errors
    /// 空、先頭が小文字でない、または英小文字・数字・ハイフン以外がある場合。
    pub fn parse(name: &str) -> Result<Self, HookHealthError> {
        if !name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(HookHealthError::InvalidHookName);
        }
        Ok(Self(name.to_string()))
    }
    /// 永続化・描画の境界に渡す正準名。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
