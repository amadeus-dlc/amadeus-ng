//! `IntentError` — `Intent` の構築ガードが拒否する形。

use std::fmt;

use super::plan_error::PlanError;
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
    /// 解決済み計画に同じ slug が 2 回以上現れる (BR1.5)。
    DuplicateSlug {
        /// 文書順で最初に重複した slug。
        slug: String,
    },
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
            IntentError::DuplicateSlug { slug } => write!(f, "duplicate stage slug: {slug}"),
            IntentError::StageDisplayNotSingleLine { stage, found } => write!(
                f,
                "stage display is not single line: stage {stage}, found U+{:04X}",
                *found as u32
            ),
        }
    }
}

impl From<PlanError> for IntentError {
    /// 計画そのものの違反を intent の構築エラーへ写す。検査の正本は
    /// [`StageEntries::new`] であり、`Intent` はその結果を写すだけである。
    ///
    /// [`StageEntries::new`]: crate::orchestration::StageEntries::new
    fn from(error: PlanError) -> IntentError {
        match error {
            PlanError::Empty => IntentError::Empty,
            PlanError::InitializationMustExecute => IntentError::InitializationMustExecute,
            PlanError::InitializationMustBeUnconditional => {
                IntentError::InitializationMustBeUnconditional
            }
            PlanError::DuplicateSlug { slug } => IntentError::DuplicateSlug { slug },
        }
    }
}

impl std::error::Error for IntentError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_plan_violation_maps_to_its_intent_rejection() {
        // 計画の検査は `StageEntries::new` が正本で、`Intent` はその結果を写すだけ
        // である。写し漏れがあれば拒否の意味が変わるので、対応を 1 か所で固定する。
        for (violation, expected, wording) in [
            (PlanError::Empty, IntentError::Empty, "empty stage list"),
            (
                PlanError::InitializationMustExecute,
                IntentError::InitializationMustExecute,
                "initialization stage is not EXECUTE",
            ),
            (
                PlanError::InitializationMustBeUnconditional,
                IntentError::InitializationMustBeUnconditional,
                "initialization stage is CONDITIONAL",
            ),
            (
                PlanError::DuplicateSlug {
                    slug: "intent-capture".to_string(),
                },
                IntentError::DuplicateSlug {
                    slug: "intent-capture".to_string(),
                },
                "duplicate stage slug: intent-capture",
            ),
        ] {
            let mapped = IntentError::from(violation);
            assert_eq!(mapped, expected);
            assert_eq!(mapped.to_string(), wording);
        }
    }
}
