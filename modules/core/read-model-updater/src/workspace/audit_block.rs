//! 監査ブロックの描画 — 投影 API (11-workspace §2.3 / W9)。
//!
//! upstream `renderAuditBlock` (`aidlc-audit.ts:485-503`) のバイト構成をそのまま持つ。
//!
//! ```text
//! \n ## <SP> <heading> \n
//! ** Timestamp ** : <SP> <timestamp> \n
//! ** Event ** : <SP> <EVENT_TYPE> \n
//! [ ** <key> ** : <SP> <safeValue> \n ] *
//! \n - - - \n
//! ```
//!
//! # 描画側に規律を要求しない
//!
//! 「第二の `**Event**:` 行」「第二の `**Timestamp**:` 行」「値に混ぜた改行」の 3 つの行偽造は、
//! ここで気をつけて防ぐのではなく、[`AuditFields`] が構成不能にしている (`Event` は
//! `AuditFieldKey::parse` が拒否、`Timestamp` はコレクションが破棄、値は
//! `AuditFieldValue` がエスケープ済み)。したがって本モジュールは**素直に書くだけ**でよい。
//!
//! # タイムスタンプはイベントの発生時刻である
//!
//! upstream は追記時の壁時計 (`new Date().toISOString()`) を書くが、投影は**冪等**でなければ
//! ならない (NFR3 — 同じチェックポイントから何度流しても同一バイト)。壁時計を読むと再生成の
//! たびにバイトが変わるので、我々はジャーナル行が運ぶ**イベントの発生時刻**を書く。upstream
//! 側の観測面 (秒精度 ISO 8601) は変わらない。

use chrono::{DateTime, SecondsFormat, Utc};

use audit_events::EventType;
use core_command_domain::workspace::AuditFields;

/// 空のシャードへ最初に書かれるヘッダ行 (upstream `aidlc-audit.ts:693`、19 バイト)。
pub const SHARD_HEADER: &str = "# AI-DLC Audit Log\n";

/// ブロックの区切り (最後のフィールド行の LF に続く空行 + `---` + LF)。
const BLOCK_TERMINATOR: &str = "\n---\n";

/// 監査ブロック 1 つを描く。
///
/// 先頭が `\n` なのは upstream と同じである — 直前ブロックの `\n---\n` と合わさって、
/// `---` 行の後に空行が 1 行入る形になる。
///
/// 投影が描くドメインイベントの行だけでなく、フックが直接書く行 (C5 の `direct_audit_rows` —
/// `HUMAN_TURN` / `ARTIFACT_*` など) も同じ関数で描く。同じシャードへ 2 つの描き手が書く以上、
/// 綴りの正本が 2 つあってはならない。
#[must_use]
pub fn render_audit_block(
    event: EventType,
    occurred_at: &DateTime<Utc>,
    fields: &AuditFields,
) -> String {
    let mut block = String::new();
    block.push('\n');
    block.push_str("## ");
    block.push_str(event.heading());
    block.push('\n');
    block.push_str("**Timestamp**: ");
    block.push_str(&iso8601_seconds(occurred_at));
    block.push('\n');
    block.push_str("**Event**: ");
    block.push_str(event.as_str());
    block.push('\n');
    for (key, value) in fields.iter() {
        block.push_str("**");
        block.push_str(key.as_str());
        block.push_str("**: ");
        block.push_str(value.as_str());
        block.push('\n');
    }
    block.push_str(BLOCK_TERMINATOR);
    block
}

