//! 監査シャードの読取 (本家 `readAuditShardEvents` の写し — ブロックを `\n---\n` で割る)。

use std::path::Path;

/// 監査ブロック 1 件の、診断が見る欄。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AuditEvent {
    event: String,
    timestamp: String,
    stage: Option<String>,
}

impl AuditEvent {
    pub(super) fn event(&self) -> &str {
        &self.event
    }

    pub(super) fn timestamp(&self) -> &str {
        &self.timestamp
    }

    /// `Stage` 欄、無ければ `Slug` 欄。
    pub(super) fn stage(&self) -> Option<&str> {
        self.stage.as_deref()
    }
}

/// 読めた全シャードの連結本文と、そこから拾った (Event, Timestamp) 付きブロック。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AuditLedger {
    shard_count: usize,
    content: String,
    events: Vec<AuditEvent>,
}

impl AuditLedger {
    /// `<root>/audit/*.md` を名前順に読む (無い・読めないシャードは飛ばす)。
    pub(super) fn read(docs_root: &Path) -> Self {
        let mut shards: Vec<_> = std::fs::read_dir(docs_root.join("audit"))
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
            .collect();
        shards.sort();
        let mut parts = Vec::new();
        let mut events = Vec::new();
        for shard in &shards {
            let Ok(content) = std::fs::read_to_string(shard) else {
                continue;
            };
            let normalized = content.replace("\r\n", "\n");
            for block in normalized.split("\n---\n") {
                let (Some(event), Some(timestamp)) =
                    (field(block, "Event"), field(block, "Timestamp"))
                else {
                    continue;
                };
                events.push(AuditEvent {
                    event,
                    timestamp,
                    stage: field(block, "Stage").or_else(|| field(block, "Slug")),
                });
            }
            parts.push(content);
        }
        Self {
            shard_count: shards.len(),
            content: parts.join("\n"),
            events,
        }
    }

    pub(super) const fn shard_count(&self) -> usize {
        self.shard_count
    }

    pub(super) fn content(&self) -> &str {
        &self.content
    }

    pub(super) fn events(&self) -> &[AuditEvent] {
        &self.events
    }
}

/// `**Field**: value` の行 (先頭の `- ` は剥がす — 本家 `auditBlockField`)。
fn field(block: &str, name: &str) -> Option<String> {
    let prefix = format!("**{name}**:");
    block.lines().find_map(|raw| {
        let line = raw.strip_prefix("- ").unwrap_or(raw);
        line.strip_prefix(&prefix)
            .map(|value| value.trim().to_string())
    })
}
