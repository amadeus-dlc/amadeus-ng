//! `ScopeResolutionError` — scope 解決ラダーが拒否した形。
//!
//! 運ぶのは**材料だけ** (拒否された scope 名・環境変数の値) で、利用者向けの逐語文言は
//! 出す側が組む (`coding-rules/error-handling.md`)。

/// 解決の失敗 (材料のみ — 逐語文言は出す側が組む)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeResolutionError {
    /// 無効な明示 `--scope` (分岐 3b — 無条件検証)。
    UnknownExplicit {
        /// 拒否された scope 名。
        scope: String,
    },
    /// 無効な `AWS_AIDLC_DEFAULT_SCOPE` (分岐 4)。
    UnknownEnv {
        /// 拒否された環境変数の値。
        value: String,
    },
    /// ラダーを通しても解決できない (state 由来値が定義に無い等)。
    Unresolvable {
        /// 拒否された scope 名。
        scope: String,
    },
}
