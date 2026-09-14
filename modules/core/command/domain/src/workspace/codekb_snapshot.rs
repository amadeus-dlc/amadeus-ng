//! `CodekbSnapshot` — 走査の直前に取る 2 つの世代の写し。

use super::codekb_generation::CodekbGeneration;
use super::codekb_repo_id::CodekbRepoId;
use super::codekb_scope_paths::CodekbScopePaths;
use super::codekb_source_fingerprint::CodekbSourceFingerprint;

/// 走査が拠って立つ 2 つの世代 (ストアと源) と、それを取った範囲。
///
/// これを持って走査へ入り、公開のときに同じ 2 つを差し出す。**その間に片方でも動いていたら
/// 公開は拒まれる** — それが compare-and-swap の意味である。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodekbSnapshot {
    repo: CodekbRepoId,
    paths: CodekbScopePaths,
    store_generation: CodekbGeneration,
    source_fingerprint: CodekbSourceFingerprint,
}

impl CodekbSnapshot {
    /// 4 つの材料を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        repo: CodekbRepoId,
        paths: CodekbScopePaths,
        store_generation: CodekbGeneration,
        source_fingerprint: CodekbSourceFingerprint,
    ) -> CodekbSnapshot {
        CodekbSnapshot {
            repo,
            paths,
            store_generation,
            source_fingerprint,
        }
    }

    /// codekb を鍵付けるリポジトリ識別子。
    #[must_use]
    pub const fn repo(&self) -> &CodekbRepoId {
        &self.repo
    }

    /// 指紋を取った範囲。
    #[must_use]
    pub const fn paths(&self) -> &CodekbScopePaths {
        &self.paths
    }

    /// 写しを取った時点のストアの世代。
    #[must_use]
    pub const fn store_generation(&self) -> &CodekbGeneration {
        &self.store_generation
    }

    /// 写しを取った時点の源の指紋。
    #[must_use]
    pub const fn source_fingerprint(&self) -> &CodekbSourceFingerprint {
        &self.source_fingerprint
    }
}
