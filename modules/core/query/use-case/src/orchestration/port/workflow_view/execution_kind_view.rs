//! `ExecutionKindView` — `ALWAYS` / `CONDITIONAL` (upstream 01 §3.3)。
//!
//! ステージ著者側の適用可否であり、プラン所属 (`PlanActionView`) とは直交する。

use super::unknown_value::UnknownValue;

/// ステージ著者が宣言する適用可否。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExecutionKindView {
    /// 常に適用される。`condition` を理由に実行を辞退できない。
    Always,
    /// `condition` が成立しなければ実行を辞退しうる。
    Conditional,
}

impl ExecutionKindView {
    /// 宣言順の全値。
    pub const ALL: [ExecutionKindView; 2] =
        [ExecutionKindView::Always, ExecutionKindView::Conditional];

    /// # Errors
    ///
    /// 2 値 (大文字) 以外は [`UnknownValue`] で拒否する。
    pub fn parse(s: &str) -> Result<ExecutionKindView, UnknownValue> {
        Ok(match s {
            "ALWAYS" => ExecutionKindView::Always,
            "CONDITIONAL" => ExecutionKindView::Conditional,
            other => return Err(UnknownValue::new(other)),
        })
    }

    /// `stage-graph.json` 上の語 (**大文字**。`parse` の逆写像)。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ExecutionKindView::Always => "ALWAYS",
            ExecutionKindView::Conditional => "CONDITIONAL",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_values_round_trip_in_upper_case_only() {
        for e in ExecutionKindView::ALL {
            assert_eq!(ExecutionKindView::parse(e.as_str()).unwrap(), e);
        }
        let rejected = ExecutionKindView::parse("always").unwrap_err();
        assert_eq!(rejected.as_str(), "always");
    }
}
