//! `CodekbScopePaths` — 走査範囲のパスの並び。

use core_infrastructure::collections::FirstClassCollection;

use super::codekb_scope_path::CodekbScopePath;

/// リポジトリ根を丸ごと主張する綴り (upstream 逐語 — この 1 綴りだけが短絡する)。
const ROOT_CLAIM: &str = "./";

/// 走査範囲のパスの並び (記録順を保つ)。
///
/// 被覆の判断をこの型が所有する — 呼び手が要素を取り出して `starts_with` を書き直すと、
/// 「末尾の `/` があるときだけ接頭辞として飲み込む」という規約が 2 箇所に散る
/// (`coding-rules/tell-dont-ask.md`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodekbScopePaths(Vec<CodekbScopePath>);

impl CodekbScopePaths {
    /// 記録順のまま束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn of(paths: Vec<CodekbScopePath>) -> CodekbScopePaths {
        CodekbScopePaths(paths)
    }

    /// この並びが与えられたパスを覆うか (upstream `scopePathCovered` の逐語)。
    ///
    /// 覆うのは**完全一致**か、`/` で終わるこちらのディレクトリが接頭辞として飲み込む場合の
    /// 2 つだけである。glob ではない — 走査範囲はディレクトリかファイルの綴りで書かれる。
    #[must_use]
    pub fn covers(&self, path: &CodekbScopePath) -> bool {
        self.0.iter().any(|candidate| {
            candidate == path
                || (candidate.as_str().ends_with('/')
                    && path.as_str().starts_with(candidate.as_str()))
        })
    }

    /// リポジトリ根を丸ごと主張しているか。主張していれば個々のパスを見るまでもなく覆う。
    #[must_use]
    pub fn claims_root(&self) -> bool {
        self.0.iter().any(|path| path.as_str() == ROOT_CLAIM)
    }
}

impl FirstClassCollection for CodekbScopePaths {
    type Item<'a> = &'a CodekbScopePath;
    type Filtered = CodekbScopePaths;

    fn len(&self) -> usize {
        self.0.len()
    }

    fn at(&self, index: usize) -> Option<&CodekbScopePath> {
        self.0.get(index)
    }

    fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, &'a CodekbScopePath) -> A) -> A {
        self.0.iter().fold(initial, fold)
    }

    fn filter(&self, mut predicate: impl FnMut(&CodekbScopePath) -> bool) -> CodekbScopePaths {
        CodekbScopePaths(
            self.0
                .iter()
                .filter(|path| predicate(path))
                .cloned()
                .collect(),
        )
    }
}
