//! `IntentError` — `Intent` の構築ガードが拒否する形。

use std::fmt;

use crate::workflow_definition::UnknownScope;

/// `Intent` を組めない形 (材料のみ — 利用者向け文言はアダプタ層)。
///
/// initialization フェーズの扱いは BR2.2 — 状態ファイルを起こす工程そのものなので、
/// SKIP にも CONDITIONAL にもできない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentError {
    /// 定義が知らないスコープ名 (材料は定義側の `UnknownScope`)。`create` だけが返す。
    UnknownScope(UnknownScope),
    /// 解決済み計画が 0 件 — コンパイル済みグラフが空の場合のみ (防御的)。
    Empty,
    /// initialization フェーズのステージが SKIP に畳まれた、または先頭ステージが SKIP。
    InitializationMustExecute,
    /// initialization フェーズのステージが CONDITIONAL。
    InitializationMustBeUnconditional,
    /// ステージの表示属性 (表題・担当エージェント) が単一行でない。
    ///
    /// 表示属性は状態ファイルの bullet 行に書かれる値なので、改行が混ざると 2 行目以降が
    /// フィールドとして読めなくなる。定義側の値をそのまま信じず、計画を解決する時点で止める。
    StageDisplayNotSingleLine {
        /// 問題のあったステージ。
        stage: String,
        /// 走査順に最初に見つかった不正コードポイント。
        found: char,
    },
}

impl fmt::Display for IntentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntentError::UnknownScope(scope) => write!(
                f,
                "unknown scope: {} (valid: {})",
                scope.scope(),
                scope.valid_scopes().join(", ")
            ),
            IntentError::Empty => f.write_str("empty stage list"),
            IntentError::InitializationMustExecute => {
                f.write_str("initialization stage is not EXECUTE")
            }
            IntentError::InitializationMustBeUnconditional => {
                f.write_str("initialization stage is CONDITIONAL")
            }
            IntentError::StageDisplayNotSingleLine { stage, found } => write!(
                f,
                "stage display is not single line: stage {stage}, found U+{:04X}",
                *found as u32
            ),
        }
    }
}

impl std::error::Error for IntentError {}
