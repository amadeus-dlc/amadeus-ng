//! `AuditEventRecord` — 台帳から読み取ったイベント 1 件 (位置つき)。

use core::fmt;

use chrono::{DateTime, NaiveDateTime, Utc};

use super::audit_events::EventType;

/// タイムスタンプ行の接頭辞。
const TIMESTAMP_PREFIX: &str = "**Timestamp**: ";
/// イベント行の接頭辞。
const EVENT_PREFIX: &str = "**Event**: ";
/// タイムスタンプ行の書式（秒精度 ISO 8601 UTC — 描き手は
/// `to_rfc3339_opts(SecondsFormat::Secs, true)` で書く）。
const TIMESTAMP_FORMAT: &str = "%Y-%m-%dT%H:%M:%SZ";

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

    /// タイムスタンプを**時刻として**読む（秒精度 ISO でなければ `None`）。
    ///
    /// 逐語の文字列を時刻に解釈するのは行の持ち主の仕事であり、読み手ごとに書式を
    /// 書き写さない（`coding-rules/domain-services.md` — 導出はまず所有する型へ）。
    /// 秒精度以外の綴り（小数秒・オフセット付き）は書式が違うので読まない — 台帳の
    /// 描き手が書く形はただ 1 つである。
    #[must_use]
    pub fn instant(&self) -> Option<DateTime<Utc>> {
        NaiveDateTime::parse_from_str(&self.timestamp, TIMESTAMP_FORMAT)
            .ok()
            .map(|naive| naive.and_utc())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn record(timestamp: &str) -> AuditEventRecord {
        record_of(
            &format!("\n**Timestamp**: {timestamp}\n**Event**: HUMAN_TURN\n"),
            0,
        )
        .expect("材料は揃っている")
    }

    /// 秒精度 ISO だけが時刻として読める。
    #[test]
    fn only_the_second_precision_utc_spelling_reads_as_an_instant() {
        assert_eq!(
            record("2026-08-21T09:00:00Z").instant(),
            Some(
                DateTime::parse_from_rfc3339("2026-08-21T09:00:00Z")
                    .expect("固定の ISO")
                    .with_timezone(&Utc)
            )
        );
        assert_eq!(record("2026-08-21T09:00:00.500Z").instant(), None);
        assert_eq!(record("2026-08-21T09:00:00+09:00").instant(), None);
        assert_eq!(record("never").instant(), None);
    }
}
