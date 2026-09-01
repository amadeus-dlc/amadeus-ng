//! `ExecutionKind` — `ALWAYS` / `CONDITIONAL` (upstream 01 §3.3)。
//!
//! **ステージ著者側の適用可否**であり、プラン所属 (`PlanAction` の EXECUTE / SKIP) とも
//! ゲート軸とも**直交**する。3 軸を畳み込まないこと。

use super::unknown_execution_kind::UnknownExecutionKind;

/// ステージ著者が宣言する適用可否。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExecutionKind {
    /// 常に適用される。`condition` を理由に実行を辞退できない。
    Always,
    /// `condition` が成立しなければ実行を辞退しうる — `skipped` の自己申告が受理される
    /// 唯一の宣言 (プランが既に SKIP と言っている場合を除く)。
    Conditional,
}

impl ExecutionKind {
    /// 宣言順の全値。
    pub const ALL: [ExecutionKind; 2] = [ExecutionKind::Always, ExecutionKind::Conditional];

    /// # Errors
    ///
    /// 2 値 (大文字) 以外は `UnknownExecutionKind` で拒否する。
    pub fn parse(s: &str) -> Result<ExecutionKind, UnknownExecutionKind> {
        Ok(match s {
            "ALWAYS" => ExecutionKind::Always,
            "CONDITIONAL" => ExecutionKind::Conditional,
            other => return Err(UnknownExecutionKind::new(other)),
        })
    }

    /// ステージ frontmatter / `stage-graph.json` 上の語 (**大文字**。`parse` の逆写像)。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ExecutionKind::Always => "ALWAYS",
            ExecutionKind::Conditional => "CONDITIONAL",
        }
    }

    /// CONDITIONAL なステージは `skipped` を自己申告しうる (10 §I13 の受理条件の片腕)。
    #[must_use]
    pub fn is_conditional(self) -> bool {
        self == ExecutionKind::Conditional
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn both_values_round_trip_in_upper_case_only() {
        for e in ExecutionKind::ALL {
            assert_eq!(ExecutionKind::parse(e.as_str()).unwrap(), e);
        }
        // 小文字は閉集合外。拒否理由の材料として生値を逐語で持ち帰る
        let rejected = ExecutionKind::parse("always").unwrap_err();
        assert_eq!(rejected.as_str(), "always");
        assert_eq!(rejected, UnknownExecutionKind::new("always"));
    }

    #[test]
    fn conditional_is_the_only_self_skippable_declaration() {
        assert!(ExecutionKind::Conditional.is_conditional());
        assert!(!ExecutionKind::Always.is_conditional());
    }

    proptest! {
        #[test]
        fn rejects_anything_outside_the_closed_set(s in "[A-Za-z]{1,12}") {
            let known = ExecutionKind::ALL.iter().any(|e| e.as_str() == s);
            prop_assert_eq!(ExecutionKind::parse(&s).is_ok(), known);
        }
    }
}
