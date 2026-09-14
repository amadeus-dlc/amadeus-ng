//! `ReScopeParseView` — 走査範囲ブロックを読んだ結果（読めた / 読めなかった理由）。

use super::re_scope_view::ReScopeView;

/// 走査範囲ブロックの読取結果。
///
/// **「読めなかった」は失敗ではなく観測である。** ブロックを持たない legacy ストアも、綴りが
/// 壊れたストアも、`codekb-scope-diff` が `UNKNOWN_SCOPE` として**判定に載せて exit 0 で返す**
/// もので、読取 I/O の失敗 ([`super::super::ReadModelReadError`]) とは別物である。だから
/// ここは `Result` ではなく、3 つの観測を並べた判別共用体で表す。
///
/// `Absent` と `Malformed` が運ぶのは upstream が `reason` として出す**保存トークン**
/// (`absent` / `malformed`) の区別と、`detail` の材料である。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReScopeParseView {
    /// ブロックが在り、綴りも通った。
    Parsed(ReScopeView),
    /// fenced yaml の `scope_version` ブロックが無い（scope 追跡より前のストア）。
    Absent(String),
    /// ブロックは在るが綴りが通らない（未知の版・`kind` 欠落・網羅の矛盾など）。
    Malformed(String),
}
