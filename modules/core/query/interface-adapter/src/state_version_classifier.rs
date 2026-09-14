//! 状態版の分類器の注入シーム — 分類は U2 (コマンド側ドメイン) の 1 箇所が行う。
//!
//! クエリ側はドメインへ依存しないので、合成ルートが分類器を実装して渡す。診断が
//! 分類規則を写し取ることはない (`coding-rules/cqrs-boundaries.md` 規則 2)。

use core_query_use_case::orchestration::StateVersionView;

/// `aidlc-state.md` の本文から版を分類する。
pub trait StateVersionClassifier {
    /// runtime と同じ分類器で判定した答え。
    fn classify(&self, state_content: &str) -> StateVersionView;
}
