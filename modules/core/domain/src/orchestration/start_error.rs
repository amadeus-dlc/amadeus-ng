//! `StartError` — `start` が初期状態を組み立てられない理由 (functional-spec §5)。

use std::fmt;

use crate::workflow_definition::UnknownScope;

/// 集約の生成時不変条件の違反 (集約は生成されない)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartError {
    /// 定義が知らないスコープ名 (材料は定義側の `UnknownScope`)。
    UnknownScope(UnknownScope),
    /// ステージ 0 件 — コンパイル済みグラフが空の場合のみ (防御的)。
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

impl fmt::Display for StartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StartError::UnknownScope(scope) => write!(
                f,
                "unknown scope: {} (valid: {})",
                scope.scope(),
                scope.valid_scopes().join(", ")
            ),
            StartError::Empty => f.write_str("empty stage list"),
            StartError::InitializationMustExecute => {
                f.write_str("initialization stage is not EXECUTE")
            }
            StartError::InitializationMustBeUnconditional => {
                f.write_str("initialization stage is CONDITIONAL")
            }
            StartError::StageDisplayNotSingleLine { stage, found } => write!(
                f,
                "stage display is not single line: stage {stage}, found U+{:04X}",
                *found as u32
            ),
        }
    }
}

impl std::error::Error for StartError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_definition::UnknownScope;

    #[test]
    fn the_unknown_scope_rejection_carries_the_definition_material() {
        let err = StartError::UnknownScope(UnknownScope::new(
            "nope",
            vec!["classic".to_string(), "express".to_string()],
        ));
        assert_eq!(
            err.to_string(),
            "unknown scope: nope (valid: classic, express)"
        );
    }

    #[test]
    fn the_stage_display_rejection_carries_the_stage_and_the_codepoint() {
        let err = StartError::StageDisplayNotSingleLine {
            stage: "domain-design".to_string(),
            found: '\n',
        };
        assert_eq!(
            err.to_string(),
            "stage display is not single line: stage domain-design, found U+000A"
        );
    }

    #[test]
    fn the_remaining_rejections_carry_material_not_wording() {
        assert_eq!(StartError::Empty.to_string(), "empty stage list");
        assert_eq!(
            StartError::InitializationMustExecute.to_string(),
            "initialization stage is not EXECUTE"
        );
        assert_eq!(
            StartError::InitializationMustBeUnconditional.to_string(),
            "initialization stage is CONDITIONAL"
        );
    }

    #[test]
    fn the_error_is_a_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(StartError::Empty);
        assert_eq!(err.to_string(), "empty stage list");
    }

    #[test]
    fn rejections_compare_by_value() {
        assert_eq!(StartError::Empty, StartError::Empty);
        assert_ne!(StartError::Empty, StartError::InitializationMustExecute);
    }
}
