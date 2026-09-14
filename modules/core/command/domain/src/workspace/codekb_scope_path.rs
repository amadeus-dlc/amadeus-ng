//! `CodekbScopePath` — 走査範囲が名指すリポジトリ相対のパス 1 本。

use std::fmt;

use super::codekb_scope_path_error::CodekbScopePathError;

/// 走査範囲が名指すパス。
///
/// **綴りを正規化しない**のが要点である — 末尾の `/` の有無が被覆の意味を変えるからである
/// (`src/` はその配下を飲み込むが、`src` は完全一致しか飲み込まない。upstream
/// `scopePathCovered` の逐語)。正規化すると、その区別が消える。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodekbScopePath(String);

impl CodekbScopePath {
    const fn of_value(value: String) -> Self {
        Self(value)
    }

    /// 綴りをそのまま受ける。
    ///
    /// # Errors
    ///
    /// 空、または空白だけの綴りを拒否する。
    pub fn parse(s: &str) -> Result<CodekbScopePath, CodekbScopePathError> {
        if s.trim().is_empty() {
            return Err(CodekbScopePathError::Empty);
        }
        Ok(Self::of_value(s.to_string()))
    }

    /// 記録されたままの綴り。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CodekbScopePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
