//! 実行状態リードモデルの読取失敗。
//!
//! **不在はここに来ない** — active-intent がまだ無いワークフロー (誕生分岐 4a) は正常な観測
//! であり、[`ExecutionStateDao::find`] の `Ok(None)` が運ぶ。ここが捉えるのは「在るのに
//! 読めない」「読めたが復号できない」だけである。
//!
//! 運ぶのは**材料だけ**で、利用者向けの逐語文言は出す側 (ユースケースの `wording`) が組む
//! (`coding-rules/error-handling.md`)。復号の拒否理由も `String` に畳んである — 復号器の型は
//! アダプタの持ち物であり、ポート面に出せばクエリ側の契約が実装の内部を語ってしまう。
//!
//! `path` が運ぶのは**読取対象の所在**であって媒体の宣言ではない — ファイルならパス、
//! 別の媒体なら読める形の所在を入れる。文言の材料として要るだけで、ポートは格納形式を
//! 約束しない (オーナー追補裁定 2026-08-31)。
//!
//! [`ExecutionStateDao::find`]: super::ExecutionStateDao::find

use std::fmt;

/// 実行状態リードモデル読取の失敗 (不在は失敗ではない)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStateReadError {
    /// 読取対象は在るが読めない (権限・UTF-8 破損・EISDIR 等)。
    NotReadable {
        /// 読もうとした対象の所在。
        path: String,
        /// 失敗の理由 (OS 由来)。
        cause: String,
    },
    /// 読めたが実行状態リードモデルとして復号できない。
    Malformed {
        /// 読んだ対象の所在。
        path: String,
        /// 復号の拒否理由 (材料のみ — 復号器の型ではなくその描写)。
        cause: String,
    },
}

impl fmt::Display for ExecutionStateReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionStateReadError::NotReadable { path, cause }
            | ExecutionStateReadError::Malformed { path, cause } => write!(f, "{path}: {cause}"),
        }
    }
}

impl std::error::Error for ExecutionStateReadError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_failures_describe_the_path_and_the_cause() {
        assert_eq!(
            ExecutionStateReadError::NotReadable {
                path: "/r/aidlc-state.md".to_string(),
                cause: "permission denied".to_string(),
            }
            .to_string(),
            "/r/aidlc-state.md: permission denied"
        );
        assert_eq!(
            ExecutionStateReadError::Malformed {
                path: "/r/aidlc-state.md".to_string(),
                cause: "missing field \"Status\"".to_string(),
            }
            .to_string(),
            "/r/aidlc-state.md: missing field \"Status\""
        );
    }
}
