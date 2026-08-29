//! 監査シャード**横断**の位置付き読取（FR1.1）— 列挙・連結（投影側の I/O）と順序規則
//! （ドメインの純関数）の合流点。
//!
//! 他クローンのシャードは読み取り専用の外部入力である（C5 rules）。読み手は
//! 「ファイル名順に連結 → timestamp 昇順 → 同値はバッファ位置」で 1 つの並びを得る。素朴に
//! 「連結バッファの末尾が最新」と読むと、辞書順で後ろのシャードから**より古い**イベントを
//! 拾ってしまう — その誤りが起きないことをここで固定する。

// テストコードでは unwrap / expect / panic を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use audit_events::EventType;
use chrono::{DateTime, Utc};
use core_command_domain::workspace::{
    AuditEventRecord, AuditFieldKey, AuditFields, find_all_events,
};
use core_query_read_model_updater::workspace::{
    append_audit_shard, read_all_audit_shards, render_audit_block,
};
use tempfile::TempDir;

fn at(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text)
        .expect("テストの ISO 8601")
        .with_timezone(&Utc)
}

fn block(event: EventType, timestamp: &str) -> String {
    let fields = AuditFields::new().with(
        AuditFieldKey::parse("Stage").expect("文法内"),
        "practices-discovery",
    );
    render_audit_block(event, &at(timestamp), &fields)
}

/// 2 つのクローンのシャードを持つ記録ディレクトリ。
struct Ledger {
    _dir: TempDir,
    audit: std::path::PathBuf,
}

impl Ledger {
    fn new() -> Ledger {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let audit = dir.path().join("audit");
        Ledger { _dir: dir, audit }
    }

    fn shard(&self, name: &str) -> std::path::PathBuf {
        self.audit.join(name)
    }

    fn write(&self, name: &str, blocks: &str) {
        append_audit_shard(&self.shard(name), blocks).expect("追記");
    }

    fn events(&self) -> Vec<(String, &'static str)> {
        let buffer = read_all_audit_shards(&self.audit);
        find_all_events(&buffer)
            .iter()
            .map(|record| (record.timestamp().to_string(), record.event().as_str()))
            .collect()
    }
}

#[test]
fn a_lexically_later_shard_holding_an_older_event_does_not_become_the_latest() {
    let ledger = Ledger::new();
    // 辞書順で先のシャードに**新しい**イベント、後のシャードに**古い**イベントを置く。
    ledger.write(
        "aaa-00000001.md",
        &block(EventType::StageCompleted, "2026-08-21T09:00:09Z"),
    );
    ledger.write(
        "zzz-00000002.md",
        &block(EventType::HumanTurn, "2026-08-21T09:00:01Z"),
    );

    assert_eq!(
        ledger.events(),
        [
            ("2026-08-21T09:00:01Z".to_string(), "HUMAN_TURN"),
            ("2026-08-21T09:00:09Z".to_string(), "STAGE_COMPLETED"),
        ],
        "時刻順であってファイル名順ではない"
    );
}

#[test]
fn the_same_second_across_shards_is_broken_by_file_name_then_append_order() {
    let ledger = Ledger::new();
    ledger.write(
        "bbb-00000002.md",
        &format!(
            "{}{}",
            block(EventType::GateApproved, "2026-08-21T09:00:00Z"),
            block(EventType::StageCompleted, "2026-08-21T09:00:00Z")
        ),
    );
    ledger.write(
        "aaa-00000001.md",
        &block(EventType::HumanTurn, "2026-08-21T09:00:00Z"),
    );

    assert_eq!(
        ledger
            .events()
            .into_iter()
            .map(|(_, event)| event)
            .collect::<Vec<_>>(),
        ["HUMAN_TURN", "GATE_APPROVED", "STAGE_COMPLETED"],
        "aaa が先、bbb 内は追記順"
    );
}

#[test]
fn the_shard_header_does_not_become_an_event() {
    // 空シャードへの初回書込はヘッダ行を先に置く。読み手がそれをイベントと取り違えない。
    let ledger = Ledger::new();
    ledger.write(
        "aaa-00000001.md",
        &block(EventType::HumanTurn, "2026-08-21T09:00:00Z"),
    );
    assert!(
        std::fs::read_to_string(ledger.shard("aaa-00000001.md"))
            .expect("読める")
            .starts_with("# AI-DLC Audit Log\n")
    );
    assert_eq!(ledger.events().len(), 1);
}

#[test]
fn an_empty_ledger_directory_reads_as_no_events() {
    let ledger = Ledger::new();
    assert_eq!(read_all_audit_shards(&ledger.audit), "");
    assert!(find_all_events("").is_empty());
}

#[test]
fn the_latest_is_taken_from_the_ordering_not_from_the_buffer_tail() {
    let ledger = Ledger::new();
    ledger.write(
        "aaa-00000001.md",
        &block(EventType::StageCompleted, "2026-08-21T09:00:09Z"),
    );
    ledger.write(
        "zzz-00000002.md",
        &block(EventType::HumanTurn, "2026-08-21T09:00:01Z"),
    );
    let buffer = read_all_audit_shards(&ledger.audit);
    let ordered = find_all_events(&buffer);
    assert_eq!(
        ordered.latest().map(AuditEventRecord::event),
        Some(EventType::StageCompleted)
    );
}
