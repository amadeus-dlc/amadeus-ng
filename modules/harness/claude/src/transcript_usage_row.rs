//! Claude形式のJSONL 1行を、課金対象のassistant応答として読む。
//!
//! 本家 `tools/aidlc-usage.ts:346-404` の `parseTranscriptLine`。壊れた行・非assistant行・
//! `usage` の無い行は「課金対象ではない」観測として落とす（例外にしない）。
use crate::transcript_token_counts::TranscriptTokenCounts;
use serde_json::Value;

/// 1 回の llm 呼出しに対応する行。
#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptUsageRow {
    uuid: String,
    message_id: String,
    timestamp: String,
    model: String,
    agent_id: Option<String>,
    counts: TranscriptTokenCounts,
}

impl TranscriptUsageRow {
    const fn new(
        uuid: String,
        message_id: String,
        timestamp: String,
        model: String,
        agent_id: Option<String>,
        counts: TranscriptTokenCounts,
    ) -> Self {
        Self {
            uuid,
            message_id,
            timestamp,
            model,
            agent_id,
            counts,
        }
    }

    /// 1 行を読む。課金対象でなければ `None`。
    ///
    /// `fallback_agent_id` は行が自分の `agentId` を持たないときの持ち主（sub-agent の
    /// ファイル名から来る）である。
    #[must_use]
    pub fn parse(line: &str, fallback_agent_id: Option<&str>) -> Option<Self> {
        let trimmed = core_infrastructure::ecmascript::trim(line);
        if trimmed.is_empty() {
            return None;
        }
        let value: Value = serde_json::from_str(trimmed).ok()?;
        let message = value.get("message")?;
        if message.get("role").and_then(Value::as_str) != Some("assistant") {
            return None;
        }
        let usage = message.get("usage").filter(|usage| usage.is_object())?;
        Some(Self::new(
            text(value.get("uuid")),
            text(message.get("id")),
            text(value.get("timestamp")),
            text(message.get("model")),
            value
                .get("agentId")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or_else(|| fallback_agent_id.map(str::to_owned)),
            TranscriptTokenCounts::of_usage(usage),
        ))
    }

    /// 行の識別子（cursor の進み先）。
    #[must_use]
    pub fn uuid(&self) -> &str {
        &self.uuid
    }

    /// 同じ llm 呼出しをまとめる `message.id`。空の行はまとめない。
    #[must_use]
    pub fn message_id(&self) -> &str {
        &self.message_id
    }

    /// 行の時刻（逐語）。
    #[must_use]
    pub fn timestamp(&self) -> &str {
        &self.timestamp
    }

    /// `message.model` の逐語。正規化はしない。
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// 行の持ち主のsub-agent識別子。
    #[must_use]
    pub fn agent_id(&self) -> Option<&str> {
        self.agent_id.as_deref()
    }

    /// トークン量。
    #[must_use]
    pub const fn counts(&self) -> TranscriptTokenCounts {
        self.counts
    }

    /// 同じ群のもう 1 行と比べ、代表としてふさわしいのはどちらか。
    ///
    /// 総量が大きいほうを採り、同点なら **後の行** を採る（新形式は先頭行が 0 で、実測値が
    /// 最後に来る。旧形式は全行が同値なので最後の行＝実際の終端になる）。
    #[must_use]
    pub fn outranked_by(&self, later: &Self) -> bool {
        later.counts.magnitude() >= self.counts.magnitude()
    }
}

/// 文字列でなければ空文字にする（本家の `typeof x === "string" ? x : ""`）。
fn text(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ASSISTANT: &str = r#"{"uuid":"u1","timestamp":"2026-09-10T00:00:00Z","type":"assistant","message":{"id":"msg_1","role":"assistant","model":"claude-opus-4-8","usage":{"input_tokens":100,"output_tokens":20}}}"#;

    #[test]
    fn a_priced_assistant_line_carries_its_identity_and_counts() {
        let row = TranscriptUsageRow::parse(ASSISTANT, None).expect("課金対象の行");
        assert_eq!(row.uuid(), "u1");
        assert_eq!(row.message_id(), "msg_1");
        assert_eq!(row.timestamp(), "2026-09-10T00:00:00Z");
        assert_eq!(row.model(), "claude-opus-4-8");
        assert_eq!(row.agent_id(), None);
        assert_eq!(row.counts().input(), 100.0);
    }

    #[test]
    fn lines_without_priced_assistant_usage_are_dropped() {
        for line in [
            "",
            "   ",
            "{ broken",
            r#"{"message":{"role":"user","usage":{}}}"#,
            r#"{"message":{"role":"assistant"}}"#,
            r#"{"message":{"role":"assistant","usage":"none"}}"#,
            r#"{"message":"text"}"#,
            "[1,2,3]",
            "42",
        ] {
            assert_eq!(TranscriptUsageRow::parse(line, None), None, "行: {line}");
        }
    }

    #[test]
    fn a_sub_agent_line_falls_back_to_the_files_owner() {
        let row = TranscriptUsageRow::parse(ASSISTANT, Some("agent-one")).expect("課金対象の行");
        assert_eq!(row.agent_id(), Some("agent-one"));
        let own = TranscriptUsageRow::parse(
            r#"{"agentId":"own","message":{"role":"assistant","usage":{}}}"#,
            Some("agent-one"),
        )
        .expect("課金対象の行");
        assert_eq!(own.agent_id(), Some("own"));
    }

    #[test]
    fn the_representative_of_a_split_run_is_the_last_maximum() {
        let zero = TranscriptUsageRow::parse(
            r#"{"uuid":"a","message":{"id":"m","role":"assistant","usage":{"input_tokens":0}}}"#,
            None,
        )
        .expect("課金対象の行");
        let real = TranscriptUsageRow::parse(
            r#"{"uuid":"b","message":{"id":"m","role":"assistant","usage":{"input_tokens":9}}}"#,
            None,
        )
        .expect("課金対象の行");
        assert!(zero.outranked_by(&real), "0の行は実測値の行に負ける");
        assert!(!real.outranked_by(&zero), "実測値の行は0の行に負けない");
        assert!(real.outranked_by(&real), "同点は後の行を採る");
    }
}
