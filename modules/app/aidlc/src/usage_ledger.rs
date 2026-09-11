//! 機械ローカルな利用量台帳（`aidlc/.aidlc-sessions/usage-ledger.json`）。
//!
//! 本家 `tools/aidlc-usage.ts` の producer 相当である。ジャーナルにもリードモデルにも
//! 属さない per-user のランタイム状態で、`.gitignore` 済みの `aidlc/.aidlc-sessions/`
//! 配下に置く（[`crate::session_navigation`] と同じ扱い）。
//!
//! 台帳は `cursors`（ファイルごとの読取り位置と保留群）・workspace 全体の診断用集計・
//! `workflows`（作業 × session の権威ある集計）から成る。JSON の形と鍵順は
//! `JSON.stringify(ledger, null, 2)` の観測契約であり、[`UsageLedger::render`] がそれを写す。
//!
//! 停止フラグ `AIDLC_DISABLE_USAGE_TRACKING=1` は producer と consumer の両方を止める。
//! 呼出しごとに読む（各フックは別プロセスであり、環境変数は起動ごとに変わる）。
//! 環境変数の書換えは edition 2024 で `unsafe` になり、本ワークスペースは
//! `unsafe_code = "forbid"` なので、切替の検証は子プロセスを起こす契約テストが持つ。
mod fold_attribution;
mod fold_source;
mod ledger_cursor;
mod ledger_lock;
mod message_group;
mod model_rates;
mod ordered_map;
mod pending_group;
mod priced_row;
mod stage_bucket;
mod totals;
mod transcript_chunk;
mod transcript_session;
mod usage_aggregate;
mod workflow_usage;

pub(crate) use fold_attribution::FoldAttribution;
pub(crate) use fold_source::FoldSource;
pub(crate) use ledger_lock::LedgerLock;
pub(crate) use model_rates::ModelRates;
pub(crate) use transcript_session::TranscriptSession;

use core_infrastructure::canon_json::{
    JsonValue, Number, ObjectMembers, SerializationProfile, parse, serialize,
};
use ledger_cursor::LedgerCursor;
use message_group::MessageGroup;
use ordered_map::OrderedMap;
use pending_group::PendingGroup;
use priced_row::PricedRow;
use std::path::Path;
use transcript_chunk::TranscriptChunk;
use usage_aggregate::UsageAggregate;
use workflow_usage::WorkflowUsage;

/// 利用量の記録が止められているか。
pub(crate) fn tracking_disabled() -> bool {
    std::env::var("AIDLC_DISABLE_USAGE_TRACKING").as_deref() == Ok("1")
}

/// 現在の台帳の版。数え方が変わったら上げ、古い台帳は読まずに作り直す（本家 v3）。
const CURRENT_SCHEMA_VERSION: u64 = 3;

/// 利用量台帳。
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct UsageLedger {
    cursors: OrderedMap<LedgerCursor>,
    aggregate: UsageAggregate,
    workflows: OrderedMap<WorkflowUsage>,
}

impl UsageLedger {
    const fn new(
        cursors: OrderedMap<LedgerCursor>,
        aggregate: UsageAggregate,
        workflows: OrderedMap<WorkflowUsage>,
    ) -> Self {
        Self {
            cursors,
            aggregate,
            workflows,
        }
    }

    /// ファイルから読む。無い・壊れている・古い版はいずれも空の台帳にする（作り直し）。
    pub(crate) fn load(path: &Path) -> Self {
        let Ok(bytes) = std::fs::read(path) else {
            return Self::default();
        };
        parse(&String::from_utf8_lossy(&bytes))
            .map_or_else(|_| Self::default(), |value| Self::of_json(&value))
    }

    /// JSON から読む。版が古い、または `byteOffset` の無い cursor があれば空にする。
    ///
    /// 古い版の合計は別の数え方で作られており、その上へ足すと二重計上になる。
    /// 直せないので捨てて、次の畳み込みで会話履歴から作り直す（本家 `loadLedger` の MIGRATION）。
    pub(crate) fn of_json(value: &JsonValue) -> Self {
        let JsonValue::Object(members) = value else {
            return Self::default();
        };
        let version = match members.get("schemaVersion") {
            Some(JsonValue::Number(Number::PosInt(number))) => *number as f64,
            Some(JsonValue::Number(Number::NegInt(number))) => *number as f64,
            Some(JsonValue::Number(Number::Float(number))) => *number,
            _ => 0.0,
        };
        if version < CURRENT_SCHEMA_VERSION as f64 {
            return Self::default();
        }
        let mut cursors = OrderedMap::new();
        if let Some(JsonValue::Object(entries)) = members.get("cursors") {
            for (key, entry) in entries.iter() {
                let Some(cursor) = LedgerCursor::of_json(entry) else {
                    return Self::default();
                };
                cursors.insert(key, cursor);
            }
        }
        let workflows = match members.get("workflows") {
            Some(JsonValue::Object(entries)) => {
                entries.fold_left(OrderedMap::new(), |mut map, key, workflow| {
                    map.insert(key, WorkflowUsage::of_json(workflow));
                    map
                })
            }
            _ => OrderedMap::new(),
        };
        Self::new(cursors, UsageAggregate::of_json(Some(value)), workflows)
    }

