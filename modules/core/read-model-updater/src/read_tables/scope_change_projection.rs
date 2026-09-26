//! `read_scope_change` の行を組む投影 — 要求 scope と state の scope の照合を
//! [`ScopeChangeRow`] へ写す。

use core_command_domain::orchestration::IntentExecutionId;

use super::row_id;
use super::spelling;
use crate::orchestration::ScopeChangeRow;

/// 実行 1 本 × 有効 scope 1 つの照合を 1 行へ写す。
///
/// `same_as_state` は intent が持つ scope との一致である。判断はここに無い — 呼出側が
/// 集約の答え同士を比べ、その結果の綴りだけを行が持つ。
pub(super) fn row(
    execution_id: &IntentExecutionId,
    scope: &str,
    same_as_state: bool,
) -> ScopeChangeRow {
    ScopeChangeRow::new(
        row_id::scope_change(execution_id.as_str(), scope),
        execution_id.as_str().to_string(),
        scope.to_string(),
        spelling::scope_change(same_as_state).to_string(),
    )
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
            row(&execution_id(), "classic", true).kind(),
            "same-as-state"
        );
        assert_eq!(
            row(&execution_id(), "express", false).kind(),
            "scope-change"
        );
    }
}