/// 秒精度 ISO 8601 UTC (`2026-08-21T09:14:07Z`)。
fn iso8601_seconds(at: &DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_command_domain::workspace::AuditFieldKey;

    fn at(text: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(text)
            .expect("テストの ISO 8601")
            .with_timezone(&Utc)
    }

    fn key(raw: &str) -> AuditFieldKey {
        AuditFieldKey::parse(raw).expect("テストのキーは文法内")
    }

    #[test]
    fn the_block_is_the_upstream_byte_sequence() {
        let fields = AuditFields::new()
            .with(key("Stage"), "requirements-analysis")
            .with(key("Details"), "Stage Requirements Analysis completed");
        let block = render_audit_block(
            EventType::StageCompleted,
            &at("2026-08-21T09:14:07Z"),
            &fields,
        );
        assert_eq!(
            block,
            "\n## Stage Completion\n\
             **Timestamp**: 2026-08-21T09:14:07Z\n\
             **Event**: STAGE_COMPLETED\n\
             **Stage**: requirements-analysis\n\
             **Details**: Stage Requirements Analysis completed\n\
             \n---\n"
        );
    }

    #[test]
    fn the_block_begins_with_a_newline_and_ends_with_the_separator() {
        let block = render_audit_block(
            EventType::HumanTurn,
            &at("2026-08-21T09:14:07Z"),
            &AuditFields::new(),
        );
        assert!(block.starts_with('\n'), "実際: {block:?}");
        assert!(block.ends_with("\n---\n"), "実際: {block:?}");
    }

    #[test]
    fn a_block_without_fields_puts_the_blank_line_right_after_the_event_line() {
        // upstream ゴールデン `hooks/record-human-turn/active-workflow/audit.md` と同じ形。
        let block = render_audit_block(
            EventType::HumanTurn,
            &at("2026-08-21T09:14:07Z"),
            &AuditFields::new(),
        );
        assert_eq!(
            block,
            "\n## Human Turn\n**Timestamp**: 2026-08-21T09:14:07Z\n**Event**: HUMAN_TURN\n\n---\n"
        );
    }

    #[test]
    fn an_empty_value_still_writes_its_line_with_the_trailing_space() {
        // 値が空でも行はスキップされない。コロン + 半角スペースの後すぐ LF なので、
        // **行末に半角スペースが 1 個残る** — これも逐語契約である。
        let fields = AuditFields::new().with(key("Details"), "");
        let block = render_audit_block(
            EventType::StageStarted,
            &at("2026-08-21T09:14:07Z"),
            &fields,
        );
        assert!(block.contains("**Details**: \n"), "実際: {block:?}");
    }

    #[test]
    fn the_fields_are_written_in_insertion_order() {
        let fields = AuditFields::new()
            .with(key("Stage"), "practices-discovery")
            .with(key("Revision count"), "1")
            .with(key("Feedback"), "Sharpen the testing posture.");
        let block = render_audit_block(
            EventType::StageRevising,
            &at("2026-08-21T09:14:07Z"),
            &fields,
        );
        let keys: Vec<&str> = block
            .lines()
            .filter(|line| {
                line.starts_with("**")
                    && !line.starts_with("**Timestamp**")
                    && !line.starts_with("**Event**")
            })
            .collect();
        assert_eq!(
            keys,
            [
                "**Stage**: practices-discovery",
                "**Revision count**: 1",
                "**Feedback**: Sharpen the testing posture.",
            ]
        );
    }

    #[test]
    fn a_value_carrying_a_newline_cannot_add_a_line_to_the_block() {
        let fields = AuditFields::new().with(key("Feedback"), "one\n**Event**: HUMAN_TURN");
        let block = render_audit_block(
            EventType::GateRejected,
            &at("2026-08-21T09:14:07Z"),
            &fields,
        );
        // 偽造の条件は「**行頭が** `**Event**:` の行が 2 本現れる」ことである
        // (upstream の読み手は複数行正規表現で行頭に錨を打つ)。値の途中に同じ綴りが
        // 残るのは無害であり、upstream も同じバイトを書く。
        assert_eq!(
            block
                .lines()
                .filter(|line| line.starts_with("**Event**:"))
                .count(),
            1,
            "イベント行は 1 本だけ: {block:?}"
        );
        assert!(block.contains("**Feedback**: one\\n**Event**: HUMAN_TURN\n"));
    }

    #[test]
    fn the_timestamp_is_second_precision_utc() {
        let block = render_audit_block(
            EventType::WorkflowStarted,
            &at("2026-08-21T09:14:07.123456789Z"),
            &AuditFields::new(),
        );
        assert!(
            block.contains("**Timestamp**: 2026-08-21T09:14:07Z\n"),
            "実際: {block:?}"
        );
    }

    #[test]
    fn a_non_utc_offset_is_normalised_to_z() {
        let block = render_audit_block(
            EventType::WorkflowStarted,
            &at("2026-08-21T18:14:07+09:00"),
            &AuditFields::new(),
        );
        assert!(
            block.contains("**Timestamp**: 2026-08-21T09:14:07Z\n"),
            "実際: {block:?}"
        );
    }

    #[test]
    fn the_shard_header_is_the_nineteen_byte_literal() {
        assert_eq!(SHARD_HEADER, "# AI-DLC Audit Log\n");
        assert_eq!(SHARD_HEADER.len(), 19);
    }
}