    /// 本家の鍵順で JSON にする。
    pub(crate) fn to_json(&self) -> JsonValue {
        let mut fields = ObjectMembers::new();
        fields.insert(
            "schemaVersion",
            JsonValue::Number(Number::PosInt(CURRENT_SCHEMA_VERSION)),
        );
        fields.insert(
            "cursors",
            JsonValue::Object(self.cursors.fold_left(
                ObjectMembers::new(),
                |mut members, key, cursor| {
                    members.insert(key, cursor.to_json());
                    members
                },
            )),
        );
        let mut fields = fields.combine(&self.aggregate.to_members());
        fields.insert(
            "workflows",
            JsonValue::Object(self.workflows.fold_left(
                ObjectMembers::new(),
                |mut members, key, workflow| {
                    members.insert(key, workflow.to_json());
                    members
                },
            )),
        );
        JsonValue::Object(fields)
    }

    /// `JSON.stringify(ledger, null, 2)` と同じバイト列（末尾改行なし）。
    pub(crate) fn render(&self) -> String {
        let mut text = serialize(&self.to_json(), SerializationProfile::ContractPretty);
        if text.ends_with('\n') {
            text.pop();
        }
        text
    }

    /// 原子的に書く（親ディレクトリは作る）。
    ///
    /// # Errors
    /// ディレクトリ作成・一時ファイル書込・rename の失敗。
    pub(crate) fn write(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        core_infrastructure::atomic::write_file_atomic(path, self.render().as_bytes())
    }

    /// 1 ファイルの新しいバイトを畳み込み、その cursor を進める。
    ///
    /// 群は次の群が始まって初めて完了と分かる。締めない呼出しでは最後の群を保留し、
    /// その先頭バイト位置まで巻き戻して次回に読み直す。締める呼出しでも、末尾に書きかけの
    /// 断片が残っていれば（同じ群の続きかもしれないので）締めない
    /// （本家 `foldFileIntoLedger` の HOLDBACK）。
    pub(crate) fn fold_source(
        &mut self,
        source: &FoldSource,
        rates: &ModelRates,
        current: &FoldAttribution,
    ) {
        let cursor = self.cursors.get(source.cursor_key()).cloned();
        let previous_offset = cursor.as_ref().map_or(0, LedgerCursor::byte_offset);
        let Some(chunk) =
            TranscriptChunk::read(source.path(), previous_offset, source.close_last_group())
        else {
            return;
        };
        let groups = MessageGroup::collapse(chunk.parsed_rows(source.fallback_agent_id()));
        let close_last_group = source.close_last_group() && !chunk.trailing_partial();
        let held = if close_last_group {
            None
        } else {
            groups.last().cloned()
        };
        let fold_count = groups.len() - usize::from(held.is_some());
        let new_byte_offset = held
            .as_ref()
            .map_or(chunk.new_byte_offset(), MessageGroup::byte_start);
        let prior_pending = if chunk.reset() {
            None
        } else {
            cursor.as_ref().and_then(|cursor| cursor.pending().cloned())
        };
        let mut last = cursor.as_ref().map_or_else(
            || (String::new(), String::new(), String::new()),
            |cursor| {
                (
                    cursor.last_uuid().to_string(),
                    cursor.last_timestamp().to_string(),
                    cursor.last_message_id().to_string(),
                )
            },
        );
        for (index, group) in groups.iter().take(fold_count).enumerate() {
            let row = group.representative();
            let attribution = match &prior_pending {
                Some(pending) if index == 0 && pending.byte_offset() == group.byte_start() => {
                    pending.attribution().captured_over(current)
                }
                _ => current.clone(),
            };
            let priced = PricedRow::new(
                row,
                rates.bucket(row.model()),
                source.agent_bucket(row),
                rates.price(row.counts(), row.model()),
            );
            self.fold_row(&priced, &attribution);
            last.0 = priced.uuid().to_string();
            last.1 = priced.timestamp().to_string();
            if !priced.message_id().is_empty() {
                last.2 = priced.message_id().to_string();
            }
        }
        let pending = match held {
            Some(held) => Some(match prior_pending {
                Some(pending) if pending.byte_offset() == held.byte_start() => pending,
                _ => PendingGroup::new(
                    held.byte_start(),
                    held.representative().message_id().to_string(),
                    current.clone(),
                ),
            }),
            None if close_last_group => None,
            None => prior_pending,
        };
        self.cursors.insert(
            source.cursor_key(),
            LedgerCursor::new(last.0, last.1, last.2, new_byte_offset, pending),
        );
    }

