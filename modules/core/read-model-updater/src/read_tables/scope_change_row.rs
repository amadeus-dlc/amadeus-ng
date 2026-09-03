//! `ScopeChangeRow` — `read_scope_change` の 1 行 (要求 scope と state の scope の照合)。

use core_command_domain::orchestration::IntentExecutionId;

use super::row_id;
use super::spelling;

/// `read_scope_change` の 1 行。主キーは 1 列 `id` (自然キー
/// (`execution_id`, `scope`) から導いた代理キー)。`execution_id` は `read_execution.id` を
/// 指す FK である。
///
/// 読取コマンドは `--scope <名前>` の値でこの表を引く。**行が返らなければ無効な scope**で
/// あり (有効な scope にしか行が無い)、返れば `kind` が「state の scope と違うので
/// scope-change を出す」か「同じなので通常どおり進む」かを言う。
///
/// upstream は現在値を見ない config-change (depth / test_strategy / review) と違い、
/// scope だけは現在値との比較で分岐する — だから scope だけが表になる (設計 §0)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeChangeRow {
    id: String,
    execution_id: String,
    scope: String,
    kind: String,
}

impl ScopeChangeRow {
    /// 実行 1 本 × 有効 scope 1 つの照合を 1 行へ写す (**この型の唯一の構築経路**)。
    ///
    /// `same_as_state` は intent が持つ scope との一致である。判断はここに無い — 呼出側が
    /// 集約の答え同士を比べ、その結果の綴りだけをこの型が持つ。
    #[must_use]
    pub fn of(
        execution_id: &IntentExecutionId,
        scope: &str,
        same_as_state: bool,
    ) -> ScopeChangeRow {
        ScopeChangeRow {
            id: row_id::scope_change(execution_id.as_str(), scope),
            execution_id: execution_id.as_str().to_string(),
            scope: scope.to_string(),
            kind: spelling::scope_change(same_as_state).to_string(),
        }
    }

    /// 主キー — 自然キー (`execution_id`, `scope`) から導いた代理キー。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// `read_execution.id` を指す FK。
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// 要求されうる scope 名 (有効な scope だけが行になる)。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// 照合の答え (`scope-change` / `same-as-state`)。
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn execution_id() -> IntentExecutionId {
        IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("UUIDv7")
    }

    #[test]
    fn the_two_answers_are_spelled_distinctly() {
        assert_eq!(
            ScopeChangeRow::of(&execution_id(), "classic", true).kind(),
            "same-as-state"
        );
        assert_eq!(
            ScopeChangeRow::of(&execution_id(), "express", false).kind(),
            "scope-change"
        );
    }
}
