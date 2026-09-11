//! 利用量の畳み込みで、どこまでの群を「完了」と見なすか。
//!
//! Claudeは1回のllm呼出し（同じ`message.id`）を複数のJSONL行へ分けて書く。最後の群は
//! 次の群が始まるまで完了と確定できないので、締め方を段の性格で切り替える
//! （本家 `tools/aidlc-usage.ts` の `FoldMode`）。

/// 畳み込みの締め方。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldMode {
    /// どのファイルも最後の群を保留する（PostToolUse）。
    Holdback,
    /// mainの最後の群だけを締める（通常のPreToolUse）。
    SealMain,
    /// すべてのファイルの完了群を締める（工程を進めるPreToolUse・Stop）。
    FlushAll,
}

impl FoldMode {
    /// mainの会話履歴の最後の群を締めるか。
    #[must_use]
    pub const fn seals_main(self) -> bool {
        !matches!(self, Self::Holdback)
    }

    /// sub-agentの会話履歴の最後の群を締めるか。
    #[must_use]
    pub const fn seals_subagents(self) -> bool {
        matches!(self, Self::FlushAll)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holdback_seals_nothing_and_flush_all_seals_everything() {
        assert!(!FoldMode::Holdback.seals_main());
        assert!(!FoldMode::Holdback.seals_subagents());
        assert!(FoldMode::SealMain.seals_main());
        assert!(!FoldMode::SealMain.seals_subagents());
        assert!(FoldMode::FlushAll.seals_main());
        assert!(FoldMode::FlushAll.seals_subagents());
    }
}
