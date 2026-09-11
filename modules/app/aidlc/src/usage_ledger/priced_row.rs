//! 値付けとエージェント帰属まで済ませた 1 行。
use harness_claude::{TranscriptTokenCounts, TranscriptUsageRow};

/// 台帳へ畳み込む直前の行。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PricedRow {
    uuid: String,
    message_id: String,
    timestamp: String,
    model_bucket: String,
    agent_bucket: String,
    counts: TranscriptTokenCounts,
    usd: Option<f64>,
}

impl PricedRow {
    /// 会話履歴の行に、値付けとエージェント帰属を与える。
    pub(crate) fn new(
        row: &TranscriptUsageRow,
        model_bucket: String,
        agent_bucket: String,
        usd: Option<f64>,
    ) -> Self {
        Self {
            uuid: row.uuid().to_string(),
            message_id: row.message_id().to_string(),
            timestamp: row.timestamp().to_string(),
            model_bucket,
            agent_bucket,
            counts: row.counts(),
            usd,
        }
    }

    /// cursor が覚える行の識別子。
    pub(crate) fn uuid(&self) -> &str {
        &self.uuid
    }

    /// cursor が覚える `message.id`。空なら覚え直さない。
    pub(crate) fn message_id(&self) -> &str {
        &self.message_id
    }

    /// cursor が覚える時刻。
    pub(crate) fn timestamp(&self) -> &str {
        &self.timestamp
    }

    /// `byModel` の鍵（正規化できた世代名、できなければ逐語のモデル名）。
    pub(crate) fn model_bucket(&self) -> &str {
        &self.model_bucket
    }

    /// `byAgent` の鍵（`main` / sidecar の agentType / `subagent`）。
    pub(crate) fn agent_bucket(&self) -> &str {
        &self.agent_bucket
    }

    /// トークン量。
    pub(crate) const fn counts(&self) -> TranscriptTokenCounts {
        self.counts
    }

    /// 値付けできた USD。未知世代は `None`。
    pub(crate) const fn usd(&self) -> Option<f64> {
        self.usd
    }
}
