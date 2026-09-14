//! `CodekbRepoId` — codekb を鍵付けるリポジトリ識別子。
//!
//! codekb は**リポジトリ識別子で鍵付けられ**、space の中の全 intent が共有する。その識別子は
//! `aidlc/spaces/<space>/codekb/<repo>/` の**パス片としてそのまま使われる**ので、生の文字列が
//! `join()` に届く前にここで検査する ([`super::SpaceName`] と同じ流儀)。
//!
//! 文法は upstream `REPO_NAME_REGEX` (`/^[A-Za-z0-9][A-Za-z0-9._-]*$/`) の逐語である
//! (`aidlc-lib.ts:10480`)。先頭を英数字に限ることで `.` / `..` と隠しディレクトリが同時に
//! 落ちるので、upstream の `name !== "." && name !== ".."` は別途要らない。

use std::fmt;

use super::codekb_repo_id_error::CodekbRepoIdError;

/// パース済みのリポジトリ識別子 (Always Valid — 不正値はこの型に存在しない)。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodekbRepoId(String);

impl CodekbRepoId {
    // 検査済みの識別子だけを包む。
    const fn of_id(id: String) -> Self {
        Self(id)
    }

    /// upstream の文法で検査する。**前後の空白は落とさない** — upstream も落とさないので、
    /// 空白付きの `--repo` はこちらでも拒否になる。
    ///
    /// # Errors
    ///
    /// 空・先頭が英数字でない・`[A-Za-z0-9._-]` 以外の文字を含む綴りを拒否する。
    pub fn parse(s: &str) -> Result<CodekbRepoId, CodekbRepoIdError> {
        let mut chars = s.chars();
        match chars.next() {
            None => return Err(CodekbRepoIdError::Empty),
            Some(c) if !c.is_ascii_alphanumeric() => {
                return Err(CodekbRepoIdError::InvalidLeading(c));
            }
            Some(_) => {}
        }
        for c in chars {
            if !(c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')) {
                return Err(CodekbRepoIdError::InvalidChar(c));
            }
        }
        Ok(Self::of_id(s.to_string()))
    }

    /// 検証済みのパス片 — `join()` に渡してよい唯一の形。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CodekbRepoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
