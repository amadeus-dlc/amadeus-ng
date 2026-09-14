//! heartbeat ファイル 1 本の写し。

use super::ObservedTimestamp;

/// `<hook>.last` の名前と中身。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeartbeatEntry {
    hook: String,
    timestamp: ObservedTimestamp,
}

impl HeartbeatEntry {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(hook: String, timestamp: ObservedTimestamp) -> Self {
        Self { hook, timestamp }
    }

    /// フック名 (`.last` を除いたファイル名)。
    #[must_use]
    pub fn hook(&self) -> &str {
        &self.hook
    }

    /// 最終発火の時刻。
    #[must_use]
    pub const fn timestamp(&self) -> &ObservedTimestamp {
        &self.timestamp
    }
}
