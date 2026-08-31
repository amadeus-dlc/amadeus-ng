//! memory 層ルール束 (決定論的 steering) の読取失敗。
//!
//! 層が**無い**のは正常なのでここには来ない — 無い層は単に [`MemoryRules`] の列に
//! 現れない (02 §10 / b24 の読み順)。捉えるのは「在るのに読めない」だけであり、それは
//! blocking で `error` directive になる。
//!
//! `path` が運ぶのは**読取対象の所在**であって媒体の宣言ではない — 逐語文言の材料として
//! 要るだけで、ポートは格納形式を約束しない (オーナー追補裁定 2026-08-31)。
//!
//! 分割不能セクションもここには来ない — 読取時ではなく [`MemoryRules::plan_for`] のパック時に
//! 判明するので、ユースケースがその `Err` を `error` directive へ写す。
//!
//! [`MemoryRules`]: crate::orchestration::MemoryRules
//! [`MemoryRules::plan_for`]: crate::orchestration::MemoryRules::plan_for

use std::fmt;

/// 必須ルール層が在るのに読めない (権限・UTF-8 破損など)。材料のみを運び、文言は
/// ユースケースの `wording` が組む (`coding-rules/error-handling.md`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRulesReadError {
    path: String,
    cause: String,
}

impl MemoryRulesReadError {
    /// 読もうとした対象の所在と、OS 由来の理由を束ねる。
    #[must_use]
    pub const fn new(path: String, cause: String) -> MemoryRulesReadError {
        MemoryRulesReadError { path, cause }
    }

    /// 読もうとした対象の所在。逐語文言の材料として境界を越える
    /// (`coding-rules/abstract-data-type.md`
    /// §境界での変換 — 文言を組むのはユースケースの `wording` である)。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// 失敗の理由 (OS 由来)。同じく逐語文言の材料。
    #[must_use]
    pub fn cause(&self) -> &str {
        &self.cause
    }
}

impl fmt::Display for MemoryRulesReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.cause)
    }
}

impl std::error::Error for MemoryRulesReadError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_failure_carries_the_path_and_the_cause_as_material() {
        let error =
            MemoryRulesReadError::new("memory/org.md".to_string(), "permission denied".to_string());
        assert_eq!(error.path(), "memory/org.md");
        assert_eq!(error.cause(), "permission denied");
        assert_eq!(error.to_string(), "memory/org.md: permission denied");
    }
}
