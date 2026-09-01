//! `StateVersionKind` — 状態ファイルの版の 4 分類 (upstream `{kind:"ok"|"unparseable"|"past"|"future"}` と 1:1)。

/// 4 分類 (upstream `{kind:"ok"|"unparseable"|"past"|"future"}` と 1:1)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateVersionKind {
    /// `CURRENT_STATE_VERSION` と一致 — そのまま読み書きしてよい。
    Ok,
    /// State Version 行が無い、または値が行末アンカーに収まらない / 整数でない。
    /// upstream はこの分類でアーカイブ (`mv aidlc aidlc.archive`) と作り直しを指示する。
    Unparseable,
    /// `CURRENT_STATE_VERSION` 未満 — 旧版が書いた state ファイル。
    Past,
    /// `CURRENT_STATE_VERSION` 超過 — 新しい版の state ファイルを古い実装が読んでいる。
    Future,
}
