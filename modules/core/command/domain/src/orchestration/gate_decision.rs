//! `GateDecision` — `next` が名指すステージの承認ゲートの 3 値 (BR1.3 + walking-skeleton)。
//!
//! 判断の所在は集約である。`next_decision` の `RunStage` が運ぶ答えであり、RMU が
//! `read_next_answer.gate` へ綴りで投影する。クエリ側の
//! [`GateField`](https://docs.rs) は**別の型**である — あちらはワイヤ (directive の `gate`
//! フィールド) の形であり、こちらはドメインの判断そのものである
//! (`coding-rules/cqrs-boundaries.md` — コマンド側とクエリ側は相互に依存しない)。

/// `next` が名指すステージのゲート判断。
///
/// upstream `computeGate` (ピン `3c3146cf` `aidlc-orchestrate.ts:1756-1771`) の 3 値と同型:
/// initialization は `Ungated`、walking-skeleton ゲートのステージで stance 未記録なら
/// `Unresolved`、それ以外は `Gated`。stance が記録済みなら `resolveSkeletonGate` がどの
/// stance でも `true` を返すので `Gated` である。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateDecision {
    /// 承認ゲート付き。
    Gated,
    /// ゲートなし (initialization フェーズ)。
    Ungated,
    /// walking-skeleton の分類待ち — conductor が stance を報告するまで決まらない。
    Unresolved,
}

impl GateDecision {
    /// リードモデル `read_next_answer.gate` の綴り (**この 3 語が正本**)。
    ///
    /// 逐語文言ではなく**キーになる分類子**である — ワイヤの `gate` を描くのは出す側
    /// (プレゼンタ) の仕事である (`coding-rules/cqrs-boundaries.md` 規則 6)。
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            GateDecision::Gated => "gated",
            GateDecision::Ungated => "ungated",
            GateDecision::Unresolved => "unresolved",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_decisions_spell_distinct_row_values() {
        assert_eq!(GateDecision::Gated.spelling(), "gated");
        assert_eq!(GateDecision::Ungated.spelling(), "ungated");
        assert_eq!(GateDecision::Unresolved.spelling(), "unresolved");
    }
}
