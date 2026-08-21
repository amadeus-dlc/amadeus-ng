//! `ExecutionKind` — `ALWAYS` / `CONDITIONAL` (upstream 01 §3.3)。
//!
//! **ステージ著者側の適用可否**であり、プラン所属 (`PlanAction` の EXECUTE / SKIP) とも
//! ゲート軸とも**直交**する。3 軸を畳み込まないこと。

/// ステージ著者が宣言する適用可否。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExecutionKind {
    Always,
    Conditional,
}

/// 閉集合外の値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownExecutionDeclaration(pub String);

impl ExecutionKind {
    pub const ALL: [ExecutionKind; 2] = [ExecutionKind::Always, ExecutionKind::Conditional];

    /// # Errors
    ///
    /// 2 値 (大文字) 以外は `UnknownExecutionDeclaration` で拒否する。
    pub fn parse(s: &str) -> Result<ExecutionKind, UnknownExecutionDeclaration> {
        Ok(match s {
            "ALWAYS" => ExecutionKind::Always,
            "CONDITIONAL" => ExecutionKind::Conditional,
            other => return Err(UnknownExecutionDeclaration(other.to_string())),
        })
    }

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
        assert_eq!(
            ExecutionKind::parse("always"),
            Err(UnknownExecutionDeclaration("always".to_string()))
        );
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
