//! 人間の返答の代役 — 監査シャードへ `HUMAN_TURN` を 1 行追記する。
//!
//! 実運用では UserPromptSubmit のフック（`aidlc engine hook record-human-turn`）がこの行を
//! 書く。2.8.2 の承認・差し戻しは、直近の解決より後にこの行があることを要求する
//! （`aidlc-state.ts` `approvalPreconditions` / `handleReject`）。試験では、承認の前に
//! 人間が返答したことをこの 1 行で表す。
#![allow(dead_code, clippy::expect_used)]
use std::fs;
use std::path::Path;

/// `record` の監査シャード（最初の 1 本）へ、現在時刻の `HUMAN_TURN` を追記する。
pub(crate) fn append_human_reply(record: &Path) {
    let audit = record.join("audit");
    let shard = fs::read_dir(&audit)
        .expect("監査ディレクトリは在る")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .min()
        .expect("シャードは 1 つ以上ある");
    let mut content = fs::read_to_string(&shard).expect("シャードは読める");
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    content.push_str(&format!(
        "\n## Human Turn\n**Timestamp**: {now}\n**Event**: HUMAN_TURN\n\n---\n"
    ));
    fs::write(&shard, content).expect("シャードは書ける");
}
