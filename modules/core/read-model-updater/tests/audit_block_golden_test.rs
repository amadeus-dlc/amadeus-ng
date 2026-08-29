//! 監査ブロック描画の**ゴールデン逐語一致** (FR1.1 / W9)。
//!
//! `tests/golden/upstream-3c3146cf/**/audit.md` は upstream の実バイトである（U1 採取）。
//! 各ファイルをブロックへ切り、フィールドを読み取ってから [`render_audit_block`] で描き直し、
//! **元のバイトと 1 バイトも違わない**ことを検査する。
//!
//! 描き直しが通るということは、見出し 86 語・フィールド順・行終端・区切りのすべてが upstream と
//! 一致しているということである。ゴールデンの `<TS>` は採取時の正規化なので、こちらの出力にも
//! 同じ正規化（`normalization.json` の ISO 8601 規則）を当ててから比べる。
//!
//! ゴールデンはコマンド 1 回が**追記した分**（デルタ）である。したがって空のシャードを新規に
//! 作ったケース（`cli/intent-create`）だけがヘッダ行 `# AI-DLC Audit Log\n` を先頭に持ち、
//! 既存シャードへ追記したケースは持たない。この非対称もここで固定する。

// テストコードでは unwrap / expect / panic を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use core_command_domain::workspace::EventType;
use core_command_domain::workspace::{AuditFieldKey, AuditFields};
use core_read_model_updater::workspace::{SHARD_HEADER, render_audit_block};

/// ゴールデンが正規化で潰した実行時値の置き換え先。
const TS_PLACEHOLDER: &str = "<TS>";

/// 描き直しに使う任意の発生時刻（正規化で `<TS>` に潰れるので値そのものは観測されない）。
const RENDER_AT: &str = "2026-08-22T13:43:00Z";

/// 監査ブロックの区切り（ここでファイルを切る）。
const BLOCK_TERMINATOR: &str = "\n---\n";

fn golden_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../tests/golden/upstream-3c3146cf")
}

/// `**<key>**: <value>` 形のフィールド行を 1 本読む。
fn field_line(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("**")?;
    let (key, value) = rest.split_once("**: ")?;
    Some((key, value))
}

/// 読み取ったブロック 1 つ。
struct Block {
    event: EventType,
    fields: AuditFields,
}

/// upstream が書いたブロックを読み取る（描き直しの材料を取り出す）。
fn parse_block(block: &str, source: &Path) -> Block {
    let body = block
        .strip_prefix('\n')
        .unwrap_or_else(|| panic!("{}: ブロックは改行で始まる: {block:?}", source.display()));
    let mut lines = body.lines();

    let heading = lines
        .next()
        .and_then(|line| line.strip_prefix("## "))
        .unwrap_or_else(|| panic!("{}: 見出し行がない", source.display()))
        .to_string();
    let timestamp = lines
        .next()
        .and_then(|line| line.strip_prefix("**Timestamp**: "))
        .unwrap_or_else(|| panic!("{}: Timestamp 行がない", source.display()));
    assert_eq!(
        timestamp,
        TS_PLACEHOLDER,
        "{}: 採取時に正規化されているはず",
        source.display()
    );
    let event_name = lines
        .next()
        .and_then(|line| line.strip_prefix("**Event**: "))
        .unwrap_or_else(|| panic!("{}: Event 行がない", source.display()));
    let event = EventType::parse(event_name)
        .unwrap_or_else(|| panic!("{}: 閉集合外のイベント {event_name}", source.display()));
    assert_eq!(
        event.heading(),
        heading,
        "{}: 見出しが upstream と食い違う",
        source.display()
    );

    let mut fields = AuditFields::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        let (key, value) = field_line(line).unwrap_or_else(|| {
            panic!("{}: フィールド行として読めない: {line:?}", source.display())
        });
        let key = AuditFieldKey::parse(key).unwrap_or_else(|error| {
            panic!("{}: フィールドキーが文法外: {error}", source.display())
        });
        fields = fields.with(key, value);
    }
    Block { event, fields }
}

