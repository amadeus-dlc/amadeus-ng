//! heartbeat ファイル 1 本の写し。

use super::TimestampView;

/// `<hook>.last` の名前と中身。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeartbeatEntryView {
    hook: String,
    timestamp: TimestampView,
}

impl HeartbeatEntryView {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(hook: String, timestamp: TimestampView) -> Self {
        Self { hook, timestamp }
    }

    /// フック名 (`.last` を除いたファイル名)。
    #[must_use]
    pub fn hook(&self) -> &str {
        &self.hook
    }

    /// 最終発火の時刻。
    #[must_use]
    pub const fn timestamp(&self) -> &TimestampView {
        &self.timestamp
    }
}
