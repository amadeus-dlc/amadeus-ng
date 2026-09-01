//! `AuditEventRecord` — 台帳から読み取ったイベント 1 件 (位置つき)。

use core::fmt;

use super::audit_events::EventType;

/// タイムスタンプ行の接頭辞。
const TIMESTAMP_PREFIX: &str = "**Timestamp**: ";
/// イベント行の接頭辞。
const EVENT_PREFIX: &str = "**Event**: ";

/// 台帳から読み取ったイベント 1 件（位置つき）。
///
/// フィールドは private。順序の材料（タイムスタンプと位置）を外から書き換えられると、
/// 並びの意味が壊れるためである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEventRecord {
    timestamp: String,
    event: EventType,
    position: usize,
}

impl AuditEventRecord {
    /// 行が名乗っていた秒精度 ISO タイムスタンプ（逐語 — 解釈しない）。
    #[must_use]
    pub fn timestamp(&self) -> &str {
        &self.timestamp
    }

    /// イベント型（閉集合の 86 語）。
    #[must_use]
    pub const fn event(&self) -> EventType {
        self.event
    }

    /// 連結バッファ内でのブロックの位置（0 始まり）。タイを破る材料である。
    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }
}

impl fmt::Display for AuditEventRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} @{}",
            self.timestamp,
            self.event.as_str(),
            self.position
        )
    }
}

/// ブロック 1 つから順序の材料を取り出す（欠落・閉集合外は `None`）。
///
/// 本型の構造体リテラルを書く唯一の場所なので、順序付けの側ではなくここに置く
/// （`OrderedAuditEvents::find_in` がこれを呼ぶ — coding-rules/domain-services.md）。
pub(super) fn record_of(block: &str, position: usize) -> Option<AuditEventRecord> {
    let mut timestamp = None;
    let mut event = None;
    for line in block.lines() {
        if timestamp.is_none()
            && let Some(value) = line.strip_prefix(TIMESTAMP_PREFIX)
        {
            timestamp = Some(value.to_string());
        }
        if event.is_none()
            && let Some(value) = line.strip_prefix(EVENT_PREFIX)
        {
            event = EventType::parse(value);
        }
    }
    Some(AuditEventRecord {
        timestamp: timestamp?,
        event: event?,
        position,
    })
}
