//! ソース索引に基づくユースケースのドメイン getter 検査。
//!
//! rustc の型検査ではない。マクロ展開、動的 dispatch、関連型、外部 crate の実装、
//! 複雑なパターンからの型復元は対象外。型を確定できない呼出しは推測で禁止しない。

mod index;
mod infer;
mod projection;
mod resolve;
mod usage;

use std::collections::BTreeSet;

use crate::check::{Finding, is_suppressed, is_test_path};
use index::Index;

/// 複数ファイルのドメイン定義とポートの戻り値を結ぶ読み取り専用索引。
pub(crate) struct DomainIndex(Index);

impl DomainIndex {
    /// 同じ走査の全ソースから索引を構築する。構文エラーは通常の検査が報告する。
    pub(crate) fn build(sources: &[(String, String)]) -> Self {
        Self(Index::build(sources))
    }

    /// command/query の use-case 層だけに適用し、既存の理由付き抑制を尊重する。
    pub(crate) fn check(&self, path: &str, source: &str) -> Vec<Finding> {
        if !path.contains("/use-case/src/")
            || is_test_path(path)
            || self.0.test_files.contains(path)
        {
            return Vec::new();
        }
        let Ok(file) = syn::parse_file(source) else {
            return Vec::new();
        };
        let lines: Vec<_> = source.lines().collect();
        let mut seen = BTreeSet::new();
        usage::check(&self.0, path, &file)
            .into_iter()
            .filter(|finding| !is_suppressed(&lines, finding))
            .filter(|finding| seen.insert((finding.line, finding.message.clone())))
            .collect()
    }
}

#[cfg(test)]
mod tests;