    /// 1 行を workspace 全体・作業・当該 session の 3 つへ足し込む。
    fn fold_row(&mut self, row: &PricedRow, attribution: &FoldAttribution) {
        let stage = attribution.stage_slug();
        self.aggregate.fold(row, stage);
        self.workflows
            .update(attribution.workflow_key(), |workflow| {
                workflow.fold(row, stage, attribution.session_key());
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY: &str = "{\n  \"schemaVersion\": 3,\n  \"cursors\": {},\n  \"totals\": {\n    \"tokens\": {\n      \"input\": 0,\n      \"output\": 0,\n      \"cacheCreate5m\": 0,\n      \"cacheCreate1h\": 0,\n      \"cacheRead\": 0\n    },\n    \"usd\": 0\n  },\n  \"byStage\": {},\n  \"byModel\": {},\n  \"byAgent\": {},\n  \"workflows\": {}\n}";

    fn line(uuid: &str, message_id: &str, input: u64, output: u64) -> String {
        format!(
            r#"{{"uuid":"{uuid}","timestamp":"2026-09-10T00:00:00Z","type":"assistant","message":{{"id":"{message_id}","role":"assistant","model":"claude-opus-4-8","usage":{{"input_tokens":{input},"output_tokens":{output},"cache_read_input_tokens":0,"cache_creation_input_tokens":0,"cache_creation":{{"ephemeral_5m_input_tokens":0,"ephemeral_1h_input_tokens":0}}}}}}}}"#
        )
    }

    fn two_message_transcript() -> (tempfile::TempDir, String) {
        let directory = tempfile::tempdir().expect("一時ディレクトリ");
        let path = directory.path().join("simple.jsonl");
        std::fs::write(
            &path,
            format!(
                "{}\n{}\n",
                line("u1", "msg_1", 100, 20),
                line("u2", "msg_2", 200, 40)
            ),
        )
        .expect("テストのファイル");
        let text = path.to_string_lossy().into_owned();
        (directory, text)
    }

    fn attribution(stage: Option<&str>, session: &str, workflow: &str) -> FoldAttribution {
        FoldAttribution::new(stage.map(str::to_string), session.into(), workflow.into())
    }

    fn rendered(ledger: &UsageLedger) -> serde_json::Value {
        serde_json::from_str(&ledger.render()).expect("自分で書いた JSON")
    }

    fn number(value: &serde_json::Value, pointer: &str) -> f64 {
        value
            .pointer(pointer)
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(f64::NAN)
    }

    #[test]
    fn an_empty_ledger_renders_the_upstream_shape() {
        assert_eq!(UsageLedger::default().render(), EMPTY);
    }

    #[test]
    fn an_older_or_cursor_less_ledger_is_discarded_and_a_current_one_is_kept() {
        let directory = tempfile::tempdir().expect("一時ディレクトリ");
        let path = directory.path().join("usage-ledger.json");
        for stale in [
            "{ not json",
            "[]",
            r#"{"schemaVersion":2,"cursors":{},"totals":{"tokens":{"input":9}}}"#,
            r#"{"cursors":{}}"#,
            r#"{"schemaVersion":3,"cursors":{"/t.jsonl":{"lastUuid":"u","lastMessageId":"m"}},"totals":{"tokens":{"input":9}}}"#,
        ] {
            std::fs::write(&path, stale).expect("テストのファイル");
            assert_eq!(UsageLedger::load(&path).render(), EMPTY, "{stale}");
        }
        assert_eq!(
            UsageLedger::load(&directory.path().join("missing.json")).render(),
            EMPTY
        );
        std::fs::write(
            &path,
            r#"{"schemaVersion":3,"cursors":{"/t.jsonl":{"lastUuid":"u","lastTimestamp":"","lastMessageId":"m","byteOffset":5}},"totals":{"tokens":{"input":9,"output":0,"cacheCreate5m":0,"cacheCreate1h":0,"cacheRead":0},"usd":0.5},"byStage":{},"byModel":{},"byAgent":{},"workflows":{}}"#,
        )
        .expect("テストのファイル");
        let kept = rendered(&UsageLedger::load(&path));
        assert_eq!(number(&kept, "/cursors/~1t.jsonl/byteOffset"), 5.0);
        assert_eq!(number(&kept, "/totals/tokens/input"), 9.0);
        assert_eq!(number(&kept, "/totals/usd"), 0.5);
    }

    #[test]
    fn a_holdback_fold_keeps_the_last_group_pending_and_a_seal_closes_it() {
        let (_directory, transcript) = two_message_transcript();
        let rates = ModelRates::defaults();
        let current = attribution(None, "s", "w");
        let mut ledger = UsageLedger::default();
        ledger.fold_source(&FoldSource::main(&transcript, false), &rates, &current);
        let held = rendered(&ledger);
        let cursor = format!("/cursors/{}", transcript.replace('/', "~1"));
        assert_eq!(number(&held, &format!("{cursor}/byteOffset")), 325.0);
        assert_eq!(
            held.pointer(&format!("{cursor}/pending/messageId")),
            Some(&serde_json::Value::String("msg_2".into()))
        );
        assert_eq!(number(&held, "/totals/tokens/input"), 100.0);
        assert_eq!(number(&held, "/workflows/w/sessions/s/totals/usd"), 0.001);

        ledger.fold_source(&FoldSource::main(&transcript, true), &rates, &current);
        let sealed = rendered(&ledger);
        assert_eq!(number(&sealed, &format!("{cursor}/byteOffset")), 650.0);
        assert_eq!(sealed.pointer(&format!("{cursor}/pending")), None);
        assert_eq!(number(&sealed, "/totals/tokens/input"), 300.0);
        assert_eq!(number(&sealed, "/byModel/opus-4-8/usd"), 0.003);
        assert_eq!(number(&sealed, "/byAgent/main/tokens/output"), 60.0);

        ledger.fold_source(&FoldSource::main(&transcript, true), &rates, &current);
        assert_eq!(
            rendered(&ledger),
            sealed,
            "新しいバイトが無ければ何も変わらない"
        );
    }

    #[test]
    fn a_held_group_folds_into_the_attribution_captured_when_it_was_held() {
        let (_directory, transcript) = two_message_transcript();
        let rates = ModelRates::defaults();
        let mut ledger = UsageLedger::default();
        ledger.fold_source(
            &FoldSource::main(&transcript, false),
            &rates,
            &attribution(None, "s1", "w1"),
        );
        ledger.fold_source(
            &FoldSource::main(&transcript, true),
            &rates,
            &attribution(Some("st"), "s2", "w2"),
        );
        let value = rendered(&ledger);
        assert_eq!(number(&value, "/workflows/w1/totals/tokens/input"), 300.0);
        assert_eq!(
            number(&value, "/workflows/w1/byStage/st/totals/tokens/input"),
            200.0
        );
        assert_eq!(
            number(&value, "/workflows/w1/sessions/s1/totals/tokens/input"),
            300.0
        );
        assert_eq!(
            value.pointer("/workflows/w2"),
            None,
            "保留時の作業へ算入する"
        );
    }

    #[test]
    fn truncation_resets_the_cursor_and_drops_the_pending_group() {
        let (directory, transcript) = two_message_transcript();
        let rates = ModelRates::defaults();
        let current = attribution(None, "s", "w");
        let mut ledger = UsageLedger::default();
        ledger.fold_source(&FoldSource::main(&transcript, false), &rates, &current);
        // 保留位置（325）より短いファイルへ差し替える = 切り詰め。
        let short = r#"{"uuid":"r1","timestamp":"2026-09-10T00:00:00Z","type":"assistant","message":{"id":"msg_short","role":"assistant","model":"claude-opus-4-8","usage":{"input_tokens":500,"output_tokens":50}}}"#;
        assert!(short.len() < 325);
        std::fs::write(directory.path().join("simple.jsonl"), format!("{short}\n"))
            .expect("テストのファイル");
        ledger.fold_source(&FoldSource::main(&transcript, true), &rates, &current);
        let value = rendered(&ledger);
        let cursor = format!("/cursors/{}", transcript.replace('/', "~1"));
        assert_eq!(
            number(&value, &format!("{cursor}/byteOffset")),
            (short.len() + 1) as f64
        );
        assert_eq!(
            value.pointer(&format!("{cursor}/lastUuid")),
            Some(&serde_json::Value::String("r1".into()))
        );
        assert_eq!(value.pointer(&format!("{cursor}/pending")), None);
        assert_eq!(number(&value, "/totals/tokens/input"), 600.0);
    }

    #[test]
    fn a_written_ledger_reads_back_byte_for_byte() {
        let (directory, transcript) = two_message_transcript();
        let rates = ModelRates::defaults();
        let mut ledger = UsageLedger::default();
        ledger.fold_source(
            &FoldSource::main(&transcript, false),
            &rates,
            &attribution(Some("stage"), "s", "w"),
        );
        let path = directory
            .path()
            .join("aidlc/.aidlc-sessions/usage-ledger.json");
        ledger.write(&path).expect("書ける");
        let bytes = std::fs::read(&path).expect("書いたファイル");
        assert_eq!(String::from_utf8_lossy(&bytes), ledger.render());
        assert!(
            !bytes.ends_with(b"\n"),
            "JSON.stringify は末尾改行を付けない"
        );
        assert_eq!(UsageLedger::load(&path), ledger);
    }
}