/// ゴールデンと同じ正規化（ISO 8601 UTC → `<TS>`）を実測値へ当てる。
fn normalise(rendered: &str, at: &DateTime<Utc>) -> String {
    let stamp = at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    rendered.replace(&stamp, TS_PLACEHOLDER)
}

/// `**/audit.md` を全部集める。
fn audit_goldens(dir: &Path, found: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).expect("ゴールデンのディレクトリがある");
    for entry in entries {
        let path = entry.expect("ディレクトリ項目").path();
        if path.is_dir() {
            audit_goldens(&path, found);
        } else if path.file_name().is_some_and(|name| name == "audit.md") {
            found.push(path);
        }
    }
}

#[test]
fn every_upstream_audit_block_is_reproduced_byte_for_byte() {
    let at = DateTime::parse_from_rfc3339(RENDER_AT)
        .expect("固定の ISO 8601")
        .with_timezone(&Utc);

    let mut goldens = Vec::new();
    audit_goldens(&golden_root(), &mut goldens);
    goldens.sort();
    assert!(!goldens.is_empty(), "ゴールデンが 1 件も見つからない");

    let mut blocks_checked = 0_usize;
    let mut headers_seen = 0_usize;
    let mut events_seen = std::collections::BTreeSet::new();
    for path in &goldens {
        let content = std::fs::read_to_string(path).expect("ゴールデンは読める");
        if content.is_empty() {
            // 行を 1 本も書かなかったケース（フックが無視した等）。描くものが無い。
            continue;
        }
        assert!(
            content.ends_with(BLOCK_TERMINATOR),
            "{}: 末尾が区切りでない",
            path.display()
        );
        // 空のシャードへの初回書込だけがヘッダ行を持つ (upstream `aidlc-audit.ts:693`)。
        let blocks = match content.strip_prefix(SHARD_HEADER) {
            Some(rest) => {
                headers_seen += 1;
                rest
            }
            None => content.as_str(),
        };
        for block in blocks.split_inclusive(BLOCK_TERMINATOR) {
            let parsed = parse_block(block, path);
            let rendered = render_audit_block(parsed.event, &at, &parsed.fields);
            assert_eq!(
                normalise(&rendered, &at),
                block,
                "{}: 描き直したバイトが upstream と違う",
                path.display()
            );
            events_seen.insert(parsed.event.as_str());
            blocks_checked += 1;
        }
    }

    // 検査した中身そのものを固定しておく — ゴールデンが減ったのに緑のまま、を防ぐ。
    assert_eq!(blocks_checked, 70, "検査したブロック数");
    assert_eq!(
        headers_seen, 1,
        "ヘッダ行を持つのは空シャードを作った 1 ケースだけ"
    );
    assert_eq!(
        events_seen.into_iter().collect::<Vec<_>>(),
        [
            "ARTIFACT_CREATED",
            "ARTIFACT_UPDATED",
            "ERROR_LOGGED",
            "GATE_APPROVED",
            "GATE_REJECTED",
            "HUMAN_TURN",
            "PHASE_COMPLETED",
            "PHASE_SKIPPED",
            "PHASE_STARTED",
            "PHASE_VERIFIED",
            "PRACTICES_AFFIRMED",
            "RECOMPOSED",
            "STAGE_AWAITING_APPROVAL",
            "STAGE_COMPLETED",
            "STAGE_JUMPED",
            "STAGE_REVISING",
            "STAGE_SKIPPED",
            "STAGE_STARTED",
            "WORKFLOW_PARKED",
            "WORKFLOW_STARTED",
            "WORKFLOW_UNPARKED",
            "WORKSPACE_INITIALISED",
            "WORKSPACE_SCAFFOLDED",
            "WORKSPACE_SCANNED",
        ],
        "ゴールデンが実際に固定しているイベント型 (86 語中 24 語)"
    );
}
