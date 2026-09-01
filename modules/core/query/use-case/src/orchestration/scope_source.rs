//! `ScopeSource` — scope 解決ラダーのどの段が勝ったか。
//!
//! 観測可能な**分類**だけを運ぶ (逐語文言は出す側が組む — `coding-rules/error-handling.md` と
//! 同趣旨)。ラダー本体は [`ResolvedScope`] と同居する。
//!
//! [`ResolvedScope`]: super::ResolvedScope

/// 解決の出所 (観測可能な分類 — 逐語文言は出す側が組む)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeSource {
    /// リードモデルの `Scope` (稼働中は常に勝つ)。
    State,
    /// 明示 `--scope`。
    Explicit,
    /// 位置引数のキーワード推論。
    Inferred,
    /// `AWS_AIDLC_DEFAULT_SCOPE`。
    Env,
    /// デフォルト定数 (自由記述含む)。
    Default,
}
